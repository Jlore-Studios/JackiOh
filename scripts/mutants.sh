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
#   sh scripts/mutants.sh score <area>             the finished run's row of the summary table, its
#                                                  survivors and its counts by file (an audit's rows)
#   sh scripts/mutants.sh floor <scores> [file]    each area's score in <scores> (what `score` printed
#                                                  for each area) held to its line of the floor,
#                                                  .github/mutants-floor.txt; fails naming each
#                                                  area under its floor
#
# The areas: engine, ai, run, shared, server, tools and wasm (each crate, the three WebAssembly ones
# as one); the cards by set, cards-<set> for each of crates/cards/src/scripts/<set>/ and cards-lib for
# the rest of the crate, and `cards` itself, whose k picks the set (k mod the sets) and then that
# set's shard; web-<layer> for each of Game.tsx's layers (animations, effects, audio, cards, drag,
# settings) and the client's two adapters (wire, wasm); edge for the Worker. Shard k of a Rust area
# holds the k-th of n runs of its mutants in the order cargo-mutants finds them, file after file
# (its `slice` sharding, the default since 26.0), and of a web area the k-th run of its source lines,
# file after file (SIZE_WEB_LINES, below); the weeks walk every shard in turn.
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
# two mutants at a time. A mutant's time is mostly compiling its package and its tests again. In the
# run crate it is 3 s (259 mutants in 13 minutes on #3512's branch), so one shard holds the whole
# crate; in shared about a minute (243 mutants in 1 h 46 min, shard 16/21 on #3813's branch) and in
# the AI 3 minutes (118 in 2 h 52 min, shard 1/30 on main, 2026-11-12). The engine's are 6 minutes'
# build, since the cards are its tests' dev-dependency and are built again with it, and 2.5 minutes'
# tests, and the server's 8 to 12 minutes' tests: at 150 and 80 a shard, their first shards on main
# (2026-11-12) reached about 35 and 25 mutants before the step's five hours ended, so they hold 40 and
# 20 now, the server's tested one at a time (JOBS_SERVER, below). The rest are estimates until a weekly
# run times them: about a minute in the cards and four in the tools.
SIZE_ENGINE=40
SIZE_CARDS=250
SIZE_AI=120
SIZE_RUN=1000
SIZE_SHARED=250
SIZE_SERVER=20
SIZE_TOOLS=120
SIZE_WASM=200
# Source lines per Stryker shard. Stryker runs each mutant against only the tests that cover it, but a
# layer the board imports relates most of the web's tests, so a mutant takes about 10 s whatever its
# file: the drag's 1,639 lines held 1,729 mutants, tested at about 10 s each after an 11.5-minute first
# test run (#3512's branch), and animations.ts's 2,019 lines 1,671, of which the step's five hours
# reached 1,524 after a 31-minute first run (main, 2026-11-12). Stryker writes its report only at the
# end, so a shard must finish inside the step. A mutant of a component nearly every test renders runs
# far longer: web-cards' first 900 lines (CardBack.tsx, CardFace.tsx and more) took 4 h 51 min of the
# step's five hours on #3813's branch (2026-11-13), so a shard holds 600 lines.
# The files are packed in order, and a file longer than a shard is cut into ranges of lines
# (Stryker's `file:a-b`), so one long file never holds a shard past the step. A mutant whose code
# crosses a cut (a whole function's body, say) falls in neither range.
SIZE_WEB_LINES=600
# cargo-mutants' --jobs: mutants tested at once, each in its own copy of the tree. The server's one at a
# time, since its test binary alone takes 4 GB of a runner's 16 (TEST_MEMORY_SERVER, below).
JOBS=${MUTANTS_JOBS:-2}
JOBS_SERVER=${MUTANTS_JOBS:-1}
# The memory, in bytes, one test binary may take: 4 GiB of data (its heap and its other private
# writable mappings, its threads' stacks among them), set by prlimit as cargo's runner for the host,
# so only the tests and never rustc or the linker. A mutant that takes a loop's step away (`at += 1`
# as `at *= 1` in crates/shared's face/diff.rs, whose loop pushes a token each time round) fills a
# runner's 16 GB in seconds, long before the test timeout ends it, and the runner goes with it:
# crates/shared's ninth shard lost two runners so (mutants.yml on main, 2026-11-11). Under the cap the
# allocation past it fails, the test binary aborts, and the mutant is caught. Without prlimit
# (macOS), no cap.
TEST_MEMORY=${MUTANTS_TEST_MEMORY:-4294967296}
# The server's test binary, `server`, runs all of its 1,690 tests in one process (CI splits them four
# ways), and its data grows to 5.3 GB of address space, 4.0 GB resident, by the end (measured on #3813's
# branch, 2026-11-13, with the Stinkyscape+ catalog): under the 4 GiB cap every baseline aborted
# (mutants.yml run 43376574076), so no server mutant could be tested. It takes 8 GiB.
TEST_MEMORY_SERVER=${MUTANTS_TEST_MEMORY:-8589934592}
host=
if command -v prlimit >/dev/null 2>&1; then
  host=$(rustc -vV 2>/dev/null | sed -n 's/^host: //p' | tr 'a-z-' 'A-Z_')
