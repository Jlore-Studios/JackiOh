#!/bin/sh
# Mutation testing (#3490): whether a test fails when the code it covers is wrong, which coverage
# (super.yml's llvm-cov) cannot say. cargo-mutants over the Rust crates (.cargo/mutants.toml: its
# settings and every mutant left out, with the reason) and Stryker over the web's layers and the
# Worker (apps/web/stryker.config.mjs, apps/edge/stryker.config.mjs). mutants.yml runs a shard of
# every area once a week and writes each one's score to its summary.
#
#   sh scripts/mutants.sh <area> [k] [-- <args>]   one shard of an area: shard k mod n, n the area's
#                                                  shard count; without k, every shard (the whole area)
#   sh scripts/mutants.sh --in-diff [base]         the mutants on the lines changed since the branch
#                                                  left base (origin/main), working tree included: a
#                                                  builder's check of their own tests before pushing;
#                                                  it fails when one survives
#   sh scripts/mutants.sh areas                    every area: its mutants, those left out, its shards
#   sh scripts/mutants.sh score <area>             the finished run's row of the summary table, and
#                                                  its survivors
#
# The areas: engine, ai, run, shared, server, tools and wasm (each crate, the three WebAssembly ones
# as one); the cards by set, cards-<set> for each of crates/cards/src/scripts/<set>/ and cards-lib for
# the rest of the crate, and `cards` itself, whose k picks the set (k mod the sets) and then that
# set's shard; web-<layer> for each of Game.tsx's layers (animations, effects, audio, cards, drag,
# settings) and the client's two adapters (wire, wasm); edge for the Worker. Shard k of an area holds
# its mutants k, k+n, k+2n… (cargo-mutants' sharding; the web's by file the same way), so any one
# shard samples the whole area and the weeks walk every shard in turn.
#
# A run writes target/mutants/<area>/: cargo-mutants' mutants.out/ (caught.txt, missed.txt…) or
# Stryker's mutation.json and mutation.html. The Rust areas need cargo-mutants
# (`cargo install cargo-mutants --locked`); the web's need the workspace's install (Stryker is among
# its devDependencies) and the engine's WebAssembly module (scripts/build-wasm.sh). Stryker mutates
# the files where they are, since the tests read files by their path from the repository's root,
# and puts each back when it ends: run it on a clean tree. Anything after `--` goes to cargo-mutants
# or Stryker.
set -euf # -f: no pathname expansion, so the globs below reach cargo-mutants as written

cd "$(dirname "$0")/.."
ROOT=$(pwd)

# Mutants per shard, sized so one shard fits mutants.yml's five-hour step on a runner's four cores,
# two mutants at a time. A mutant's time is mostly compiling its package and its tests again: about a
# minute in run, shared and the cards, three in the engine (the cards are its tests' dev-dependency)
# and the AI (whose tests play games), four in the server and the tools.
SIZE_ENGINE=150
SIZE_CARDS=250
SIZE_AI=120
SIZE_RUN=400
SIZE_SHARED=250
SIZE_SERVER=80
SIZE_TOOLS=120
SIZE_WASM=200
# Source files per Stryker shard: Stryker runs each mutant against only the tests that cover it.
SIZE_WEB_FILES=40
# cargo-mutants' --jobs: mutants tested at once, each in its own copy of the tree.
JOBS=${MUTANTS_JOBS:-2}
# Survivors listed under a summary's row, so a summary stays readable; the report has them all.
SURVIVORS_SHOWN=200

die() {
  echo "mutants: $*" >&2
  exit 1
}

# The card sets, one area each, and the rest of the crate.
card_areas() {
  for dir in $(ls crates/cards/src/scripts); do
    [ -d "crates/cards/src/scripts/$dir" ] && echo "cards-$dir"
  done
  echo cards-lib
}

# FILTER (cargo-mutants' arguments that pick the area's mutants) and SIZE for a Rust area.
rust_area() {
  case "$1" in
    engine) FILTER="-p jackioh-engine" SIZE=$SIZE_ENGINE ;;
    ai) FILTER="-p jackioh-ai" SIZE=$SIZE_AI ;;
    run) FILTER="-p jackioh-run" SIZE=$SIZE_RUN ;;
    shared) FILTER="-p jackioh-shared" SIZE=$SIZE_SHARED ;;
    server) FILTER="-p jackioh-server" SIZE=$SIZE_SERVER ;;
    tools) FILTER="-p jackioh-tools" SIZE=$SIZE_TOOLS ;;
    wasm) FILTER="-p jackioh-wasm -p jackioh-wasm-core -p jackioh-wasm-c" SIZE=$SIZE_WASM ;;
    cards-lib) FILTER="-p jackioh-cards -e crates/cards/src/scripts/**" SIZE=$SIZE_CARDS ;;
    cards-?*)
      [ -d "crates/cards/src/scripts/${1#cards-}" ] || return 1
      FILTER="-p jackioh-cards -f crates/cards/src/scripts/${1#cards-}/**" SIZE=$SIZE_CARDS
      ;;
    *) return 1 ;;
  esac
  SIZE=${MUTANTS_SHARD_SIZE:-$SIZE}
}

