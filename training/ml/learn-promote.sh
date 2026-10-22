#!/usr/bin/env bash
# The learn lane's promotion on main as it is now (#466; .github/workflows/learn.yml, training/learn.md
# "Running unattended"). retrain.sh fits and screens against main as it was when the run began, an hour
# or two earlier, and main moves about once an hour in what a gate depends on (the AI, the engine, the
# cards). CI's training gate replays the gate against main as it is when the pull request opens
# (`promote --verify`), and runs the practice-claim tests on the merge, so the promotion is gated and
# its fixture recorded on the newest main: the screened network goes onto `origin/main` as fetched,
# the gate plays against that main's AI, and the pull request opens only if main has not moved in
# those paths since. When it moved during the gate, the network goes onto the newer main and is gated
# again, at most --tries times. A gate that does not pass ends it: the network is never gated again
# after a loss, so no pass is picked out of several.
#
#   training/ml/learn-promote.sh --out <retrain.sh's --out> --games <n> [--tries 3]
#
# Run after retrain.sh's screen passed, in its tree (the network in crates/ai/src/learned/weights.json,
# <out>/parent-jackioh the build of the commit it checked out), with pnpm installed for the fixture.
# Exits 0 with ai/learn checked out, ready to push; 1 when a gate does not pass or the AI's tests fail
# on the newer main; 3 when main moved through every try. Writes <out>/promote.md,
# <out>/promotion.patch and <out>/deliver.json, whose "stage" says how it ended: "ready", "same" (the
# network is main's own), "tests", "gate" or "moving".
set -euo pipefail

out="" games="" tries=3
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --games) games="$2"; shift 2 ;;
    --tries) tries="$2"; shift 2 ;;
    *) echo "learn-promote.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$out" ] && [ -n "$games" ] || { echo "learn-promote.sh: --out and --games are required" >&2; exit 2; }

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$root"
out=$(cd "$out" && pwd)
say() { echo "learn-promote: $*"; }
decided() { printf '%s\n' "$1" > "$out/deliver.json"; }

# What the gate's two builds and the fixture's game are made of. A change to any of them between the
# gate's main and the pull request's base gives CI another parent, or another fixture game, than the
# lane's. Anything else may move: CI's other jobs test the merge as for any pull request.
play_paths=(crates/ai crates/engine crates/cards crates/wasm crates/tools/src/promote.rs
  crates/tools/src/arena.rs crates/server/tests/fixtures/practice-claim.json apps/web/src/practice
  Cargo.lock rust-toolchain.toml)
switch_off='^pub const AI_EVAL_KIND: EvalKind = EvalKind::Hand;'
switch_on='pub const AI_EVAL_KIND: EvalKind = EvalKind::Learned;'

cp crates/ai/src/learned/weights.json "$out/weights-screened.json"
screened_on=$(git rev-parse HEAD)
git config user.name "JackiOh learn lane"
git config user.email "learn-lane@users.noreply.github.com"

attempt=0
while [ "$attempt" -lt "$tries" ]; do
  attempt=$((attempt + 1))
  git fetch --quiet origin main
  base=$(git rev-parse origin/main)
  git checkout --quiet --force -B ai/learn "$base"
  cp "$out/weights-screened.json" crates/ai/src/learned/weights.json
  sed -i "s/$switch_off/$switch_on/" crates/ai/src/config.rs
  grep -q "^$switch_on" crates/ai/src/config.rs

  if [ "$base" = "$screened_on" ]; then
    parent="$out/parent-jackioh"
  else
    # main moved since the screen: its AI is the parent now, and its AI's tests must pass with the
    # network, as they will in CI.
    say "try $attempt on main ${base:0:9}: the parent built again, and the AI's tests with the network"
    rm -rf "$out/main-src"
    git worktree add --quiet --detach "$out/main-src" "$base"
    cargo build --release -p jackioh-tools --quiet --manifest-path "$out/main-src/Cargo.toml" \
      --target-dir "$out/main-target"
    cp "$out/main-target/release/jackioh" "$out/parent-$attempt"
    git worktree remove --force "$out/main-src"
    parent="$out/parent-$attempt"
    if ! cargo test -p jackioh-ai > "$out/ai-tests-main-$attempt.txt" 2>&1; then
      say "the network fails the AI's tests on main ${base:0:9} ($out/ai-tests-main-$attempt.txt)"
      decided "{\"stage\":\"tests\",\"attempt\":$attempt,\"base\":\"$base\"}"
      exit 1
    fi
  fi

  git add crates/ai/src/learned/weights.json crates/ai/src/config.rs
  if git diff --cached --quiet; then
    say "the network is main's own: nothing to promote"
    decided "{\"stage\":\"same\",\"attempt\":$attempt,\"base\":\"$base\"}"
    exit 1
  fi
  git commit --quiet -m "Stack (learn): the learned evaluation retrained on $games games" \
    -m "$(cat "$out/summary.json")"
  say "try $attempt: the gate against main ${base:0:9}"
  cargo build --release -p jackioh-tools --quiet
  status=0
  target/release/jackioh promote --lane learn --parent-bin "$parent" > "$out/promote.md" || status=$?
  cp "$out/promote.md" "$out/promote-$attempt.md"
  if [ "$status" != 0 ]; then
    say "the gate did not pass on main ${base:0:9}"
    decided "{\"stage\":\"gate\",\"attempt\":$attempt,\"base\":\"$base\"}"
    exit 1
  fi
  generation=$(jq -r .generation crates/ai/generation.json)
  git add crates/ai/generation.json training/history/learn.jsonl
  git commit --quiet -m "AI gen $generation (learn): the learned evaluation, retrained on $games games" \
    -m "$(cat "$out/promote.md")"

  # The practice game the AI's own moves are recorded in (R1303002), which the server's test re-runs
  # move for move: the promoted AI plays it differently, so it records it again into the promotion
  # commit, the one file outside crates/ai/** and training/history/** CI lets a lane change.
  say "the practice-claim fixture, recorded again by the promoted AI"
  JACKIOH_WASM_LUAU=0 sh scripts/build-wasm.sh > "$out/wasm-$attempt.txt" 2>&1
  JACKIOH_RECORD_PRACTICE_CLAIM=1 pnpm --dir apps/web exec vitest run src/practice/claim.test.ts \
    > "$out/claim-$attempt.txt" 2>&1
  pnpm --dir apps/web exec vitest run src/practice/claim.test.ts >> "$out/claim-$attempt.txt" 2>&1
  git add crates/server/tests/fixtures/practice-claim.json
  git commit --quiet --amend --no-edit
  git format-patch -2 --stdout > "$out/promotion.patch"

  git fetch --quiet origin main
  if git diff --quiet "$base" origin/main -- "${play_paths[@]}"; then
    say "main has not moved in what the gate depends on: ready to push"
    decided "{\"stage\":\"ready\",\"attempt\":$attempt,\"base\":\"$base\",\"generation\":$generation}"
    exit 0
  fi
  say "main moved in what the gate depends on during try $attempt:"
  git log --oneline "$base..origin/main" -- "${play_paths[@]}" | head -10
done
decided "{\"stage\":\"moving\",\"attempt\":$attempt}"
say "main moved through all $tries tries"
exit 3
