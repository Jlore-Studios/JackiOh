#!/bin/sh
# Rehearse a deploy of the server before it happens (CI's db job).
#
#   sh crates/server/tests/deploy/rehearse.sh
#
# `wrangler deploy` builds the Docker image wrangler.jsonc's `containers` names (its `image` and
# `image_build_context`), and the Worker runs it in a Cloudflare container with the image's own
# command, `jackioh-server release`, on every boot: migrate, then seed the catalog, then serve. A
# boot that fails leaves the site with no server, since nothing keeps an older one serving. The 0013
# outage was a migration that a superuser could apply and Supabase's migrating role could not: every
# other suite here migrates as the superuser `postgres`, so none could see it. This one reproduces
# what the container meets:
#
#   1. A database owned by `migrator`, a login role that is NOT a superuser, as Supabase's
#      `postgres` role is not; the Supabase-managed pieces (tests/db/bootstrap.sql: roles,
#      auth.users, auth.uid()) are installed by the superuser, as Supabase installs them, and
#      granted to it; and what it creates in `public` is service_role's by default, as what
#      Supabase's `postgres` creates is.
#   2. The image, built from wrangler.jsonc's `image` and `image_build_context` as `wrangler deploy`
#      builds it, run with no command, as the container is, and with what the Worker hands the
#      container (apps/edge/src/routing.ts's `containerEnv`): NODE_ENV=production, production's
#      TRUSTED_PROXY_HOPS and PUBLIC_ORIGINS, read from wrangler.jsonc's `vars` so the rehearsal
#      cannot drift from the deploy, and a DEPLOYED_COMMIT, but no CATALOG_VERSION. The secrets are
#      placeholders: the server checks their shape at boot, and nothing here calls out.
#   3. The readiness probe the Worker's daily cron requests (READY_PATH, /readyz, which asks
#      Postgres): it must answer within READY_SECONDS, followed by /api/catalog's version, the one
#      crates/server/.env.example's CATALOG_VERSION line names (`patches ship` bumps it with
#      patches.json, R388), and, in its x-deployed-commit header, the commit handed in
#      DEPLOYED_COMMIT (the Worker compares /healthz's with its own, and the deploy workflows wait
#      for /api/catalog's); the container runs under the `basic` instance's INSTANCE_MEMORY_MB and
#      must use less than MEMORY_LIMIT_MB of it (headroom); and a second boot must migrate nothing.
#      Each boot is stopped with SIGTERM (docker stop; tini forwards it) and must exit 0.
#   4. CATALOG_VERSION cannot stop a deploy (docs/v0.3.0/SURFACE.md §11.3, #488): the binary
#      compiles its catalog version in from crates/cards/patches/patches.json and serves it whatever
#      the environment says. So a last boot with a stale value, as a host's setting left from an
#      older deploy would be, must still serve .env.example's version, warn naming the stale value,
#      and leave the database stamped with that version, never the stale one.
#
# CI splits the rehearsal so every job finishes under its five-minute budget (#1556): one job
# per Dockerfile stage, then the image, then one boot shard per database shape.
# REHEARSAL_STAGE=deps stops after building the Dockerfile's `deps` stage (the image's
# dependencies, `cargo chef cook`), REHEARSAL_STAGE=game after its `game` stage (the engine, the
# cards and the AI, compiled on them) and REHEARSAL_STAGE=crates after its `crates` stage (the
# server's library, compiled on those), with the cache flags below: no database, no image, no boot.
# REHEARSAL_DEPS_ONLY=1 is REHEARSAL_STAGE=deps. CI builds each stage in a job of its own before the
# rehearsal (#653, #1223, #2013: db-deploy-deps, db-deploy-game, then db-deploy-crates), so a change
# to a manifest or Cargo.lock, which recompiles every dependency, is paid in the first, the engine,
# the cards and the AI in the second, the server's library in the third, and the rehearsal's own
# build compiles only the server's binary and its thin-LTO link.
# REHEARSAL_STAGE=image builds the whole image exactly as the rehearsal below does (`-t` the image,
# the same cache flags, `--load` under buildx) and, when REHEARSAL_SAVE_IMAGE is set, saves it with
# `docker save` to that path, then stops: no database, no container, no boot (CI's db-deploy-image,
# which uploads the tarball as an artifact). REHEARSAL_LOAD_IMAGE loads such a tarball with
# `docker load` instead of building, then runs the boots below unchanged; REHEARSAL_BOOT selects
# which: `migrate` runs the first two boots on one database (the reseed, then the nothing-to-migrate
# check, which only holds on the migrated database), `stale` runs only the stale-CATALOG_VERSION boot
# on a fresh database, and an empty REHEARSAL_BOOT (or `all`) runs all three in order on one database,
# which is what the daily full rehearsal and local runs keep doing (CI's db-deploy-boot matrix runs
# one REHEARSAL_BOOT value per job on the downloaded tarball).
# REHEARSAL_STAGE cannot be combined with REHEARSAL_BOOT, REHEARSAL_SAVE_IMAGE cannot be combined
# with REHEARSAL_LOAD_IMAGE, and neither image flag applies to the deps, game or crates stages
# (those builds tag no image).
#
# Needs Docker, or an existing superuser connection via REHEARSAL_PGHOST / REHEARSAL_PGPORT
# (a socket directory or a host) and REHEARSAL_PGPASSWORD; it creates and drops its own database.
# The image is built and run with Docker either way.
set -eu

