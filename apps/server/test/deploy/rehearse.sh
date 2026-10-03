#!/bin/sh
# Rehearse a Render deploy of apps/server before it happens (`pnpm test:deploy`, CI's db job).
#
#   pnpm test:deploy        # or: sh apps/server/test/deploy/rehearse.sh
#
# Render runs render.yaml's startCommand, `release` (db:migrate, then db:seed-catalog) and then the
# server, and keeps the previous deploy serving if that fails. The 0013 outage was a migration that
# a superuser could apply and Supabase's migrating role could not: every other suite here migrates
# as the superuser `postgres`, so none could see it. This one reproduces what Render meets:
#
#   1. A database owned by `migrator`, a login role that is NOT a superuser, as Supabase's
#      `postgres` role is not; the Supabase-managed pieces (test/db/bootstrap.sql: roles, auth.users,
#      auth.uid()) are installed by the superuser, as Supabase installs them, and granted to it.
#   2. render.yaml's own startCommand and its non-secret env values (CATALOG_VERSION, NODE_ENV,
#      TRUSTED_PROXY_HOPS), read from the file so the rehearsal cannot drift from the deploy. The
#      secrets are placeholders: the server checks their shape at boot, and nothing here calls out.
#   3. The readiness probe Render uses, GET /api/catalog: it must answer within READY_SECONDS with
#      render.yaml's catalog version, the server's memory must fit MEMORY_LIMIT_MB (the free
#      instance's 512 MB, less headroom), and a second boot must migrate nothing.
#
# Needs Docker, or an existing superuser connection via REHEARSAL_PGHOST / REHEARSAL_PGPORT
# (a socket directory or a host) and REHEARSAL_PGPASSWORD; it creates and drops its own database.
set -eu

REPO=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
DB=jackioh_deploy_rehearsal
PORT_UNDER_TEST=18787
READY_SECONDS=120
MEMORY_LIMIT_MB=450
CONTAINER=jackioh-pg-deploy

PGHOST_=${REHEARSAL_PGHOST:-}
PGPORT_=${REHEARSAL_PGPORT:-5432}
PGPASSWORD_=${REHEARSAL_PGPASSWORD:-postgres}
SERVER_PID=""

