-- ============================================================================
-- Migration 0029: play telemetry (SPEC §9.11, R1442; issue #637)
-- ============================================================================
-- How people play an online match, kept so the ladder bots (#636) can copy a
-- human's pace, emotes and giving up. Three tables, each keyed by the match and
-- the seat, and none of them naming a profile:
--
--   * public.action_timings -- one row per move a person made: what kind of
--     action, how many actions were legal, the turn, how long the seat took
--     (from the push of the view that made it the seat's move to the move),
--     the clock it had left, whether it was the seat's first move of the turn,
--     and the ladder tier the player held when the match began.
--   * public.emote_events -- one row per emote relayed before the result: the
--     emote, the turn, the last game event before it and how long after it the
--     emote came, and how long after the opponent's last emote, when it answers
--     one.
--   * public.match_signals -- one row per seat: the turn a seat conceded on and
--     how far behind it stood, the draws it offered and whether it accepted
--     one, whether it offered a rematch and whether that made one, and how
--     many of its clocks ran out.
--
-- `seat` is the seat the account BEGAN the match in (R677), the one
-- public.matches.p1_profile_id / p2_profile_id names, so a Glitch's swap never
-- moves a row to the other account. `pilot` says who chose the moves (R376's
-- `human` or `ai`); `jackioh-server timing-fit` leaves the `ai` rows out.
--
-- The server writes the three tables in one transaction right after a match's
-- result commits (crates/server/src/actor/telemetry.rs), never action by
-- action, and a failed write never costs the result. A row whose key exists is
-- left as it stands, so a write repeated after a restart changes nothing.
-- `jackioh-server timing-backfill` folds the action logs still held
-- (public.match_actions) into action_timings rows the same way.
--
-- Retention (section 3): app.purge_expired_rows gains a third cutoff and
-- deletes the telemetry of every match that ended before it,
-- PLAY_TELEMETRY_RETENTION_DAYS (365) after the match ended. That is a new
-- signature beside 0013's two-cutoff one, which is kept: a deploy migrates
-- before it serves, and the server before it keeps serving (and purging, hourly)
-- on this schema until the new one passes its health check
-- (docs/architecture.md), as 0028 kept `display_name`. The server from this one
-- on calls only the three-cutoff function; a later migration drops the other
-- once no deployed server calls it.
--
-- Account deletion (section 2): deleting a profile deletes the rows of every
-- seat it held, before 0012's foreign keys empty its seat on public.matches.
-- The opponent's rows stay. A match deleted outright (R679's void) takes its
-- telemetry with it (`on delete cascade`).
--
-- TRUST BOUNDARY: no client role may read or write any of it, as for
-- public.game_records (0014): RLS on, NO policy, no grant to anon or
-- authenticated. The server (service_role, BYPASSRLS) is the only reader and
-- writer.
--
-- Apply order: ... -> 0004 (`public.matches`) -> 0012 -> 0013 -> ... -> 0028
-- -> 0029.
-- Safe to re-apply: create-if-not-exists, comment-on, revoke, grant,
-- create-or-replace of both functions and drop-and-create of the trigger are
-- all idempotent.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The tables.
-- ----------------------------------------------------------------------------
create table if not exists public.action_timings (
  match_id      uuid not null references public.matches(id) on delete cascade,
  seq           bigint not null,
  seat          text not null,
  action_kind   text not null,
  legal_count   integer not null,
  turn          integer not null,
  think_ms      bigint not null,
  clock_left_ms bigint,
  first_in_turn boolean not null,
  rank_bucket   text,
  pilot         text not null,
  primary key (match_id, seq),
  constraint action_timings_seat_check check (seat in ('p1', 'p2')),
  constraint action_timings_pilot_check check (pilot in ('human', 'ai')),
  constraint action_timings_think_ms_check check (think_ms >= 0)
);

comment on table public.action_timings is
  $$R1442: how long each move a seat made took, from the push of the view
  that made it the seat's move to the move, with what it was and when. One row
  per logged action (match_actions.seq); server actions and setAutoEndTurn
  make none. No client access: RLS on, no policy, no grant.$$;

alter table public.action_timings enable row level security;
revoke all on public.action_timings from public, anon, authenticated;

create table if not exists public.emote_events (
  match_id             uuid not null references public.matches(id) on delete cascade,
  ordinal              integer not null,
  seat                 text not null,
  emote_id             text not null,
  turn                 integer not null,
  trigger_event        text,
  ms_since_trigger     bigint,
  reply_to_opponent_ms bigint,
  pilot                text not null,
  primary key (match_id, ordinal),
  constraint emote_events_seat_check check (seat in ('p1', 'p2')),
  constraint emote_events_pilot_check check (pilot in ('human', 'ai'))
);

comment on table public.emote_events is
  $$R1442: each emote relayed before the match's result, in order, with the
  last game event before it. Emotes never enter the action log (R643). No
  client access: RLS on, no policy, no grant.$$;

alter table public.emote_events enable row level security;
revoke all on public.emote_events from public, anon, authenticated;

create table if not exists public.match_signals (
  match_id             uuid not null references public.matches(id) on delete cascade,
  seat                 text not null,
  conceded_turn        integer,
  concede_eval_deficit double precision,
  draw_offers          integer not null default 0,
  draw_accepted        boolean not null default false,
  rematch_offered      boolean not null default false,
  rematch_accepted     boolean not null default false,
  timeouts             integer not null default 0,
  pilot                text not null,
  primary key (match_id, seat),
  constraint match_signals_seat_check check (seat in ('p1', 'p2')),
  constraint match_signals_pilot_check check (pilot in ('human', 'ai'))
);

comment on table public.match_signals is
  $$R1442: how each seat's match ended for it: a concede's turn and how far
  behind the seat stood, draw offers, rematch offers and timeouts. No client
  access: RLS on, no policy, no grant.$$;

alter table public.match_signals enable row level security;
revoke all on public.match_signals from public, anon, authenticated;


-- ----------------------------------------------------------------------------
-- 2. Account deletion: a profile's seats' rows go with it.
-- ----------------------------------------------------------------------------
-- BEFORE DELETE, like 0012's app.release_open_matches: the seat columns of
-- public.matches still name the profile here, and 0012's foreign keys empty
-- them once the row is gone.
-- SECURITY INVOKER: it runs as whoever deletes the profile (the server's
-- service_role, or the table owner inside auth.users' cascade).
create or replace function app.forget_play_telemetry()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  delete from public.action_timings a
   using public.matches m
   where a.match_id = m.id
     and ((a.seat = 'p1' and m.p1_profile_id = old.id)
       or (a.seat = 'p2' and m.p2_profile_id = old.id));
  delete from public.emote_events e
   using public.matches m
   where e.match_id = m.id
     and ((e.seat = 'p1' and m.p1_profile_id = old.id)
       or (e.seat = 'p2' and m.p2_profile_id = old.id));
  delete from public.match_signals s
   using public.matches m
   where s.match_id = m.id
     and ((s.seat = 'p1' and m.p1_profile_id = old.id)
       or (s.seat = 'p2' and m.p2_profile_id = old.id));
  return old;
end;
$$;

comment on function app.forget_play_telemetry() is
  'Migration 0029: deletes the play telemetry of every seat a profile being '
  'deleted held, before its foreign keys run (R1442).';

revoke execute on function app.forget_play_telemetry() from public;

drop trigger if exists profiles_forget_play_telemetry on public.profiles;
create trigger profiles_forget_play_telemetry
  before delete on public.profiles
  for each row execute function app.forget_play_telemetry();


-- ----------------------------------------------------------------------------
-- 3. The retention purge, with the telemetry's cutoff added to 0013's two.
-- ----------------------------------------------------------------------------
-- SECURITY INVOKER: the server calls it as service_role, which may delete from
-- every table it names. Returns how many rows of each kind went, for the log
-- line; play_telemetry counts the three tables' rows together.
create or replace function app.purge_expired_rows(
  p_code_attempts_before  timestamptz,
  p_match_actions_before  timestamptz,
  p_play_telemetry_before timestamptz
) returns table (code_attempts bigint, match_actions bigint, play_telemetry bigint)
language plpgsql
set search_path = ''
as $$
declare
  v_attempts  bigint;
  v_actions   bigint;
  v_timings   bigint;
  v_emotes    bigint;
  v_signals   bigint;
begin
  -- Transaction-local, and cleared below; a failed delete aborts the transaction.
  perform set_config('jackioh.retention_purge', 'on', true);

  delete from public.code_attempts a
   where a.at < p_code_attempts_before;
  get diagnostics v_attempts = row_count;

  delete from public.match_actions a
   using public.matches m
   where a.match_id = m.id
     and m.status = 'over'
     and m.ended_at < p_match_actions_before;
  get diagnostics v_actions = row_count;

  perform set_config('jackioh.retention_purge', 'off', true);

  -- R1442: the telemetry tables are not append-only, so no guard is involved.
  delete from public.action_timings a
   using public.matches m
   where a.match_id = m.id
     and m.status = 'over'
     and m.ended_at < p_play_telemetry_before;
  get diagnostics v_timings = row_count;

  delete from public.emote_events e
   using public.matches m
   where e.match_id = m.id
     and m.status = 'over'
     and m.ended_at < p_play_telemetry_before;
  get diagnostics v_emotes = row_count;

  delete from public.match_signals s
   using public.matches m
   where s.match_id = m.id
     and m.status = 'over'
     and m.ended_at < p_play_telemetry_before;
  get diagnostics v_signals = row_count;

  return query select v_attempts, v_actions, v_timings + v_emotes + v_signals;
end;
$$;

comment on function app.purge_expired_rows(timestamptz, timestamptz, timestamptz) is
  'Migration 0013, extended by 0029: deletes code_attempts made before the '
  'first cutoff, the action log of every match that ended before the second, '
  'and the play telemetry of every match that ended before the third (R1442). '
  'Results are kept. Called by the server (src/api/retention.rs) with cutoffs '
  'from src/config.rs.';

revoke execute on function app.purge_expired_rows(timestamptz, timestamptz, timestamptz) from public;
grant execute on function app.purge_expired_rows(timestamptz, timestamptz, timestamptz) to postgres, service_role;

comment on function app.purge_expired_rows(timestamptz, timestamptz) is
  'Migration 0013: deletes code_attempts made before the first cutoff and the '
  'action log of every match that ended before the second. Superseded by the '
  'three-cutoff purge of migration 0029, which the server calls; kept for the '
  'server a deploy replaces.';

-- anon, authenticated: nothing on any of the three tables, and no execute on
-- either function.
-- service_role: BYPASSRLS covers the three tables directly; the grant above is
-- the purge's.
