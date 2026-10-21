#!/bin/sh
# Runs the database suite — the Postgres store (`Db::Pg`, crates/server/src/db/pg.rs) against a
# real Postgres.
#
#   sh crates/server/tests/db/run.sh               # as CI's db job runs it
#   KEEP_DB=1 sh crates/server/tests/db/run.sh     # leave the container up for poking at
#   DB_PORT=55555 sh crates/server/tests/db/run.sh
#
# Needs Docker and cargo, and takes a couple of seconds once the server crate is built. This is
# NOT part of `cargo test` on its own and never will be: without DATABASE_URL, `cargo test` runs the
# store suites against the in-memory fake (`Db::Fake`, src/db/fake.rs) only and must stay hermetic.
# The same contract runs in both places — `tests/store/contract.rs` and `tests/store/redeem_race.rs`
# run every case against `Db::Fake` always and against `Db::Pg` when DATABASE_URL is set, which is
# what this script sets — and that pair is the point.
#
# Steps, in the order a real bring-up takes them (docs/architecture.md):
#   1. a throwaway Postgres, with its port published so the store can reach it from the host;
#   2. tests/db/bootstrap.sql — the Supabase-managed pieces (roles, auth.users, auth.uid, the
#      privileges service_role has in a real project);
#   3. `jackioh-server migrate` (src/db/migrate.rs) — the real migration runner, over every real
#      migration (crates/server/migrations, compiled into the binary);
#   4. tests/db/grants.sql — service_role's table privileges, repeated after the migrations;
#   5. the server crate's `store::` tests, with DATABASE_URL pointing at it, one at a time per
#      database: shared tables, `truncate` between tests, so no two may run at once on one. The
#      suite is spread over DB_SHARDS (default 3) databases, each copied from the migrated one,
#      and the shards run side by side. Steps 1 and 2 run alongside the build, not after it.
#
# Exit status is the gate: non-zero if any step or any test fails.
set -e

REPO=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
HERE="$REPO/crates/server/tests/db"
CONTAINER=jackioh-pg-store-test
PORT=${DB_PORT:-55433}
URL="postgres://postgres:postgres@127.0.0.1:$PORT/jackioh"

cleanup() {
  if [ "${KEEP_DB:-0}" = "1" ]; then
    echo "--- container $CONTAINER left running on port $PORT (docker rm -f $CONTAINER) ---"
  else
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  fi
}
LOGDIR=$(mktemp -d)
trap 'cleanup; rm -rf "$LOGDIR"' EXIT

# Postgres comes up (steps 1 and 2) while the build compiles, which is most of this script's time and
# needs none of it: the container is idle during a cold compile, and its readiness is waited for.
bring_up_postgres() {
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  # The data directory in memory and fsync off: a throwaway database that no test restarts, whose
  # per-test `truncate` and commits were a large part of the suite's time on disk (116 s to 74 s).
  docker run -d --name "$CONTAINER" -e POSTGRES_PASSWORD=postgres -p "$PORT":5432 \
    --tmpfs /var/lib/postgresql/data postgres:16 \
    -c fsync=off -c synchronous_commit=off -c full_page_writes=off >/dev/null

  # Ready means the real server. The image's entrypoint first starts a temporary one for its own
  # setup, which answers on the socket but not on TCP and then shuts down, cutting off whatever is
  # connected (#100). Asked over TCP, pg_isready hears only the real server.
  i=0
  ready=0
  while [ "$i" -lt 60 ]; do
    if docker exec "$CONTAINER" pg_isready -h 127.0.0.1 -U postgres >/dev/null 2>&1; then
      ready=1
      break
    fi
    i=$((i + 1))
    sleep 1
  done
  if [ "$ready" -ne 1 ]; then
    echo "FAIL: postgres in $CONTAINER was not ready after ${i}s" >&2
    exit 1
  fi

  docker exec "$CONTAINER" psql -U postgres -q -c 'create database jackioh' >/dev/null
  docker cp "$HERE/bootstrap.sql" "$CONTAINER":/tmp/ >/dev/null
  docker exec "$CONTAINER" psql -U postgres -v ON_ERROR_STOP=1 -q -d jackioh -f /tmp/bootstrap.sql
}

echo "--- postgres up and bootstrapped (the Supabase-managed pieces), alongside the build ---"
bring_up_postgres &
postgres_pid=$!

# Built once: in the test profile, `cargo test`'s, and with the tests, so with the features the tests
# turn on. A plain `cargo build` (the dev profile, which Cargo.toml's test-profile overrides make
# different) or a `cargo run` (no tests, so other features) would compile the workspace again, so the
# migrate step below runs the binary this build made. Exactly what the steps below run, and no more:
# the `jackioh-server` executable (for `migrate`) and the `server_store` test target, not the server's
# other test binaries.
echo "--- build (jackioh-server and its store tests) ---"
cargo build --profile test --manifest-path "$REPO/Cargo.toml" -p jackioh-server --bin jackioh-server --test server_store || {
  kill "$postgres_pid" 2>/dev/null || true
  exit 1
}
SERVER_BIN="${CARGO_TARGET_DIR:-$REPO/target}/debug/jackioh-server"
wait "$postgres_pid"

echo "--- migrations (src/db/migrate.rs) ---"
DATABASE_URL="$URL" "$SERVER_BIN" migrate

echo "--- grants ---"
docker cp "$HERE/grants.sql" "$CONTAINER":/tmp/ >/dev/null
docker exec "$CONTAINER" psql -U postgres -v ON_ERROR_STOP=1 -q -d jackioh -f /tmp/grants.sql

# The cases share one database and truncate it, so within one database they run one at a time. The
# suite is waiting on Postgres, not computing (about a third of a core), so it runs in DB_SHARDS
# (default 3) databases at once: copies of the migrated one, each given every SHARDS-th test of
# `--list`'s order. Each shard still runs its tests with `--test-threads=1` on a database of its own.
SHARDS=${DB_SHARDS:-3}
echo "--- cargo test (crates/server/tests/store, $SHARDS databases) ---"
run_tests() {
  cargo test --manifest-path "$REPO/Cargo.toml" -p jackioh-server --test server_store "$@"
}
run_tests store:: -- --list 2>/dev/null | sed -n 's/: test$//p' >"$LOGDIR/tests.txt"
if [ ! -s "$LOGDIR/tests.txt" ]; then
  echo "FAIL: no store:: tests listed" >&2
  exit 1
fi

k=1
while [ "$k" -le "$SHARDS" ]; do
  if [ "$k" -gt 1 ]; then
    docker exec "$CONTAINER" psql -U postgres -q -c "create database jackioh_$k template jackioh" >/dev/null
  fi
  k=$((k + 1))
done

k=1
pids=""
while [ "$k" -le "$SHARDS" ]; do
  db=jackioh
  [ "$k" -gt 1 ] && db="jackioh_$k"
  awk -v n="$SHARDS" -v k="$k" 'NR % n == k % n' "$LOGDIR/tests.txt" >"$LOGDIR/shard_$k.txt"
  (
    # shellcheck disable=SC2046
    DATABASE_URL="postgres://postgres:postgres@127.0.0.1:$PORT/$db" \
      run_tests -- --exact --test-threads=1 $(cat "$LOGDIR/shard_$k.txt") >"$LOGDIR/shard_$k.log" 2>&1
  ) &
  pids="$pids $!"
  k=$((k + 1))
done

failed=0
k=1
for pid in $pids; do
  wait "$pid" || failed=1
  echo "--- shard $k ---"
  cat "$LOGDIR/shard_$k.log"
  k=$((k + 1))
done
exit "$failed"
