#!/bin/sh
# Rehearse a Render deploy of the server before it happens (`pnpm test:deploy`, CI's db job).
#
#   pnpm test:deploy        # or: sh crates/server/tests/deploy/rehearse.sh
#
# Render builds the Docker image render.yaml names (`runtime: docker`, its dockerfilePath and
# dockerContext) and runs the image's own command, `jackioh-server release`: migrate, then seed the
# catalog, then serve. It keeps the previous deploy serving if that fails. The 0013 outage was a
# migration that a superuser could apply and Supabase's migrating role could not: every other suite
# here migrates as the superuser `postgres`, so none could see it. This one reproduces what Render
# meets:
#
#   1. A database owned by `migrator`, a login role that is NOT a superuser, as Supabase's
#      `postgres` role is not; the Supabase-managed pieces (tests/db/bootstrap.sql: roles,
#      auth.users, auth.uid()) are installed by the superuser, as Supabase installs them, and
#      granted to it.
#   2. The image, built from render.yaml's dockerfilePath and dockerContext exactly as Render builds
#      it, run with render.yaml's own non-secret env values (NODE_ENV, TRUSTED_PROXY_HOPS,
#      CATALOG_VERSION), read from the file so the rehearsal cannot drift from the deploy. The
#      secrets are placeholders: the server checks their shape at boot, and nothing here calls out.
#      render.yaml must carry no buildCommand and no startCommand: the image is the whole deploy.
#   3. The readiness probe Render uses, GET /api/catalog: it must answer within READY_SECONDS with
#      render.yaml's catalog version and, in its x-deployed-commit header, the commit Render hands
#      the service in RENDER_GIT_COMMIT (deploy-watch.yml compares that header with each push); the
#      container runs under the free instance's INSTANCE_MEMORY_MB and must use less than
#      MEMORY_LIMIT_MB of it (headroom); and a second boot must migrate nothing.
#   4. CATALOG_VERSION is the server's to check now (docs/v0.3.0/SURFACE.md §11.3): the binary
#      compiles its catalog version in from crates/cards/patches/patches.json and refuses to boot
#      when the environment's disagrees. So a last boot with a stale value, as one left in Render's
#      dashboard would be, must exit non-zero without ever serving, and must leave the database
#      stamped with render.yaml's version, never the stale one.
#
# Needs Docker, or an existing superuser connection via REHEARSAL_PGHOST / REHEARSAL_PGPORT
# (a socket directory or a host) and REHEARSAL_PGPASSWORD; it creates and drops its own database.
# The image is built and run with Docker either way.
set -eu

REPO=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
DB=jackioh_deploy_rehearsal
PORT_UNDER_TEST=18787
READY_SECONDS=120
# Render's free instance, and the most of it the server may use (the rest is headroom).
INSTANCE_MEMORY_MB=512
MEMORY_LIMIT_MB=450
CONTAINER=jackioh-pg-deploy
SERVER=jackioh-server-rehearsal
IMAGE=jackioh-server:rehearsal
NETWORK=jackioh-deploy-rehearsal
# A catalog version nothing ships: what a dashboard that was never updated would still hold.
STALE_CATALOG_VERSION=v0.0.0-stale-dashboard-value
# What Render sets RENDER_GIT_COMMIT to is a full git SHA; this one is made up.
REHEARSAL_COMMIT=0123456789abcdef0123456789abcdef01234567

PGHOST_=${REHEARSAL_PGHOST:-}
PGPORT_=${REHEARSAL_PGPORT:-5432}
PGPASSWORD_=${REHEARSAL_PGPASSWORD:-postgres}
SERVER_UP=""

