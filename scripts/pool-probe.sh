#!/bin/sh
# The pool probe (#3117): runs the suites that draw from real cards with every random pool rotated one
# place (`jackioh-engine`'s `pool-probe` feature, `crates/engine/src/testkit/pool_probe.rs`), so each roll
# lands on its neighbour as it would after one more card joined the pool (R380). A test this fails leans on
# where a roll lands: it searches for the roll with `seed_where`/`cursor_where` (#848) or builds the
# position directly, and a test of a pool's content or order reads it under `with_pool_shift(0, …)`.
#
# Daily, not per pull request: it builds four crates' tests with a feature no per-PR group enables and runs
# the AI's and the server's games, which does not fit the five-minute job budget (`ci.yml`'s header). The
# per-PR check is `pool_probe`'s canary in the engine's own tests. Extra arguments go to `cargo test`.
set -eu

cd "$(dirname "$0")/.."
exec cargo test -p jackioh-cards -p jackioh-ai -p jackioh-server -p jackioh-tools \
  --features jackioh-engine/pool-probe --no-fail-fast "$@"
