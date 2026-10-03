-- ============================================================================
-- Migration 0014: game records for the card statistics
-- ============================================================================
-- Serves SPEC.md §9.11 and SPEC §11 R376-R378: every finished game leaves a
-- record of what each seat's cards did -- the decklists, the opening hands,
-- the cards drawn and the cards played, who went first and who won -- filed
-- under its match type, the patch of the cards it was played with (R375) and
-- the pilot of each seat. Card win rates are read off these records by
-- `pnpm --filter @jackioh/server stats:cards`.
--
-- public.game_records holds the records. A live record is written by the
-- server when a match ends, after its result (src/api/game-records.ts), under
-- the match's id; its mode is read off what made the match (its series, its
-- room's room_mode or its tickets' mode), so public.matches is unchanged. A
-- record of an internal AI development run (`source = 'dev'`) is loaded by
-- `stats:import` and is never counted in a live figure unless the query asks
-- for it (R378).
--
-- Nothing on a match's path reads or writes this table: the server writes a
-- record after the result has committed and swallows a failure, so a server
-- deployed before this migration is applied loses records, never a match.
--
-- A record names no account: the pilots are "human" or "ai", nothing more, so
-- deleting an account (0012) leaves the records as they are. The match's own
-- move-by-move log is still purged after MATCH_ACTION_RETENTION_DAYS (0013);
-- the record is written while the log is there, and outlives it.
--
-- SPEC §9.1's trust model: no client reads or writes this table. Only the
-- server, as service_role, does.
--
-- Apply order: 0001 -> ... -> 0013 -> 0014 (this file).
-- This file only ADDS objects. Safe to re-apply: create-if-not-exists.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- public.game_records -- one row per recorded game (R376).
-- ----------------------------------------------------------------------------
-- `record` is the whole GameRecord of packages/shared/src/stats.ts. `source`,
-- `mode` and `patch` repeat three of its fields as columns so a query can
-- filter on them with an index, and the check below holds the copies equal.
create table if not exists public.game_records (
  id          text primary key,
  source      text not null,
  mode        text not null,
  patch       text not null,
  record      jsonb not null,
  recorded_at timestamptz not null default now(),
  constraint game_records_id_check     check (length(id) > 0),
  constraint game_records_source_check check (source in ('live', 'dev')),
  constraint game_records_mode_check   check (mode in ('bo1', 'bo3', 'random')),
  constraint game_records_patch_check  check (length(patch) > 0),
  -- R378: a development record's id begins 'dev:', and a live one (a match id) never does, so
  -- neither kind can take the other's place under the primary key.
  constraint game_records_dev_id_check check ((source = 'dev') = (id like 'dev:%')),
  constraint game_records_copies_check check (
    record ->> 'id' = id
    and record ->> 'source' = source
    and record ->> 'mode' = mode
    and record ->> 'patch' = patch
  )
);

comment on table public.game_records is
  $$SPEC §9.11 / R376: one finished game's record for the card statistics --
  its match type, patch, each seat's pilot, who went first, each seat's
  decklist, opening hand, cards drawn and cards played, and the result. A
  live record (source 'live') is keyed by its match id and written by the
  server once the match has its result; a development record (source 'dev')
  is an internal AI run's, loaded by stats:import, and no live figure counts
  it unless asked (R378). Names no account. Server-only: no client role reads
  or writes it.$$;

comment on column public.game_records.id is
  $$R376: a live record's match id; a development record's
  dev:<patch>:<series>:<n>, so the same seeds run for another patch are
  other records. One record per id: a second write of the same game is
  refused.$$;

comment on column public.game_records.source is
  $$R378: 'live' for a match the server ran, 'dev' for an internal AI
  development run.$$;

comment on column public.game_records.mode is
  $$R257: the match type the record is filed under (record.mode).$$;

comment on column public.game_records.patch is
  $$R375, R376: the newest patch of the build that played the game
  (packages/cards/patches/patches.json), or the patch a development run was
  told it tests (record.patch).$$;

comment on column public.game_records.record is
  $$R376: the GameRecord itself (packages/shared/src/stats.ts).$$;

create index if not exists game_records_filter_idx
  on public.game_records (source, patch, mode);

alter table public.game_records enable row level security;

-- TRUST BOUNDARY: RLS on, and no policy of any kind. RLS default-deny plus
-- zero grants makes this table unreadable and unwritable from the Data API;
-- only service_role (BYPASSRLS) touches it.
revoke all on public.game_records from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing -- no SELECT, INSERT, UPDATE or DELETE, and no
--                policy. A record holds both hands and both decklists.
-- service_role:  BYPASSRLS covers reads and writes directly: the insert at
--                the end of a match or by stats:import, and the filtered read
--                stats:cards makes.
-- ============================================================================
