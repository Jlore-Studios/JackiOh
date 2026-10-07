#!/bin/sh
# Runs the database suite — the Postgres store (`Db::Pg`, crates/server/src/db/pg.rs) against a
# real Postgres.
#
#   pnpm test:db             # or: sh crates/server/tests/db/run.sh
#   KEEP_DB=1 pnpm test:db   # leave the container up for poking at
#   DB_PORT=55555 pnpm test:db
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
#   5. the server crate's `store::` tests, with DATABASE_URL pointing at it, one at a time: one
#      database, shared tables, `truncate` between tests, so no two may run at once.
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
trap cleanup EXIT

# Built before the container starts, so a cold compile is not counted against Postgres's readiness.
# Built once: in the test profile, `cargo test`'s, and with the tests, so with the features the tests
# turn on. A plain `cargo build` (the dev profile, which Cargo.toml's test-profile overrides make
# different) or a `cargo run` (no tests, so other features) would compile the workspace again, so the
# migrate step below runs the binary this build made.
echo "--- build (jackioh-server and its tests) ---"
cargo build --profile test --manifest-path "$REPO/Cargo.toml" -p jackioh-server --bins --tests
SERVER_BIN="${CARGO_TARGET_DIR:-$REPO/target}/debug/jackioh-server"

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" -e POSTGRES_PASSWORD=postgres -p "$PORT":5432 postgres:16 >/dev/null

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

echo "--- bootstrap (the Supabase-managed pieces) ---"
docker cp "$HERE/bootstrap.sql" "$CONTAINER":/tmp/ >/dev/null
docker exec "$CONTAINER" psql -U postgres -v ON_ERROR_STOP=1 -q -d jackioh -f /tmp/bootstrap.sql

echo "--- migrations (src/db/migrate.rs) ---"
DATABASE_URL="$URL" "$SERVER_BIN" migrate

echo "--- grants ---"
docker cp "$HERE/grants.sql" "$CONTAINER":/tmp/ >/dev/null
docker exec "$CONTAINER" psql -U postgres -v ON_ERROR_STOP=1 -q -d jackioh -f /tmp/grants.sql

echo "--- cargo test (crates/server/tests/store) ---"
DATABASE_URL="$URL" cargo test --manifest-path "$REPO/Cargo.toml" -p jackioh-server --test server store:: -- --test-threads=1
