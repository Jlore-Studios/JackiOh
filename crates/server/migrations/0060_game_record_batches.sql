-- ============================================================================
-- Migration 0060: a development run's file filed as totals only (issue #1976,
-- R1976000)
-- ============================================================================
-- Serves SPEC §9.11, R378 and R1976000.
--
-- `jackioh-server stats-import --totals-only <file>` folds a development run's
-- records into the card statistics' totals (record_totals, card_totals,
-- R1044000) and writes no row of public.game_records for them: no detail, and
-- no id. The run's file is its own copy of every game (R1044001's "the files
-- the run wrote"). What keeps a file from being counted twice is this table:
-- one row per file, `batch` the SHA-256 of its contents, claimed in the
-- transaction that folds it, so the same file imported again finds its batch
-- held and folds nothing, and an import that fails leaves no batch behind.
-- `records` is how many of the file's records were folded (a record whose id
-- game_records already holds, imported a record at a time, is left out).
--
-- TRUST BOUNDARY: as the totals (0041). RLS on, no policy and no grant: only
-- service_role (BYPASSRLS) reads or writes it. It holds no account and no game.
--
-- Apply order: ... -> 0059 -> 0060 (this file).
-- Safe to re-apply: every statement is guarded.
-- ============================================================================

create table if not exists public.game_record_batches (
  batch       text        primary key,
  records     integer     not null,
  imported_at timestamptz not null default now(),
  constraint game_record_batches_batch_check   check (batch ~ '^[0-9a-f]{64}$'),
  constraint game_record_batches_records_check check (records >= 0)
);

comment on table public.game_record_batches is
  $$R1976000: each development run's file stats-import --totals-only folded
  into the totals, by the SHA-256 of its contents, with how many of its
  records were folded. A file whose batch is here folds nothing again.
  Server-only.$$;

alter table public.game_record_batches enable row level security;

-- TRUST BOUNDARY: RLS on, no policy, no grant, as the totals (0041).
revoke all on public.game_record_batches from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on game_record_batches: no SELECT, INSERT, UPDATE or
--                DELETE, and no policy.
-- service_role:  BYPASSRLS covers stats-import's claim and every read.
-- ============================================================================