cleanup() {
  if [ -n "$SERVER_UP" ]; then docker rm -f "$SERVER" >/dev/null 2>&1 || true; fi
  if [ -z "${REHEARSAL_PGHOST:-}" ]; then docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; fi
  docker network rm "$NETWORK" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# render.yaml, read rather than restated: the runtime, the image's Dockerfile and build context,
# and each `- key:` whose next line is a literal `value:` (the `sync: false` secrets have none, and
# get placeholders below).
yaml_field() { sed -n "s/^ *$1: *//p" "$REPO/render.yaml" | head -n 1 | tr -d '"'; }
yaml_value() {
  awk -v key="$1" '
    $1 == "-" && $2 == "key:" { current = $3; next }
    current == key && $1 == "value:" { v = $2; gsub(/"/, "", v); print v; exit }
  ' "$REPO/render.yaml"
}
RUNTIME=$(yaml_field runtime)
DOCKERFILE=$(yaml_field dockerfilePath)
CONTEXT=$(yaml_field dockerContext)
CATALOG_VERSION=$(yaml_value CATALOG_VERSION)
if [ "$RUNTIME" != "docker" ] || [ -z "$DOCKERFILE" ] || [ -z "$CONTEXT" ] || [ -z "$CATALOG_VERSION" ]; then
  echo "FAIL: could not read runtime: docker, dockerfilePath, dockerContext and CATALOG_VERSION from render.yaml" >&2
  exit 1
fi
if grep -Eq '^ *(buildCommand|startCommand):' "$REPO/render.yaml"; then
  echo "FAIL: render.yaml still has a buildCommand or startCommand: the image's own command is the start" >&2
  exit 1
fi
echo "image: $DOCKERFILE (context $CONTEXT)"
echo "catalog version: $CATALOG_VERSION"

echo "--- the image, built as Render builds it ---"
docker build -f "$REPO/$DOCKERFILE" -t "$IMAGE" "$REPO/$CONTEXT"

docker network rm "$NETWORK" >/dev/null 2>&1 || true
if [ -z "$PGHOST_" ]; then
  docker network create "$NETWORK" >/dev/null
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker run -d --name "$CONTAINER" --network "$NETWORK" -e POSTGRES_PASSWORD="$PGPASSWORD_" \
    -p 127.0.0.1:55433:5432 postgres:16 >/dev/null
  PGHOST_=127.0.0.1
  PGPORT_=55433
  # The server's container reaches Postgres by its container name on the shared network.
  DATABASE_URL="postgresql://migrator:migrator@$CONTAINER:5432/$DB"
  NETWORK_ARGS="--network $NETWORK -p 127.0.0.1:$PORT_UNDER_TEST:$PORT_UNDER_TEST"
else
  # An existing Postgres on this host: the server's container shares the host's network (and, for
  # a socket directory, the directory itself).
  case "$PGHOST_" in
    /*)
      DATABASE_URL="postgresql://migrator:migrator@localhost:$PGPORT_/$DB?host=$PGHOST_"
      NETWORK_ARGS="--network host -v $PGHOST_:$PGHOST_"
      ;;
    *)
      DATABASE_URL="postgresql://migrator:migrator@$PGHOST_:$PGPORT_/$DB"
      NETWORK_ARGS="--network host"
      ;;
  esac
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
admin -d "$DB" -f "$REPO/crates/server/tests/db/bootstrap.sql" >/dev/null 2>&1
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

# $1 is the boot's log, $2 the CATALOG_VERSION it is handed. The image's own command runs: no
# command is given here, as render.yaml gives none.
start_server() {
  docker rm -f "$SERVER" >/dev/null 2>&1 || true
  # shellcheck disable=SC2086 # NETWORK_ARGS is several words on purpose.
  docker run -d --name "$SERVER" $NETWORK_ARGS \
    --memory "${INSTANCE_MEMORY_MB}m" \
    -e DATABASE_URL="$DATABASE_URL" \
    -e CATALOG_VERSION="$2" \
    -e NODE_ENV="$(yaml_value NODE_ENV)" \
    -e TRUSTED_PROXY_HOPS="$(yaml_value TRUSTED_PROXY_HOPS)" \
    -e PORT="$PORT_UNDER_TEST" \
    -e RENDER_GIT_COMMIT="$REHEARSAL_COMMIT" \
    -e SUPABASE_URL=https://rehearsal.supabase.co \
    -e SUPABASE_SECRET_KEY=rehearsal-placeholder \
    -e CODE_PEPPER=rehearsal-placeholder-pepper-of-a-length-the-server-accepts-0123456789 \
    -e PUBLIC_ORIGINS=https://jackioh.vercel.app \
    "$IMAGE" >/dev/null
  SERVER_UP=1
  SERVER_LOG=$1
}

running() { [ "$(docker inspect -f '{{.State.Running}}' "$SERVER" 2>/dev/null)" = "true" ]; }

save_log() { docker logs "$SERVER" > "$SERVER_LOG" 2>&1 || true; }

stop_server() {
  docker stop -t 30 "$SERVER" >/dev/null 2>&1 || true
  save_log
  docker rm -f "$SERVER" >/dev/null 2>&1 || true
  i=0
  while curl -fs -o /dev/null "http://localhost:$PORT_UNDER_TEST/api/catalog"; do
    i=$((i + 1))
    if [ "$i" -ge 40 ]; then echo "FAIL: the server would not stop" >&2; exit 1; fi
    sleep 1
  done
  SERVER_UP=""
}

# The container's memory in whole MB, from `docker stats` ("12.3MiB / 512MiB").
memory_mb() {
  docker stats --no-stream --format '{{.MemUsage}}' "$SERVER" | awk '{
    v = $1; unit = $1
    sub(/[A-Za-z]+$/, "", v); sub(/^[0-9.]+/, "", unit)
    f = 1 / (1024 * 1024)
    if (unit == "KiB" || unit == "kB") f = 1 / 1024
    else if (unit == "MiB" || unit == "MB") f = 1
    else if (unit == "GiB" || unit == "GB") f = 1024
    printf "%d\n", v * f
  }'
}

# Waits for /api/catalog, checks its version, its commit header and the server's memory. $1 is the
# boot's log.
check_boot() {
  i=0
  until curl -fs -D "$1.headers" -o "$1.catalog.json" "http://localhost:$PORT_UNDER_TEST/api/catalog"; do
    i=$((i + 1))
    if ! running || [ "$i" -ge "$READY_SECONDS" ]; then
      echo "FAIL: /api/catalog never answered (the server exited, or ${READY_SECONDS}s passed)"
      save_log
      cat "$1"
      exit 1
    fi
    sleep 1
  done
  served=$(grep -o '"version":"[^"]*"' "$1.catalog.json" | head -n 1 | sed 's/^"version":"\(.*\)"$/\1/')
  echo "ready after ${i}s; serving catalog $served"
  if [ "$served" != "$CATALOG_VERSION" ]; then
    echo "FAIL: serving catalog $served, render.yaml says $CATALOG_VERSION"
    exit 1
  fi
  reported=$(tr -d '\r' < "$1.headers" | awk -F': *' 'tolower($1) == "x-deployed-commit" { print $2; exit }')
  echo "reporting commit ${reported:-(none)}"
  if [ "$reported" != "$REHEARSAL_COMMIT" ]; then
    echo "FAIL: x-deployed-commit is ${reported:-missing}, RENDER_GIT_COMMIT was $REHEARSAL_COMMIT"
    exit 1
  fi
  used=$(memory_mb)
  echo "server container: ${used} MB (limit $MEMORY_LIMIT_MB of the instance's $INSTANCE_MEMORY_MB)"
  if [ "$used" -gt "$MEMORY_LIMIT_MB" ]; then
    echo "FAIL: the server needs more memory than Render's free instance has"
    exit 1
  fi
}

# The catalog version the database is stamped with (`app.settings.catalog_version`, jsonb text).
stamped_version() {
  admin -d "$DB" -tAc "select value #>> '{}' from app.settings where key = 'catalog_version'"
}

LOG_DIR=${RUNNER_TEMP:-/tmp}
echo "--- first boot: every migration, the reseed, then the server ---"
start_server "$LOG_DIR/deploy-boot-1.log" "$CATALOG_VERSION"
check_boot "$LOG_DIR/deploy-boot-1.log"
save_log
grep -E "^(migrate|seed-catalog):" "$LOG_DIR/deploy-boot-1.log" || true
if ! grep -q "^seed-catalog: wrote .* at catalog version $CATALOG_VERSION\$" "$LOG_DIR/deploy-boot-1.log"; then
  echo "FAIL: the reseed did not stamp catalog $CATALOG_VERSION"
  cat "$LOG_DIR/deploy-boot-1.log"
  exit 1
fi
stop_server

echo "--- second boot: nothing to migrate, the same catalog ---"
start_server "$LOG_DIR/deploy-boot-2.log" "$CATALOG_VERSION"
check_boot "$LOG_DIR/deploy-boot-2.log"
save_log
if ! grep -q "migrate: nothing to do" "$LOG_DIR/deploy-boot-2.log"; then
  echo "FAIL: the second boot migrated again"
  cat "$LOG_DIR/deploy-boot-2.log"
  exit 1
fi
stop_server

echo "--- a stale CATALOG_VERSION: the boot is refused, the database keeps render.yaml's ---"
start_server "$LOG_DIR/deploy-boot-stale.log" "$STALE_CATALOG_VERSION"
i=0
while running; do
  if curl -fs -o /dev/null "http://localhost:$PORT_UNDER_TEST/api/catalog"; then
    save_log
    echo "FAIL: the server served with CATALOG_VERSION=$STALE_CATALOG_VERSION; it must refuse to boot"
    cat "$LOG_DIR/deploy-boot-stale.log"
    exit 1
  fi
  i=$((i + 1))
  if [ "$i" -ge "$READY_SECONDS" ]; then
    save_log
    echo "FAIL: the server neither served nor exited within ${READY_SECONDS}s on a stale CATALOG_VERSION"
    cat "$LOG_DIR/deploy-boot-stale.log"
    exit 1
  fi
  sleep 1
done
code=$(docker inspect -f '{{.State.ExitCode}}' "$SERVER")
save_log
if [ "$code" = "0" ]; then
  echo "FAIL: a boot with CATALOG_VERSION=$STALE_CATALOG_VERSION exited 0; it must fail so Render keeps the previous deploy"
  cat "$LOG_DIR/deploy-boot-stale.log"
  exit 1
fi
if ! grep -q "$STALE_CATALOG_VERSION" "$LOG_DIR/deploy-boot-stale.log"; then
  echo "FAIL: the refusal does not name the stale CATALOG_VERSION"
  cat "$LOG_DIR/deploy-boot-stale.log"
  exit 1
fi
echo "refused, exit $code: $(grep "$STALE_CATALOG_VERSION" "$LOG_DIR/deploy-boot-stale.log" | head -n 1)"
stamped=$(stamped_version)
if [ "$stamped" != "$CATALOG_VERSION" ]; then
  echo "FAIL: the database is stamped $stamped after the refused boot, render.yaml says $CATALOG_VERSION"
  exit 1
fi
docker rm -f "$SERVER" >/dev/null 2>&1 || true
SERVER_UP=""

echo "--- the deploy rehearsal passed ---"