fi
# Each test binary runs under prlimit with `$1` bytes of data, as cargo's runner for the host.
cap_tests() {
  if [ -n "$host" ]; then
    export "CARGO_TARGET_${host}_RUNNER=prlimit --data=$1 --"
  fi
}
cap_tests "$TEST_MEMORY"
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

# The cards crate reaches its scripts through the registry build.rs writes to OUT_DIR, which
# cargo-mutants cannot follow: it finds a crate's files by their `mod` lines. So a card set's area
# runs on a scratch copy of the tree whose cards lib.rs also declares every script as a module no
# build compiles (`#[cfg(any())]`): cargo-mutants finds the scripts there, and the registry that the
# copy's own build.rs writes compiles each mutated file. TREE is the copy, made once.
TREE=
cards_tree() {
  [ -z "$TREE" ] || return 0
  TREE=$(mktemp -d "${TMPDIR:-/tmp}/jackioh-mutants.XXXXXX")
  trap 'rm -rf "$TREE"' EXIT
  git ls-files -co --exclude-standard | while IFS= read -r file; do
    if [ -f "$file" ]; then echo "$file"; fi
  done | tar -cf - -T - | tar -xf - -C "$TREE"
  for file in $(git ls-files -co --exclude-standard -- 'crates/cards/src/scripts/*/*.rs'); do
    rel=${file#crates/cards/src/}
    module=$(echo "${rel#scripts/}" | sed 's/\.rs$//; s|/|_|')
    printf '\n#[cfg(any())]\n#[path = "%s"]\nmod mutants_%s;\n' "$rel" "$module"
  done >>"$TREE/crates/cards/src/lib.rs"
}

# FILTER (cargo-mutants' arguments that pick the area's mutants, its tree among them) and SIZE for a
# Rust area.
rust_area() {
  AREA_JOBS=$JOBS
  cap_tests "$TEST_MEMORY"
  case "$1" in
    engine) FILTER="-p jackioh-engine" SIZE=$SIZE_ENGINE ;;
    ai) FILTER="-p jackioh-ai" SIZE=$SIZE_AI ;;
    run) FILTER="-p jackioh-run" SIZE=$SIZE_RUN ;;
    shared) FILTER="-p jackioh-shared" SIZE=$SIZE_SHARED ;;
    server)
      FILTER="-p jackioh-server" SIZE=$SIZE_SERVER AREA_JOBS=$JOBS_SERVER
      cap_tests "$TEST_MEMORY_SERVER"
      ;;
    tools) FILTER="-p jackioh-tools" SIZE=$SIZE_TOOLS ;;
    wasm) FILTER="-p jackioh-wasm -p jackioh-wasm-core -p jackioh-wasm-c" SIZE=$SIZE_WASM ;;
    cards-lib) FILTER="-p jackioh-cards -e crates/cards/src/scripts/**" SIZE=$SIZE_CARDS ;;
    cards-?*)
      [ -d "crates/cards/src/scripts/${1#cards-}" ] || return 1
      cards_tree
      FILTER="--dir $TREE -p jackioh-cards -f crates/cards/src/scripts/${1#cards-}/**" SIZE=$SIZE_CARDS
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
    wire) paths=apps/web/src/wire ;;
    wasm) paths=apps/web/src/wasm ;;
    *) return 1 ;;
  esac
  # Not the generated (ts-rs's types in generated/, the engine's and the server's constants), and not
  # the cards' procedural art (cards/art/, 155 files and 80,000 lines when it was left out): its
  # numbers draw a picture, so its mutant changes the picture, which a unit test cannot judge and
  # e2e's component and pixel specs do.
  git ls-files -- $paths | grep -E '\.tsx?$' |
    grep -vE '\.test\.tsx?$|\.d\.ts$|/test/|/generated/|/pkg/|/wire/(engineConfig|serverConfig)\.ts$|/cards/art/' || true
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
  out=$ROOT/target/mutants/$area
  total=$(cargo mutants --list $FILTER | count_lines)
  all=$(cargo mutants --list --no-config $FILTER | count_lines)
  n=$(shards "$total" "$SIZE")
  shard=
  label="1/1"
  if [ -n "$k" ] && [ "$n" -gt 1 ]; then
    shard="--shard $((k % n))/$n"
    label="$((k % n + 1))/$n"
  fi
  mkdir -p "$out"
  # The shard's mutants by name, as the outcome files name them, so `score` can name any it never
  # finished.
  cargo mutants --list $FILTER $shard >"$out/list.txt"
  mine=$(count_lines <"$out/list.txt")
  printf 'shard=%s\nmutants=%s\nexcluded=%s\n' "$label" "$mine" "$((all - total))" >"$out/area.env"
  echo "mutants: $area shard $label, $mine of the area's $total mutants ($((all - total)) left out)"
  code=0
  cargo mutants $FILTER $shard --jobs "$AREA_JOBS" --output "$out" "$@" || code=$?
  case $code in
    0 | 2 | 3) return 0 ;;
    *) return "$code" ;;
  esac
}