REPO=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
DB=jackioh_deploy_rehearsal
PORT_UNDER_TEST=18787
READY_SECONDS=120
# The path the Worker's daily cron requests (apps/edge/src/index.ts's `scheduled`).
READY_PATH=/readyz
# The `basic` container instance's memory (wrangler.jsonc's `instance_type`), and the most of it the
# server may use once booted (the rest is headroom for live matches and the ladder bots' search).
INSTANCE_MEMORY_MB=1024
MEMORY_LIMIT_MB=450
CONTAINER=jackioh-pg-deploy
SERVER=jackioh-server-rehearsal
IMAGE=jackioh-server:rehearsal
NETWORK=jackioh-deploy-rehearsal
# A catalog version nothing ships: what a host's setting that was never updated would still hold.
STALE_CATALOG_VERSION=v0.0.0-stale-dashboard-value
# What the deploy hands the Worker as DEPLOYED_COMMIT is a full git SHA; this one is made up.
REHEARSAL_COMMIT=0123456789abcdef0123456789abcdef01234567

PGHOST_=${REHEARSAL_PGHOST:-}
PGPORT_=${REHEARSAL_PGPORT:-5432}
PGPASSWORD_=${REHEARSAL_PGPASSWORD:-postgres}
SERVER_UP=""
# The one Dockerfile stage to build alone (REHEARSAL_STAGE, or `deps` for REHEARSAL_DEPS_ONLY), or
# empty for the whole rehearsal. REHEARSAL_STAGE=image builds the whole image and, with
# REHEARSAL_SAVE_IMAGE, saves it, instead of booting it. REHEARSAL_BOOT runs a subset of the boots
# below on a loaded (REHEARSAL_LOAD_IMAGE) or freshly built image: `migrate` the first two,
# `stale` only the stale-CATALOG_VERSION boot, `all` (or empty) all three in order.
STAGE=${REHEARSAL_STAGE:-}
if [ -z "$STAGE" ] && [ -n "${REHEARSAL_DEPS_ONLY:-}" ]; then STAGE=deps; fi
BOOT=${REHEARSAL_BOOT:-}
SAVE_IMAGE=${REHEARSAL_SAVE_IMAGE:-}
LOAD_IMAGE=${REHEARSAL_LOAD_IMAGE:-}
case "$STAGE" in
  "" | deps | game | crates | image) ;;
  *) echo "FAIL: REHEARSAL_STAGE is deps, game, crates or image, not $STAGE" >&2; exit 1 ;;
esac
case "$BOOT" in
  "" | all | migrate | stale) ;;
  *) echo "FAIL: REHEARSAL_BOOT is all, migrate or stale, not $BOOT" >&2; exit 1 ;;
esac
if [ -n "$STAGE" ] && [ -n "$BOOT" ]; then
  echo "FAIL: REHEARSAL_STAGE=$STAGE and REHEARSAL_BOOT=$BOOT cannot be combined: a boot shard loads the image instead of building a stage" >&2
  exit 1
fi
if [ -n "$SAVE_IMAGE" ] && [ -n "$LOAD_IMAGE" ]; then
  echo "FAIL: REHEARSAL_SAVE_IMAGE and REHEARSAL_LOAD_IMAGE cannot both be set" >&2
  exit 1
