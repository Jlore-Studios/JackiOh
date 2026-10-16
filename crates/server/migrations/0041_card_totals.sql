-- ============================================================================
-- Migration 0041: the card statistics' totals (issue #1044) -- records folded
-- into counts, and a record's detail kept a week
-- ============================================================================
-- Serves SPEC §9.11, R1044000 and R1044001.
--
-- R1044000: every record written is folded, in the transaction that writes
-- it, into three tables of counts, which every card figure is read from:
--   - record_totals: records by patch, source, mode and both seats' pilots;
--   - card_totals: per patch, source, mode, the seat's pilot and card, R377's
--     breakdowns (`stats`), the drill-down's games by the turn the card was
--     played on (`turns`) and with each card that shared its deck (`co`, each
--     with where the pair was first met in id order, for the drill-down's tie
--     order);
--   - decklist_totals: each patch's live Best-of-1 human decklists, with their
--     games and wins and where each was first seen, for the ladder bots
--     (R1454, R750000).
-- The server reads and writes them (src/db/pg.rs, stats_totals_add): it loads
-- the rows a fold touches, adds the record's counts in Rust (the engine's
-- `StatsTotals`, the one counting path), and writes them back, holding a
-- transaction-level advisory lock per patch and source so two folds into the
-- same rows never interleave.
--
-- game_records gains `folded`, true once a record's counts are in the totals.
-- The server writes every new record folded; a record written before this
-- migration is not, and `jackioh-server stats-fold` (or the server's hourly
-- retention loop, which runs the same fold first) folds it, once.
--
-- R1044001: a folded record's detail (`record`) is emptied
-- `GAME_RECORD_RAW_RETENTION_DAYS` (7) after it was written, by
-- app.purge_expired_rows' fourth cutoff. Its row, the id, source, mode, patch
-- and time, stays, so a second write of the same game is still refused and is
-- never folded twice. A record not folded is never emptied
-- (game_records_detail_folded_check).
--
-- TRUST BOUNDARY: unchanged. The three tables hold aggregates of what
-- game_records holds (no account, no single game), and like it they have RLS
-- on, no policy and no grant: only service_role (BYPASSRLS) reads or writes
-- them.
--
-- Apply order: ... -> 0040 -> 0041 (this file).
-- Safe to re-apply: every statement is guarded.
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. game_records: folded, and a detail that may be emptied once folded.
-- ----------------------------------------------------------------------------

alter table public.game_records add column if not exists folded boolean not null default false;
alter table public.game_records alter column record drop not null;

alter table public.game_records drop constraint if exists game_records_copies_check;
alter table public.game_records add constraint game_records_copies_check check (
  record is null
  or (record ->> 'id' = id
      and record ->> 'source' = source
      and record ->> 'mode' = mode
      and record ->> 'patch' = patch)
);

alter table public.game_records drop constraint if exists game_records_detail_folded_check;
alter table public.game_records add constraint game_records_detail_folded_check
  check (record is not null or folded);

comment on column public.game_records.record is
  $$R376: the GameRecord itself (crates/engine/src/wire/stats.rs). R1044001:
  null once the retention purge has emptied it, GAME_RECORD_RAW_RETENTION_DAYS
  after recorded_at, which it does only to a folded record.$$;

comment on column public.game_records.folded is
  $$R1044000: whether the record's counts are in the totals (record_totals,
  card_totals, decklist_totals). The server writes a record and folds it in
  one transaction; one written before migration 0041 is folded once, by
  stats-fold or the retention loop.$$;

-- The purge's scan, and the fold's.
create index if not exists game_records_detail_idx
  on public.game_records (recorded_at) where record is not null;
create index if not exists game_records_unfolded_idx
  on public.game_records (id collate "C") where not folded;

-- ----------------------------------------------------------------------------
-- 2. The totals.
-- ----------------------------------------------------------------------------

create table if not exists public.record_totals (
  patch    text   not null,
  source   text   not null,
  mode     text   not null,
  p1_pilot text   not null,
  p2_pilot text   not null,
  records  bigint not null,
  primary key (patch, source, mode, p1_pilot, p2_pilot),
  constraint record_totals_source_check check (source in ('live', 'dev')),
  constraint record_totals_mode_check   check (mode in ('bo1', 'bo3', 'random')),
  constraint record_totals_pilot_check  check (p1_pilot in ('human', 'ai') and p2_pilot in ('human', 'ai')),
  constraint record_totals_records_check check (records > 0)
);

comment on table public.record_totals is
  $$R1044000: the records folded, by patch, source, mode and both seats'
  pilots. A card report's games and decks, and R654's gate (a patch's live
  games), are read from it. Server-only.$$;