# An area's files as Stryker shards: "<shard> <pattern>" per line, each pattern a file or a range of
# its lines, packed in order to at most SIZE_WEB_LINES lines a shard.
web_shards() {
  echo "$1" | while IFS= read -r file; do
    lines=$(count_lines <"$file")
    if [ "$lines" -le "$SIZE_WEB_LINES" ]; then
      echo "$lines $file"
    else
      from=1
      while [ "$from" -le "$lines" ]; do
        to=$((from + SIZE_WEB_LINES - 1))
        [ "$to" -le "$lines" ] || to=$lines
        echo "$((to - from + 1)) $file:$from-$to"
        from=$((to + 1))
      done
    fi
  done | awk -v most="$SIZE_WEB_LINES" '
    { if (lines > 0 && lines + $1 > most) { shard++; lines = 0 } lines += $1; print shard + 0, $2 }'
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
  packed=$(web_shards "$files")
  n=$(($(echo "$packed" | tail -n 1 | cut -d' ' -f1) + 1))
  label="1/1"
  mine=$(echo "$packed" | cut -d' ' -f2)
  if [ -n "$k" ] && [ "$n" -gt 1 ]; then
    mine=$(echo "$packed" | awk -v k="$((k % n))" '$1 == k { print $2 }')
    label="$((k % n + 1))/$n"
  fi
  mkdir -p "$out"
  printf 'shard=%s\nfiles=%s\n' "$label" "$(echo "$mine" | count_lines)" >"$out/area.env"
  echo "mutants: $area shard $label, of the area's $total files:" $mine
  mutate=$(echo "$mine" | sed "s|^$app/||" | paste -sd, -)
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
  out=$ROOT/target/mutants/in-diff
  mkdir -p "$out"
  git diff "$(git merge-base "$base" HEAD)" -- '*.rs' >"$out/diff.patch"
  if ! [ -s "$out/diff.patch" ]; then
    echo "mutants: no Rust line changed since $base"
    return 0
  fi
  dir=
  # A changed server line is tested by the server's test binary, which takes its own cap (above).
  if grep -q '^+++ b/crates/server/' "$out/diff.patch"; then
    JOBS=$JOBS_SERVER
    cap_tests "$TEST_MEMORY_SERVER"
  fi
  if grep -q '^+++ b/crates/cards/src/scripts/' "$out/diff.patch"; then
    cards_tree
    dir="--dir $TREE"
  fi
  cargo mutants $dir --in-diff "$out/diff.patch" --jobs "$JOBS" --output "$out" "$@"
}

# Every area, one per line, as a run's summary table names it (`cards` reports as its week's set).
known_areas() {
  echo engine
  card_areas
  for area in ai run shared server tools wasm; do echo "$area"; done
  for layer in animations effects audio cards drag settings wire wasm; do echo "web-$layer"; done
  echo edge
}

list_areas() {
  echo "| area | mutants | left out | shards |"
  echo "|---|---:|---:|---:|"
  for area in $(known_areas); do
    case "$area" in
      edge) files=$(edge_files) ;;
      web-*) files=$(web_files "${area#web-}") ;;
      *)
        rust_area "$area"
        total=$(cargo mutants --list $FILTER | count_lines)
        all=$(cargo mutants --list --no-config $FILTER | count_lines)
        echo "| $area | $total | $((all - total)) | $(shards "$total" "$SIZE") |"
        continue
        ;;
    esac
    total=$(echo "$files" | count_lines)
    echo "| $area | $total files | | $(($(web_shards "$files" | tail -n 1 | cut -d' ' -f1) + 1)) |"
  done
}

