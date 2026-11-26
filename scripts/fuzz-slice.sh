#!/usr/bin/env bash
# The merge queue's slice of the daily fuzz (ci.yml's `rust (fuzz slice k/K)`, the owner's call of
# 2026-11-25). Seeds 1-SEEDS of the plain and the handicapped wave, and 1-DUOS_SEEDS of the Duos wave,
# split over the job's shards, on the build of the commit the queue would land. The daily super run
# plays seeds 1-13,200 (super.yml's FUZZ_SEEDS) a day after a change lands; this plays the first of
# them before it lands, so a change that breaks a fuzz invariant leaves the queue instead of turning
# the next daily run red. From 2026-11-19 every daily run on `_staging` was red, mostly on fuzz seeds
# that newer commits broke while the last fix was being built.
#
# It fails a change only for what that change broke. A seed that fails on the queue's commit is played
# again on the build of its base (main as the queue found it). If it fails there too, the failure is
# main's already: a warning, left to the daily run's issue, so a red main never blocks every merge
# behind it, the fix's own included. If it passes there, the change broke it, and the job fails.
#
#   bash scripts/fuzz-slice.sh play <k> <K>   play shard k of K of each wave on $JACKIOH (default
#                                             target/release/jackioh), write each failing seed as a
#                                             `<wave> <seed>` line to $OUT/failed.txt, and
#                                             `failed=true|false` to $GITHUB_OUTPUT. It fails only when
#                                             a wave ends with no failures to read (a crash).
#   bash scripts/fuzz-slice.sh judge <jackioh>   play each seed of $OUT/failed.txt again on <jackioh>,
#                                             the base's build, and fail when any passes there.
#
# SEEDS and DUOS_SEEDS come from ci.yml (the job's env); K is scripts/ci-shards.toml's [fuzz] slice.
# Shard k plays seeds (k-1)*N/K+1 .. k*N/K of each wave, so together the shards play 1..N once,
# whatever K is, and no count can skip a seed. A seed number deals the same game here as in the daily
# run, so a failure here replays with `cargo jackioh fuzz [--handicap|--duos] --from <seed> --seeds 1`.
# A seed that fails on both builds but for a different reason counts as main's (a known gap: the
# messages carry turns and ids that any change to the game moves); the daily run still plays it.
# bot/tests/test_fuzz_slice.py tests this script against a stand-in for jackioh.
set -euo pipefail

: "${JACKIOH:=target/release/jackioh}"
: "${OUT:=${RUNNER_TEMP:-/tmp}/fuzz-slice}"
: "${GITHUB_OUTPUT:=/dev/null}"

WAVES="plain handicap duos"

# The flags that play wave $1, into the array `args`.
wave_args() {
  case $1 in
    plain) args=() ;;
    handicap) args=(--handicap) ;;
    duos) args=(--duos) ;;
    *)
      echo "::error::fuzz-slice: no wave named $1" >&2
      exit 2
      ;;
  esac
}

play() {
  : "${SEEDS:?SEEDS, the plain and handicapped waves' seeds (ci.yml)}"
  : "${DUOS_SEEDS:?DUOS_SEEDS, the Duos wave's seeds (ci.yml)}"
  local k=$1 total=$2 wave n from to count status seeds seed
  if ! [ "$k" -ge 1 ] 2>/dev/null || ! [ "$k" -le "$total" ] 2>/dev/null; then
    echo "::error::fuzz-slice: shard $k of $total: k runs from 1 to its total" >&2
    exit 2
  fi
  mkdir -p "$OUT"
  : > "$OUT/failed.txt"
  for wave in $WAVES; do
    n=$SEEDS
    [ "$wave" != duos ] || n=$DUOS_SEEDS
    from=$(((k - 1) * n / total + 1))
    to=$((k * n / total))
    if [ "$to" -lt "$from" ]; then
      echo "$wave wave: no seeds in shard $k of $total"
      continue
    fi
    count=$((to - from + 1))
    wave_args "$wave"
    status=0
    "$JACKIOH" fuzz "${args[@]}" --from "$from" --seeds "$count" --json > "$OUT/$wave.json" || status=$?
    if [ "$status" = 0 ]; then
      echo "$wave wave, seeds $from..$to: every seed passed"
      continue
    fi
    seeds=$(jq -r '.failures[].seed' "$OUT/$wave.json" 2>/dev/null) || seeds=
    if [ -z "$seeds" ]; then
      echo "::error::fuzz-slice: the $wave wave (seeds $from..$to) failed (exit $status) with no failure to read; replay it with: jackioh fuzz ${args[*]} --from $from --seeds $count"
      exit 1
    fi
    echo "$wave wave, seeds $from..$to: $(echo "$seeds" | wc -l | tr -d ' ') failed on this build:"
    jq -r '.failures[] | "  seed \(.seed): \(.message)"' "$OUT/$wave.json"
    for seed in $seeds; do
      echo "$wave $seed" >> "$OUT/failed.txt"
    done
  done
  if [ -s "$OUT/failed.txt" ]; then
    echo "failed=true" >> "$GITHUB_OUTPUT"
    echo "Each failing seed is played again on the base's build next: it fails this job only if it passes there."
  else
    echo "failed=false" >> "$GITHUB_OUTPUT"
  fi
}

judge() {
  local base=$1 wave seed new=0 old=0
  if ! [ -x "$base" ]; then
    echo "::error::fuzz-slice: no base build at $base to tell this change's failures from main's" >&2
    exit 1
  fi
  if ! [ -s "$OUT/failed.txt" ]; then
    echo "no seed failed on this build"
    return 0
  fi
  while read -r wave seed; do
    wave_args "$wave"
    if "$base" fuzz "${args[@]}" --from "$seed" --seeds 1 > /dev/null 2>&1; then
      new=$((new + 1))
      echo "::error::the $wave wave's seed $seed fails on this build and passes on its base, so this change broke it. Replay it: cargo jackioh fuzz ${args[*]:+${args[*]} }--from $seed --seeds 1"
    else
      old=$((old + 1))
      echo "::warning::the $wave wave's seed $seed fails on the base too: main's already, not this change's (the daily super run's issue holds main's)"
    fi
  done < "$OUT/failed.txt"
  echo "$new seed(s) this change broke, $old main's already"
  [ "$new" = 0 ]
}

case "${1:-}" in
  play)
    [ $# = 3 ] || { echo "usage: bash scripts/fuzz-slice.sh play <k> <K>" >&2; exit 2; }
    play "$2" "$3"
    ;;
  judge)
    [ $# = 2 ] || { echo "usage: bash scripts/fuzz-slice.sh judge <jackioh>" >&2; exit 2; }
    judge "$2"
    ;;
  *)
    echo "usage: bash scripts/fuzz-slice.sh play <k> <K> | judge <jackioh>" >&2
    exit 2
    ;;
esac