create table if not exists public.card_totals (
  patch  text  not null,
  source text  not null,
  mode   text  not null,
  pilot  text  not null,
  card   text  not null,
  stats  jsonb not null,
  turns  jsonb not null default '{}'::jsonb,
  co     jsonb not null default '{}'::jsonb,
  primary key (patch, source, mode, pilot, card),
  constraint card_totals_source_check check (source in ('live', 'dev')),
  constraint card_totals_mode_check   check (mode in ('bo1', 'bo3', 'random')),
  constraint card_totals_pilot_check  check (pilot in ('human', 'ai'))
);

comment on table public.card_totals is
  $$R1044000: one card's counts over the seats of one patch, source, mode and
  pilot that held it. `stats` is R377's six breakdowns (a CardStats); `turns`
  maps a turn to [games, wins] of the games the card was played on it; `co`
  maps each other card that shared its deck to [games, wins, record, seat,
  index], the last three where the two were first met in id order.
  Server-only.$$;

create table if not exists public.decklist_totals (
  patch        text     not null,
  list         text     not null,
  deck         jsonb    not null,
  games        bigint   not null,
  wins         bigint   not null,
  first_record text     not null,
  first_seat   smallint not null,
  primary key (patch, list),
  constraint decklist_totals_seat_check check (first_seat in (0, 1))
);

comment on table public.decklist_totals is
  $$R1044000 (R1454, R750000): each distinct decklist people took into a
  patch's live Best-of-1 games: `list` its sorted ids joined with commas,
  `deck` the list as the first record held it, its games and wins, and where
  it was first seen (the record, and 0 for p1 or 1 for p2). Server-only.$$;

alter table public.record_totals   enable row level security;
alter table public.card_totals     enable row level security;
alter table public.decklist_totals enable row level security;

-- TRUST BOUNDARY: RLS on, no policy, no grant, as game_records (0014).
revoke all on public.record_totals   from public, anon, authenticated;
revoke all on public.card_totals     from public, anon, authenticated;
revoke all on public.decklist_totals from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- 3. The retention purge, with the game records' detail as a fourth cutoff.
-- ----------------------------------------------------------------------------
-- SECURITY INVOKER, as 0029's: the server calls it as service_role. 0029's
-- three-cutoff version stays for anything that still calls it, and empties no
-- game record.
create or replace function app.purge_expired_rows(
  p_code_attempts_before  timestamptz,
  p_match_actions_before  timestamptz,
  p_play_telemetry_before timestamptz,
  p_game_records_before   timestamptz
) returns table (code_attempts bigint, match_actions bigint, play_telemetry bigint, game_records bigint)
language plpgsql
set search_path = ''
as $$
declare
  v_attempts bigint;
  v_actions  bigint;
  v_telemetry bigint;
  v_records  bigint;
begin
  select p.code_attempts, p.match_actions, p.play_telemetry
    into v_attempts, v_actions, v_telemetry
    from app.purge_expired_rows(p_code_attempts_before, p_match_actions_before, p_play_telemetry_before) p;

  -- R1044001: a folded record's detail goes; its row, and so its id, stays.
  update public.game_records g
     set record = null
   where g.record is not null
     and g.folded
     and g.recorded_at < p_game_records_before;
  get diagnostics v_records = row_count;

  return query select v_attempts, v_actions, v_telemetry, v_records;
end;
$$;

comment on function app.purge_expired_rows(timestamptz, timestamptz, timestamptz, timestamptz) is
  'Migration 0041: 0029''s three cutoffs, then empties the detail of every '
  'folded game record written before the fourth, keeping its row (R1044001). '
  'Called by the server (src/api/retention.rs) with cutoffs from src/config.rs.';

revoke execute on function app.purge_expired_rows(timestamptz, timestamptz, timestamptz, timestamptz) from public;
grant execute on function app.purge_expired_rows(timestamptz, timestamptz, timestamptz, timestamptz) to postgres, service_role;

comment on function app.purge_expired_rows(timestamptz, timestamptz, timestamptz) is
  'Migration 0013, extended by 0029: deletes code_attempts made before the '
  'first cutoff, the action log of every match that ended before the second, '
  'and the play telemetry of every match that ended before the third (R1442). '
  'Superseded by the four-cutoff version (0041), which calls it.';

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on the three totals tables or game_records -- no
--                SELECT, INSERT, UPDATE or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the fold's reads and writes and the reports'
--                reads, and the EXECUTE grant above is the purge it calls.
-- ============================================================================