cleanup() {
  if [ -n "$SERVER_PID" ]; then stop_server; fi
  if [ -z "${REHEARSAL_PGHOST:-}" ]; then docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT

if [ -z "$PGHOST_" ]; then
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker run -d --name "$CONTAINER" -e POSTGRES_PASSWORD="$PGPASSWORD_" -p 127.0.0.1:55433:5432 \
    postgres:16 >/dev/null
  PGHOST_=127.0.0.1
  PGPORT_=55433
fi

admin() { PGPASSWORD="$PGPASSWORD_" psql -h "$PGHOST_" -p "$PGPORT_" -U postgres -v ON_ERROR_STOP=1 -q "$@"; }

i=0
until admin -d postgres -c 'select 1' >/dev/null 2>&1; do
  i=$((i + 1))
  if [ "$i" -ge 60 ]; then echo "FAIL: postgres did not answer within 60 s" >&2; exit 1; fi
  sleep 1
done

echo "--- a Supabase-like database: owned by a role that is not a superuser ---"
admin -d postgres -c "drop database if exists $DB" -c "do \$\$
begin
  if not exists (select 1 from pg_roles where rolname = 'migrator') then
    create role migrator login password 'migrator' nosuperuser createrole createdb bypassrls;
  end if;
end \$\$" -c "create database $DB owner migrator"
admin -d "$DB" -f "$REPO/apps/server/test/db/bootstrap.sql" >/dev/null 2>&1
admin -d "$DB" \
  -c "grant anon, authenticated, service_role to migrator with admin option" \
  -c "grant all on schema public to migrator" \
  -c "grant usage on schema auth to migrator with grant option" \
  -c "grant all on all tables in schema auth to migrator with grant option" \
  -c "grant execute on all functions in schema auth to migrator with grant option"
if [ "$(admin -d "$DB" -tAc "select rolsuper from pg_roles where rolname = 'migrator'")" != "f" ]; then
  echo "FAIL: migrator must not be a superuser, or this rehearses nothing" >&2
  exit 1
fi

case "$PGHOST_" in
  /*) DATABASE_URL="postgresql://migrator:migrator@localhost:$PGPORT_/$DB?host=$PGHOST_" ;;
  *) DATABASE_URL="postgresql://migrator:migrator@$PGHOST_:$PGPORT_/$DB" ;;
esac

# render.yaml, read rather than restated: the start command, and each `- key:` whose next line is a
# literal `value:` (the `sync: false` secrets have none, and get placeholders below).
START=$(sed -n 's/^ *startCommand: *//p' "$REPO/render.yaml")
yaml_value() {
  awk -v key="$1" '
    $1 == "-" && $2 == "key:" { current = $3; next }
    current == key && $1 == "value:" { v = $2; gsub(/"/, "", v); print v; exit }
  ' "$REPO/render.yaml"
}
CATALOG_VERSION=$(yaml_value CATALOG_VERSION)
if [ -z "$START" ] || [ -z "$CATALOG_VERSION" ]; then
  echo "FAIL: could not read startCommand and CATALOG_VERSION from render.yaml" >&2
  exit 1
fi
echo "start command: $START"
echo "catalog version: $CATALOG_VERSION"

start_server() {
  (
    cd "$REPO"
    export DATABASE_URL CATALOG_VERSION
    export NODE_ENV="$(yaml_value NODE_ENV)"
    export TRUSTED_PROXY_HOPS="$(yaml_value TRUSTED_PROXY_HOPS)"
    export PORT="$PORT_UNDER_TEST"
    export SUPABASE_URL=https://rehearsal.supabase.co
    export SUPABASE_SECRET_KEY=rehearsal-placeholder
    export CODE_PEPPER=rehearsal-placeholder-pepper-of-a-length-the-server-accepts-0123456789
    export PUBLIC_ORIGINS=https://jackioh.vercel.app
    exec sh -c "$START"
  ) > "$1" 2>&1 &
  SERVER_PID=$!
}

# The start command's whole process tree: pnpm, the shells it runs, tsx and node.
tree() {
  echo "$1"
  for child in $(pgrep -P "$1" || true); do tree "$child"; done
}

stop_server() {
  pids=$(tree "$SERVER_PID")
  kill -TERM $pids 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
  i=0
  while curl -fs -o /dev/null "http://localhost:$PORT_UNDER_TEST/api/catalog"; do
    i=$((i + 1))
    if [ "$i" -ge 30 ]; then kill -KILL $pids 2>/dev/null || true; fi
    if [ "$i" -ge 40 ]; then echo "FAIL: the server would not stop" >&2; exit 1; fi
    sleep 1
  done
  SERVER_PID=""
}

# Waits for /api/catalog, checks its version and the server's memory. $1 is the boot's log.
check_boot() {
  i=0
  until curl -fs -o "$1.catalog.json" "http://localhost:$PORT_UNDER_TEST/api/catalog"; do
    i=$((i + 1))
    if ! kill -0 "$SERVER_PID" 2>/dev/null || [ "$i" -ge "$READY_SECONDS" ]; then
      echo "FAIL: /api/catalog never answered (the start command exited, or ${READY_SECONDS}s passed)"
      cat "$1"
      exit 1
    fi
    sleep 1
  done
  served=$(node -e 'const c = require(process.argv[1]); process.stdout.write(`${c.version} ${Object.keys(c.defs).length}`)' "$1.catalog.json")
  echo "ready after ${i}s; serving catalog $served"
  if [ "${served%% *}" != "$CATALOG_VERSION" ]; then
    echo "FAIL: serving catalog ${served%% *}, render.yaml says $CATALOG_VERSION"
    exit 1
  fi
  rss_kb=0
  for pid in $(tree "$SERVER_PID"); do
    kb=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ')
    if [ -n "$kb" ] && [ "$kb" -gt "$rss_kb" ]; then rss_kb=$kb; fi
  done
  echo "largest node process: $((rss_kb / 1024)) MB (limit $MEMORY_LIMIT_MB)"
  if [ $((rss_kb / 1024)) -gt "$MEMORY_LIMIT_MB" ]; then
    echo "FAIL: the server needs more memory than Render's free instance has"
    exit 1
  fi
}

LOG_DIR=${RUNNER_TEMP:-/tmp}
echo "--- first boot: every migration, the reseed, then the server ---"
start_server "$LOG_DIR/deploy-boot-1.log"
check_boot "$LOG_DIR/deploy-boot-1.log"
grep -E "^(migrate|seed-catalog):" "$LOG_DIR/deploy-boot-1.log" || true
stop_server

echo "--- second boot: nothing to migrate, the same catalog ---"
start_server "$LOG_DIR/deploy-boot-2.log"
check_boot "$LOG_DIR/deploy-boot-2.log"
if ! grep -q "migrate: nothing to do" "$LOG_DIR/deploy-boot-2.log"; then
  echo "FAIL: the second boot migrated again"
  cat "$LOG_DIR/deploy-boot-2.log"
  exit 1
fi
stop_server

echo "--- the deploy rehearsal passed ---"