fi
case "$STAGE" in
  deps | game | crates)
    if [ -n "$SAVE_IMAGE" ] || [ -n "$LOAD_IMAGE" ]; then
      echo "FAIL: REHEARSAL_SAVE_IMAGE and REHEARSAL_LOAD_IMAGE need the built image, not the $STAGE stage" >&2
      exit 1
    fi
    ;;
  image)
    if [ -n "$LOAD_IMAGE" ]; then
      echo "FAIL: REHEARSAL_LOAD_IMAGE skips the image build, so REHEARSAL_STAGE=image cannot be set with it" >&2
      exit 1
    fi
    ;;
esac

cleanup() {
  if [ -n "$SERVER_UP" ]; then docker rm -f "$SERVER" >/dev/null 2>&1 || true; fi
  if [ -z "${REHEARSAL_PGHOST:-}" ]; then docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; fi
  docker network rm "$NETWORK" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# wrangler.jsonc, read rather than restated: the first `"key": "value"` line of each key, which is
# production's (the top level comes before `env.staging`), for the image's Dockerfile and build
# context and the non-secret settings the Worker hands the server (the secrets get placeholders
# below). The version main should serve is crates/server/.env.example's, which R388's test holds to
# the newest entry of crates/cards/patches/patches.json.
jsonc_field() { sed -n "s/^ *\"$1\": *\"\([^\"]*\)\".*/\1/p" "$REPO/wrangler.jsonc" | head -n 1; }
DOCKERFILE=$(jsonc_field image)
DOCKERFILE=${DOCKERFILE#./}
CONTEXT=$(jsonc_field image_build_context)
TRUSTED_PROXY_HOPS=$(jsonc_field TRUSTED_PROXY_HOPS)
PUBLIC_ORIGINS=$(jsonc_field PUBLIC_ORIGINS)
CATALOG_VERSION=$(sed -n 's/^CATALOG_VERSION=//p' "$REPO/crates/server/.env.example" | head -n 1)
if [ -z "$DOCKERFILE" ] || [ -z "$CONTEXT" ] || [ -z "$TRUSTED_PROXY_HOPS" ] || [ -z "$PUBLIC_ORIGINS" ]; then
  echo "FAIL: could not read image, image_build_context, TRUSTED_PROXY_HOPS and PUBLIC_ORIGINS from wrangler.jsonc" >&2
  exit 1
fi
if [ -z "$CATALOG_VERSION" ]; then
  echo "FAIL: could not read CATALOG_VERSION from crates/server/.env.example" >&2
  exit 1
fi
echo "image: $DOCKERFILE (context $CONTEXT)"
echo "catalog version: $CATALOG_VERSION"
echo "readiness: $READY_PATH"

# REHEARSAL_LOAD_IMAGE (above) skips the build: the tarball a STAGE=image job saved already
# carries the image's tag, so the boots below need no change.
if [ -n "$LOAD_IMAGE" ]; then
  echo "--- the image, loaded from a STAGE=image job's tarball ---"
  docker load -i "$LOAD_IMAGE"
else
  # STAGE (above) builds that one stage alone, left in the build cache and written to
  # REHEARSAL_CACHE_TO, with no image to tag or load; STAGE=image builds the whole image exactly
  # as the rehearsal below does.
  if [ -n "$STAGE" ] && [ "$STAGE" != "image" ]; then
    echo "--- the Dockerfile's $STAGE stage alone, built as wrangler builds it ---"
    set -- --target "$STAGE"
  else
    echo "--- the image, built as wrangler builds it ---"
    set -- -t "$IMAGE"
  fi
  # REHEARSAL_CACHE_FROM (and REHEARSAL_CACHE_TO) give buildx a layer cache to read (and write), in its
  # `--cache-from`/`--cache-to` syntax: CI's GitHub Actions cache, so an unchanged image is not compiled
  # again on every run. Same Dockerfile, same context, same image either way. Only a cache written
  # with `mode=max` keeps the build stages' layers, the Dockerfile's dependency layer (`cargo chef
  # cook`) among them, which a change to the sources alone reuses; `mode=min` keeps only the image's
  # own layers, so any change under crates/ compiles every dependency again.
  # REHEARSAL_CACHE_FROM may name several caches, separated by spaces, each read with a
  # `--cache-from` of its own (#1787). A cache write replaces that cache's whole index with the
  # records of the stages its build built, so while the deps, crates and image jobs shared one
  # scope, the deps job's write dropped the crates stage's records and the binary's, and every run
  # compiled the workspace's crates and the binary again though nothing under crates/ had changed.
  # CI gives each stage's job a scope of its own to write, and reads the earlier stages' beside it.
  # A cache spec has no spaces or glob characters, so splitting on spaces is safe.
  if [ -n "${REHEARSAL_CACHE_FROM:-}" ]; then
    for cache in $REHEARSAL_CACHE_FROM; do set -- "$@" --cache-from "$cache"; done
    if [ -n "${REHEARSAL_CACHE_TO:-}" ]; then set -- "$@" --cache-to "$REHEARSAL_CACHE_TO"; fi
    if [ -z "$STAGE" ] || [ "$STAGE" = "image" ]; then set -- "$@" --load; fi
    docker buildx build "$@" -f "$REPO/$DOCKERFILE" "$REPO/$CONTEXT"
  else
    docker build "$@" -f "$REPO/$DOCKERFILE" "$REPO/$CONTEXT"
  fi
  if [ "$STAGE" = "image" ]; then
    if [ -n "$SAVE_IMAGE" ]; then docker save "$IMAGE" -o "$SAVE_IMAGE"; fi
    echo "--- the image stage is built: the boot jobs load it and run one BOOT each ---"
    exit 0
  fi
  if [ -n "$STAGE" ]; then
    echo "--- the $STAGE stage is built and cached: the rehearsal proper builds the image on it ---"
    exit 0
  fi
  if [ -n "$SAVE_IMAGE" ]; then docker save "$IMAGE" -o "$SAVE_IMAGE"; fi
fi

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
# Supabase's default privileges for the role that migrates: service_role may use every table,
# sequence and function it creates in `public`. bootstrap.sql's own `alter default privileges`
# binds to the superuser that ran it, and the migrations here run as migrator, so without these
# the server, which runs every statement as service_role (src/db/pg.rs), could read none of the
# tables they make; the seat ledger's boot (R1523000) is the first such read before /readyz.
admin -d "$DB" \
  -c "alter default privileges for role migrator in schema public grant all on tables to service_role" \
  -c "alter default privileges for role migrator in schema public grant all on sequences to service_role" \
  -c "alter default privileges for role migrator in schema public grant all on functions to service_role"
if [ "$(admin -d "$DB" -tAc "select rolsuper from pg_roles where rolname = 'migrator'")" != "f" ]; then
  echo "FAIL: migrator must not be a superuser, or this rehearses nothing" >&2
  exit 1
fi

# $1 is the boot's log, $2 the CATALOG_VERSION it is handed, or empty for none, as the Worker hands
# none. The image's own command runs: no command is given here, as the container is given none.
start_server() {
  docker rm -f "$SERVER" >/dev/null 2>&1 || true
  CATALOG_ARGS=""
  if [ -n "$2" ]; then CATALOG_ARGS="-e CATALOG_VERSION=$2"; fi
  # shellcheck disable=SC2086 # NETWORK_ARGS and CATALOG_ARGS are several words on purpose.
  docker run -d --name "$SERVER" $NETWORK_ARGS $CATALOG_ARGS \
    --memory "${INSTANCE_MEMORY_MB}m" \
    -e DATABASE_URL="$DATABASE_URL" \
    -e NODE_ENV=production \
    -e TRUSTED_PROXY_HOPS="$TRUSTED_PROXY_HOPS" \
    -e PORT="$PORT_UNDER_TEST" \
    -e DEPLOYED_COMMIT="$REHEARSAL_COMMIT" \
    -e SUPABASE_URL=https://rehearsal.supabase.co \
    -e SUPABASE_SECRET_KEY=rehearsal-placeholder \
    -e CODE_PEPPER=rehearsal-placeholder-pepper-of-a-length-the-server-accepts-0123456789 \
    -e PUBLIC_ORIGINS="$PUBLIC_ORIGINS" \
    "$IMAGE" >/dev/null
  SERVER_UP=1
  SERVER_LOG=$1
}

running() { [ "$(docker inspect -f '{{.State.Running}}' "$SERVER" 2>/dev/null)" = "true" ]; }

save_log() { docker logs "$SERVER" > "$SERVER_LOG" 2>&1 || true; }

stop_server() {
  docker stop -t 30 "$SERVER" >/dev/null 2>&1 || true
  save_log
  exit_code=$(docker inspect -f '{{.State.ExitCode}}' "$SERVER" 2>/dev/null || echo unknown)
  docker rm -f "$SERVER" >/dev/null 2>&1 || true
  i=0
  while curl -fs -o /dev/null "http://localhost:$PORT_UNDER_TEST/api/catalog"; do
    i=$((i + 1))
    if [ "$i" -ge 40 ]; then echo "FAIL: the server would not stop" >&2; exit 1; fi
    sleep 1
  done
  SERVER_UP=""
  if [ "$exit_code" != "0" ]; then echo "FAIL: the server exited $exit_code after SIGTERM, not 0: its drain did not finish"; cat "$SERVER_LOG"; exit 1; fi
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

# Waits for the readiness probe, then /api/catalog, and checks its version, its commit header and
# the server's memory. $1 is the boot's log.
check_boot() {
  i=0
  until curl -fs -o /dev/null "http://localhost:$PORT_UNDER_TEST$READY_PATH"; do
    i=$((i + 1))
    if ! running || [ "$i" -ge "$READY_SECONDS" ]; then
      echo "FAIL: $READY_PATH never answered (the server exited, or ${READY_SECONDS}s passed)"
      save_log
      cat "$1"
      exit 1
    fi
    sleep 1
  done
  if ! curl -fs -D "$1.headers" -o "$1.catalog.json" "http://localhost:$PORT_UNDER_TEST/api/catalog"; then echo "FAIL: $READY_PATH answered but /api/catalog did not"; save_log; cat "$1"; exit 1; fi
  served=$(grep -o '"version":"[^"]*"' "$1.catalog.json" | head -n 1 | sed 's/^"version":"\(.*\)"$/\1/')
  echo "ready after ${i}s; serving catalog $served"
  if [ "$served" != "$CATALOG_VERSION" ]; then
    echo "FAIL: serving catalog $served, crates/server/.env.example says $CATALOG_VERSION"
    exit 1
  fi
  reported=$(tr -d '\r' < "$1.headers" | awk -F': *' 'tolower($1) == "x-deployed-commit" { print $2; exit }')
  echo "reporting commit ${reported:-(none)}"
  if [ "$reported" != "$REHEARSAL_COMMIT" ]; then
    echo "FAIL: x-deployed-commit is ${reported:-missing}, DEPLOYED_COMMIT was $REHEARSAL_COMMIT"
    exit 1
  fi
  used=$(memory_mb)
  echo "server container: ${used} MB (limit $MEMORY_LIMIT_MB of the instance's $INSTANCE_MEMORY_MB)"
  if [ "$used" -gt "$MEMORY_LIMIT_MB" ]; then
    echo "FAIL: the server needs more than $MEMORY_LIMIT_MB MB of the container's $INSTANCE_MEMORY_MB"
    exit 1
  fi
}

# The catalog version the database is stamped with (`app.settings.catalog_version`, jsonb text).
stamped_version() {
  admin -d "$DB" -tAc "select value #>> '{}' from app.settings where key = 'catalog_version'"
}

LOG_DIR=${RUNNER_TEMP:-/tmp}
# REHEARSAL_BOOT (above) runs a subset: `migrate` the first two boots on one database (the
# nothing-to-migrate check only holds on the migrated database), `stale` only the stale boot on a
# fresh database, `all` (or empty) all three in order on one database.
case "$BOOT" in
  "" | all | migrate)
echo "--- first boot: every migration, the reseed, then the server ---"
start_server "$LOG_DIR/deploy-boot-1.log" ""
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
start_server "$LOG_DIR/deploy-boot-2.log" ""
check_boot "$LOG_DIR/deploy-boot-2.log"
save_log
if ! grep -q "migrate: nothing to do" "$LOG_DIR/deploy-boot-2.log"; then
  echo "FAIL: the second boot migrated again"
  cat "$LOG_DIR/deploy-boot-2.log"
  exit 1
fi
stop_server
    ;;
esac
case "$BOOT" in
  "" | all | stale)
echo "--- a stale CATALOG_VERSION: the boot serves the compiled-in catalog and says so ---"
start_server "$LOG_DIR/deploy-boot-stale.log" "$STALE_CATALOG_VERSION"
check_boot "$LOG_DIR/deploy-boot-stale.log"
save_log
if ! grep -q "$STALE_CATALOG_VERSION" "$LOG_DIR/deploy-boot-stale.log"; then
  echo "FAIL: the boot does not warn about the stale CATALOG_VERSION"
  cat "$LOG_DIR/deploy-boot-stale.log"
  exit 1
fi
echo "warned: $(grep "$STALE_CATALOG_VERSION" "$LOG_DIR/deploy-boot-stale.log" | head -n 1)"
stamped=$(stamped_version)
if [ "$stamped" != "$CATALOG_VERSION" ]; then
  echo "FAIL: the database is stamped $stamped after the stale boot, crates/server/.env.example says $CATALOG_VERSION"
  exit 1
fi
stop_server
    ;;
esac

echo "--- the deploy rehearsal passed ---"