# The web layer's source files, one per line, from the repository's root.
web_files() {
  case "$1" in
    animations) paths=apps/web/src/game/animations.ts ;;
    effects) paths=apps/web/src/fx ;;
    audio) paths=apps/web/src/audio ;;
    cards) paths=apps/web/src/cards ;;
    drag) paths=apps/web/src/game/drag ;;
    settings) paths=apps/web/src/settings ;;
    # Not the generated: ts-rs's types (generated/) and the engine's and the server's constants.
    wire) paths=apps/web/src/wire ;;
    wasm) paths=apps/web/src/wasm ;;
    *) return 1 ;;
  esac
  git ls-files -- $paths | grep -E '\.tsx?$' |
    grep -vE '\.test\.tsx?$|\.d\.ts$|/test/|/generated/|/pkg/|/wire/(engineConfig|serverConfig)\.ts$' || true
}

# The Worker's source files.
edge_files() {
  git ls-files -- apps/edge/src | grep -E '\.ts$' | grep -vE '\.test\.ts$' || true
}

# Shards in an area of `count` items at `size` each: at least one.
shards() {
  n=$((($1 + $2 - 1) / $2))
  [ "$n" -ge 1 ] || n=1
  echo "$n"
}

count_lines() {
  wc -l | tr -d ' '
}

# cargo-mutants exits 2 when a mutant survived and 3 when one timed out: a report, not a failure.
run_rust() {
  area=$1 k=$2
  shift 2
  out=target/mutants/$area
  total=$(cargo mutants --list $FILTER | count_lines)
  all=$(cargo mutants --list --no-config $FILTER | count_lines)
  n=$(shards "$total" "$SIZE")
  shard=
  label="1/1"
  if [ -n "$k" ] && [ "$n" -gt 1 ]; then
    shard="--shard $((k % n))/$n"
    label="$((k % n + 1))/$n"
  fi
  mine=$(cargo mutants --list $FILTER $shard | count_lines)
  mkdir -p "$out"
  printf 'shard=%s\nmutants=%s\nexcluded=%s\n' "$label" "$mine" "$((all - total))" >"$out/area.env"
  echo "mutants: $area shard $label, $mine of the area's $total mutants ($((all - total)) left out)"
  code=0
  cargo mutants $FILTER $shard --jobs "$JOBS" --output "$out" "$@" || code=$?
  case $code in
    0 | 2 | 3) return 0 ;;
    *) return "$code" ;;
  esac
}

run_web() {
  area=$1 k=$2
  shift 2
  out=$ROOT/target/mutants/$area
  case "$area" in
    edge) app=apps/edge files=$(edge_files) ;;
    *) app=apps/web files=$(web_files "${area#web-}") ;;
  esac
  [ -n "$files" ] || die "$area has no source files"
  total=$(echo "$files" | count_lines)
  n=$(shards "$total" "${MUTANTS_SHARD_SIZE:-$SIZE_WEB_FILES}")
  label="1/1"
  if [ -n "$k" ] && [ "$n" -gt 1 ]; then
    files=$(echo "$files" | awk -v n="$n" -v k="$((k % n))" '(NR - 1) % n == k')
    label="$((k % n + 1))/$n"
  fi
  mine=$(echo "$files" | count_lines)
  mkdir -p "$out"
  printf 'shard=%s\nfiles=%s\n' "$label" "$mine" >"$out/area.env"
  echo "mutants: $area shard $label, $mine of the area's $total files"
  mutate=$(echo "$files" | sed "s|^$app/||" | paste -sd, -)
  (cd "$app" && MUTANTS_OUT=$out pnpm exec stryker run stryker.config.mjs --mutate "$mutate" "$@")
}

run_area() {
  area=$1 k=
  shift
  if [ $# -gt 0 ] && [ "$1" != "--" ]; then
    k=$1
    shift
    case "$k" in
      *[!0-9]*) die "k is a shard number (0, 1, 2…), not $k" ;;
    esac
  fi
  [ "${1:-}" != "--" ] || shift
  case "$area" in
    cards)
      [ -n "$k" ] || die "cards takes a k (it picks the set); for every card, run each cards-<set>"
      areas=$(card_areas)
      count=$(echo "$areas" | count_lines)
      set=$(echo "$areas" | sed -n "$((k % count + 1))p")
      rust_area "$set"
      run_rust "$set" "$((k / count))" "$@"
      ;;
    web-* | edge) run_web "$area" "$k" "$@" ;;
    *)
      rust_area "$area" || die "no area $area (sh scripts/mutants.sh areas lists them)"
      run_rust "$area" "$k" "$@"
      ;;
  esac
}

