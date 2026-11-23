#!/bin/sh
# scripts/mutants.sh's runner for each test binary (cargo's `target.<host>.runner`, inside prlimit's
# cap): it runs the binary and, when cargo-mutants' timeout sends SIGTERM to the process group, kills
# the binary outright. A test that installs a SIGTERM handler, as the server's drain test does
# (crates/server/tests/actor/ws_server.rs, through tokio, which never removes it), leaves its binary
# deaf to SIGTERM, so a timed-out binary ran on, holding its memory, until orphans took a runner's
# 16 GB: server shard 7/583 lost two runners so (#3836, 2026-11-15).
"$@" &
child=$!
trap 'kill -KILL "$child" 2>/dev/null' TERM INT HUP
while :; do
  wait "$child"
  status=$?
  # A trapped signal ends the wait early; wait again until the child is gone.
  kill -0 "$child" 2>/dev/null || break
done
exit "$status"