# The floor: each `<area> <percent>` line of the floor file holds that area's score in a finished
# run's table (the rows `score` printed, gathered in one file) to at least its percent. An area the
# run did not score, whose score is `–` (no mutant judged), or that has no line is not held. A line
# that names no area fails, so a mistyped area never goes ungated unseen. Each area under its floor
# is an error naming it, and the floor fails once every line is read.
floor() {
  scores=$1 file=${2:-.github/mutants-floor.txt}
  [ -f "$scores" ] || die "no scores file $scores"
  [ -f "$file" ] || die "no floor file $file"
  known=$(known_areas)
  fail=0
  while read -r area min rest; do
    case "$area" in "" | "#"*) continue ;; esac
    if ! echo "$known" | grep -qx "$area"; then
      echo "::error title=mutation floor::$file names no area $area (sh scripts/mutants.sh areas lists them)"
      fail=1
      continue
    fi
    case "$min" in
      "" | *[!0-9.]* | *.*.*)
        echo "::error title=mutation floor::$file: $area's floor is a percent, not \"$min\""
        fail=1
        continue
        ;;
    esac
    [ -z "$rest" ] || {
      echo "::error title=mutation floor::$file: $area's line holds more than its floor: $rest"
      fail=1
      continue
    }
    # A row's cells, split at its pipes: $2 the area, $9 the score.
    score=$(awk -F'|' -v area="$area" '{ a = $2; gsub(/ /, "", a) }
      /^\|/ && a == area { s = $9; gsub(/[ %]/, "", s); print "=" s; exit }' "$scores")
    case "$score" in
      "") continue ;;
      = | *[!=0-9.]*)
        echo "mutants: $area has no score this run, not held to its $min%"
        continue
        ;;
    esac
    score=${score#=}
    if awk -v s="$score" -v f="$min" 'BEGIN { exit !(s + 0 < f + 0) }'; then
      echo "::error title=mutation floor::$area scored $score%, under its floor of $min% ($file)"
      fail=1
    else
      echo "mutants: $area scored $score%, at or over its floor of $min%"
    fi
  done <"$file"
  return "$fail"
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
# By file, an audit's rows (no leading pipe, so the report's summary table takes none of them).
killed_as = {"Killed", "Timeout"}
survived_as = {"Survived", "NoCoverage"}
print(f"<details><summary>{area} by file</summary>\n\nfile | killed | survived | ignored\n--- | ---: | ---: | ---:")
for name, f in sorted(files.items()):
    by = collections.Counter(m["status"] for m in f["mutants"])
    print(f"{name} | {sum(by[s] for s in killed_as)} | {sum(by[s] for s in survived_as)} | {by['Ignored']}")
print("\n</details>\n")
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
  # The mutants the shard never finished (a build or a test that outlived the step), named when
  # their count is the shortfall's, so the next audit can see which held the job.
  if [ "$done" != "$mutants" ] && [ -f "$out/list.txt" ]; then
    for outcome in caught missed timeout unviable; do
      if [ -f "$dir/$outcome.txt" ]; then cat "$dir/$outcome.txt"; fi
    done | LC_ALL=C sort >"$out/finished.txt"
    LC_ALL=C sort "$out/list.txt" | LC_ALL=C comm -23 - "$out/finished.txt" >"$out/unfinished.txt"
    unfinished=$(count_lines <"$out/unfinished.txt")
    if [ "$unfinished" -gt 0 ] && [ "$unfinished" -eq $((mutants - done)) ]; then
      printf '<details><summary>%s: %s not finished</summary>\n\n```\n' "$area" "$unfinished"
      head -n "$SURVIVORS_SHOWN" "$out/unfinished.txt"
      printf '```\n</details>\n\n'
    fi
  fi
  # By file, an audit's rows (no leading pipe, so the report's summary table takes none of them).
  [ "$done" -gt 0 ] || return 0
  printf '<details><summary>%s by file</summary>\n\nfile | killed | survived | unviable\n--- | ---: | ---: | ---:\n' "$area"
  for outcome in caught timeout missed unviable; do
    if [ -f "$dir/$outcome.txt" ]; then sed "s/:.*/ $outcome/" "$dir/$outcome.txt"; fi
  done | awk '{ n[$1 " " $2]++; files[$1] = 1 }
    END {
      for (f in files) printf "%s | %d | %d | %d\n", f, n[f " caught"] + n[f " timeout"], n[f " missed"], n[f " unviable"]
    }' | sort
  printf '\n</details>\n\n'
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
  floor)
    [ $# -ge 2 ] && [ $# -le 3 ] || die "usage: sh scripts/mutants.sh floor <scores> [file]"
    shift
    floor "$@"
    ;;
  "" | -h | --help) sed -n '2,/^set -euf/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//' ;;
  *) run_area "$@" ;;
esac
