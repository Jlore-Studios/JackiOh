#!/bin/sh
# Apply every migration (0001-0029) to a throwaway Postgres and assert the
# invariants of SPEC §9.1, §9.4 and §9.5 against a real database.
#
#   sh crates/server/tests/sql/run.sh        # as CI's db job runs it
#
# Needs Docker only. This is NOT part of `cargo test`, and it never will be: the
# cargo suite under crates/server runs against an in-memory fake, and every
# assertion in these files is a property of a real Postgres that a fake cannot
# have — row-level security under the `authenticated` role, the append-only
# deny_row_mutation triggers, SECURITY DEFINER boundaries, and the L4 unique
# index on (profile_id, card_id) that BUILD M6-T3 names as a raw SQL test. Run
# both: `cargo test` for the server logic, this script for the schema.
#
# Exit status is the whole point — this is a gate, not a report. It exits 0 only
# when every migration applied without noise and every check passed. It exits 1
# when psql stops on an error (`\set ON_ERROR_STOP on` plus a `raise exception`
# from a failed assertion), and also when any check merely *prints* FAIL or
# UNEXPECTED: `raise notice` does not stop psql, so the grep below stays as a
# second net even though all three files now raise.
#
# 00_supabase_stub.sql stands in for the Supabase-managed pieces the migrations
# reference (the `anon`/`authenticated`/`service_role` roles, `auth.users` and
# `auth.uid()`), so a plain postgres image is enough. It is never applied to a
# real project — Supabase provides all of it. 0005 and 0006 need nothing more
# from it: 0005 grants service_role what the stub already grants (a GRANT that
# is already held is a no-op), and 0006 only replaces a function body.
#
# The migrations go in three batches with a seed between each two, because two
# of them are DATA migrations that act on rows that exist when they run, so
# there have to be rows for them to find. R254: 0007 turns every loadout into
# three decks and a trio. 03b seeds one — through 0003's own app.save_loadout,
# as a player of the old server saved it — after 0001-0006 and before
# 0007-0027, exactly the order a database that predates 0007 sees. 04 then
# checks what 0007 made of it. R1434: 0028 names every existing profile
# `Player#n` in order of sign-up. 14b signs up three profiles that predate it,
# with sign-up times out of insertion order, after 0007-0027 and before 0028.
# 15 then checks the numbers 0028 gave them and 03b's profile.
#
# The CONTAINER name is fixed so a run can be inspected afterwards; the script
# removes a previous container of that name before it starts, so never point it
# at a name another tool uses.
set -e

REPO=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
SQL="$REPO/crates/server/tests/sql"
MIGRATIONS="$REPO/crates/server/migrations"
CONTAINER=jackioh-pg-test
PSQL="docker exec $CONTAINER psql -U postgres -v ON_ERROR_STOP=1 -q"

failed=0

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" -e POSTGRES_PASSWORD=postgres postgres:16 >/dev/null
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

