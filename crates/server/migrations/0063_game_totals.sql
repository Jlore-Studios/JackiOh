-- ============================================================================
-- Migration 0063: each patch's whole games, kept past the week's purge (issue
-- #1994, R1994000)
-- ============================================================================
-- Serves SPEC §9.11, R1044000 and R1994000.
--
-- The card statistics' totals (0041) keep the records folded and each card's
-- counts for good, but nothing of a game as a whole: R1044001 empties a folded
-- record's detail after GAME_RECORD_RAW_RETENTION_DAYS, and with it who went
-- first and won, how the game ended, how many turns it ran and R1256000's
-- counts. This table keeps them, under record_totals' key (patch, source,
-- mode, both seats' pilots): `total` is the engine's GameTotal (games, draws,
-- first-seat wins, games per ending, games per turn count, and R1256000's
-- sums with the count of records that carry them), written whole by
-- `stats_totals_add` under 0041's advisory locks, as card_totals' rows are.
-- A record folded before this migration counts in record_totals and adds
-- nothing here: there is no back-fill, and GET /api/stats/games says how many
-- of a patch's games its figures count.
--
-- TRUST BOUNDARY: as the totals (0041). RLS on, no policy and no grant: only
-- service_role (BYPASSRLS) reads or writes it. It holds no account and no game.
--
-- Apply order: ... -> 0062 -> 0063 (this file).
-- Safe to re-apply: every statement is guarded.
-- ============================================================================

create table if not exists public.game_totals (
  patch    text  not null,
  source   text  not null,
  mode     text  not null,
  p1_pilot text  not null,
  p2_pilot text  not null,
  total    jsonb not null,
  primary key (patch, source, mode, p1_pilot, p2_pilot),
  constraint game_totals_source_check check (source in ('live', 'dev')),
  constraint game_totals_mode_check   check (mode in ('bo1', 'bo3', 'random')),
  constraint game_totals_pilot_check  check (p1_pilot in ('human', 'ai') and p2_pilot in ('human', 'ai')),
  constraint game_totals_games_check  check (coalesce(total ->> 'games' ~ '^[1-9][0-9]*$', false))
);

comment on table public.game_totals is
  $$R1994000: the whole games of the records folded, by patch, source, mode
  and both seats' pilots (record_totals' key). `total` is a GameTotal: games,
  draws, firstWins, reasons (games per ending), turns (games per turn count),
  and R1256000's sums over the `counted` records that carry them. A record
  folded before migration 0063 is in record_totals and not here.
  Server-only.$$;

alter table public.game_totals enable row level security;

-- TRUST BOUNDARY: RLS on, no policy, no grant, as the totals (0041).
revoke all on public.game_totals from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on game_totals: no SELECT, INSERT, UPDATE or
--                DELETE, and no policy.
-- service_role:  BYPASSRLS covers every fold and every read.
-- ============================================================================