in_diff() {
  base=${1:-origin/main}
  [ $# -lt 1 ] || shift
  [ "${1:-}" != "--" ] || shift
  out=target/mutants/in-diff
  mkdir -p "$out"
  git diff "$(git merge-base "$base" HEAD)" -- '*.rs' >"$out/diff.patch"
  if ! [ -s "$out/diff.patch" ]; then
    echo "mutants: no Rust line changed since $base"
    return 0
  fi
  cargo mutants --in-diff "$out/diff.patch" --jobs "$JOBS" --output "$out" "$@"
}

list_areas() {
  echo "| area | mutants | left out | shards |"
  echo "|---|---:|---:|---:|"
  for area in engine $(card_areas) ai run shared server tools wasm; do
    rust_area "$area"
    total=$(cargo mutants --list $FILTER | count_lines)
    all=$(cargo mutants --list --no-config $FILTER | count_lines)
    echo "| $area | $total | $((all - total)) | $(shards "$total" "$SIZE") |"
  done
  for layer in animations effects audio cards drag settings wire wasm; do
    total=$(web_files "$layer" | count_lines)
    echo "| web-$layer | $total files | | $(shards "$total" "$SIZE_WEB_FILES") |"
  done
  total=$(edge_files | count_lines)
  echo "| edge | $total files | | $(shards "$total" "$SIZE_WEB_FILES") |"
}

# A row of mutants.yml's summary table and the survivors under it. Killed counts the mutants that
# timed out (a mutant that hangs the tests is caught); the score is killed over killed and survived.
score() {
  area=$1
  out=target/mutants/$area
  shard=? mutants=? excluded=
  [ -f "$out/area.env" ] && . "./$out/area.env"
  if [ -f "$out/mutation.json" ]; then
    python3 - "$out/mutation.json" "$area" "$shard" "$SURVIVORS_SHOWN" <<'EOF'
import collections, json, sys
path, area, shard, shown = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
files = json.load(open(path))["files"]
status = collections.Counter(m["status"] for f in files.values() for m in f["mutants"])
timeout = status["Timeout"]
killed = status["Killed"] + timeout
survived = status["Survived"] + status["NoCoverage"]
unviable = status["CompileError"] + status["RuntimeError"]
judged = killed + survived
score = f"{100 * killed / judged:.1f}%" if judged else "–"
print(f"| {area} | {shard} | {killed} | {survived} | {timeout} | {unviable} | {status['Ignored']} | {score} |")
lines = [f"{name}:{m['location']['start']['line']} {m['mutatorName']}: {m.get('replacement', '')!r} ({m['status']})"
         for name, f in sorted(files.items()) for m in f["mutants"] if m["status"] in ("Survived", "NoCoverage")]
if lines:
    print(f"\n<details><summary>{area}: {len(lines)} survived</summary>\n\n```")
    print("\n".join(lines[:shown]) + ("\n…" if len(lines) > shown else ""))
    print("```\n</details>\n")
EOF
    return 0
  fi
  dir=$out/mutants.out
  [ -d "$dir" ] || {
    echo "| $area | $shard | no report | | | | | |"
    return 0
  }
  caught=0 missed=0 timeout=0 unviable=0
  [ -f "$dir/caught.txt" ] && caught=$(count_lines <"$dir/caught.txt")
  [ -f "$dir/missed.txt" ] && missed=$(count_lines <"$dir/missed.txt")
  [ -f "$dir/timeout.txt" ] && timeout=$(count_lines <"$dir/timeout.txt")
  [ -f "$dir/unviable.txt" ] && unviable=$(count_lines <"$dir/unviable.txt")
  killed=$((caught + timeout))
  judged=$((killed + missed))
  done=$((judged + unviable))
  pct=–
  [ "$judged" -eq 0 ] || pct=$(awk -v k="$killed" -v j="$judged" 'BEGIN { printf "%.1f%%", 100 * k / j }')
  [ "$done" = "$mutants" ] || shard="$shard ($done of $mutants tested)"
  echo "| $area | $shard | $killed | $missed | $timeout | $unviable | $excluded | $pct |"
  if [ "$missed" -gt 0 ]; then
    printf '\n<details><summary>%s: %s survived</summary>\n\n```\n' "$area" "$missed"
    head -n "$SURVIVORS_SHOWN" "$dir/missed.txt"
    [ "$missed" -le "$SURVIVORS_SHOWN" ] || echo "…"
    printf '```\n</details>\n\n'
  fi
}

case "${1:-}" in
  --in-diff)
    shift
    in_diff "$@"
    ;;
  areas) list_areas ;;
  score)
    [ $# -eq 2 ] || die "usage: sh scripts/mutants.sh score <area>"
    score "$2"
    ;;
  "" | -h | --help) sed -n '2,/^set -euf/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//' ;;
  *) run_area "$@" ;;
esac