for f in "$SQL"/*.sql "$MIGRATIONS"/*.sql; do
  docker cp "$f" "$CONTAINER":/tmp/ >/dev/null
done

$PSQL -d jackioh -f /tmp/00_supabase_stub.sql >/dev/null

# One migration file, applied the way src/db/migrate.rs applies it: inside a
# transaction of its own, so a file lands whole or not at all. "Clean" means
# psql said nothing but the expected "does not exist, skipping" notices of a
# drop-if-exists on a first run.
apply_migration() {
  printf '%s: ' "$1"
  out=$($PSQL -d jackioh -1 -f "/tmp/$1.sql" 2>&1 |
    grep -v "does not exist, skipping" | grep -v "^$" || true)
  if [ -z "$out" ]; then
    echo "clean"
  else
    echo "PROBLEM"
    echo "$out"
    failed=1
  fi
}

echo "--- migrations 0001-0006 ---"
for f in 0001_profiles_and_invites 0002_collection 0003_loadouts 0004_matches \
         0005_service_role_reads_auth_users 0006_redeem_ip_lock; do
  apply_migration "$f"
done

echo "--- 03b: a loadout saved before 0007 (R254's input) ---"
if ! $PSQL -d jackioh -f /tmp/03b_legacy_loadout_seed.sql; then
  echo "!!! 03b_legacy_loadout_seed: the legacy loadout could not be seeded"
  failed=1
fi

echo "--- migrations 0007-0027 ---"
for f in 0007_decks_and_trios 0008_queue_modes 0009_series 0010_jlockeed_tag \
         0011_tutorial_progress 0012_account_deletion 0013_retention_purge 0014_game_records \
         0015_classic_sets_tags 0016_catalog_growth_grants 0017_last_boards 0018_player_settings \
         0019_hero_portraits 0020_plague_tag 0021_player_stats 0022_ranked_ladder 0023_rematch 0024_glitch_boards \
         0025_patch_retcon 0026_catalyst_prime_acclaimed_tags 0027_lean_newest; do
  apply_migration "$f"
done

echo "--- 14b: profiles that predate usernames (R1434's input) ---"
if ! $PSQL -d jackioh -f /tmp/14b_profiles_before_usernames.sql; then
  echo "!!! 14b_profiles_before_usernames: the profiles could not be seeded"
  failed=1
fi

echo "--- migrations 0028-0029 ---"
apply_migration 0028_usernames
apply_migration 0029_meditative_set

# A migration directory with a file this list does not name is a migration no
# check ever ran against. Refuse it rather than pass without it.
for f in "$MIGRATIONS"/*.sql; do
  name=$(basename "$f" .sql)
  case " 0001_profiles_and_invites 0002_collection 0003_loadouts 0004_matches 0005_service_role_reads_auth_users 0006_redeem_ip_lock 0007_decks_and_trios 0008_queue_modes 0009_series 0010_jlockeed_tag 0011_tutorial_progress 0012_account_deletion 0013_retention_purge 0014_game_records 0015_classic_sets_tags 0016_catalog_growth_grants 0017_last_boards 0018_player_settings 0019_hero_portraits 0020_plague_tag 0021_player_stats 0022_ranked_ladder 0023_rematch 0024_glitch_boards 0025_patch_retcon 0026_catalyst_prime_acclaimed_tags 0027_lean_newest 0028_usernames 0029_meditative_set " in
    *" $name "*) ;;
    *) echo "!!! migration $name is not applied by this script; add it above"; failed=1 ;;
  esac
done

for f in 01_schema_invariants 02_rls_as_client 03_match_lifecycle 04_decks_and_series \
         05_tutorial_progress 06_account_deletion 07_retention_purge 08_game_records 09_catalog_growth \
         10_last_boards 11_player_settings 12_ranked 13_glitch 14_patch_retcon 15_usernames; do
  echo "--- $f ---"
  status=0
  out=$(docker exec "$CONTAINER" psql -U postgres -q -v ON_ERROR_STOP=1 \
    -d jackioh -f "/tmp/$f.sql" 2>&1) || status=$?
  printf '%s\n' "$out"
  if [ "$status" -ne 0 ]; then
    echo "!!! $f: psql exited $status — a check raised, and everything after it was skipped"
    failed=1
  fi
  # A `raise notice 'FAIL ...'` leaves psql's exit status at 0, so the text would be
  # the only signal. All five files raise instead, which is why the exit-status
  # check above is the primary gate; this grep is the backstop for a check that ever
  # regresses to a notice.
  if printf '%s\n' "$out" | grep -E 'FAIL|UNEXPECTED' >/dev/null 2>&1; then
    echo "!!! $f: a check reported FAIL/UNEXPECTED (see the lines above)"
    failed=1
  fi
done

if [ "$failed" -ne 0 ]; then
  echo "--- FAILED (docker rm -f $CONTAINER to clean up) ---"
  exit 1
fi

echo "--- done (docker rm -f $CONTAINER to clean up) ---"
