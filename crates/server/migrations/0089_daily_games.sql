-- ============================================================================
-- Migration 0089: Daily games (SPEC §9.13, issue #3727) -- a 1v1 game played
-- at days per move, its invites, and how its players are told it is their move
-- ============================================================================
-- Serves SPEC §9.13. A daily game is an ordinary 1v1 match (public.matches,
-- its log in public.match_actions, its result in public.results) that no
-- actor holds in memory: the server folds it on every read and move, and
-- stores its clock beside it. Nothing of it sets profiles.current_match_id,
-- so a player may hold several at once and still queue for a live game.
-- This file:
--
--   1. Creates public.daily_games: one row per daily match, the accounts at
--      p1 and p2 as the game began, its mode ('bo1' or 'random') and time per
--      move (1, 3, 7 or 14 days, DAILY_TIME_PER_MOVE_DAYS), its clock (who
--      holds the move and until when, `jackioh_shared::host::daily`'s
--      DailyClock) and its board (the turn, both heroes' life and the result,
--      public, DailyBoard, so a list shows it without a fold), the earliest
--      deadline held (`next_deadline_at`, which the sweep reads), the log's
--      seq its clock and board are of (`seq`, which a move's write is
--      conditional on, so two requests reading one seq never both write),
--      and when it began and ended. A row with no `ended_at` is running.
--   2. Creates public.daily_invites: a challenge to one friend, or a seek any
--      other player may take, with the sender's frozen choice (a saved deck or
--      a deal, `choice`), where it stands, the match it made and when it
--      lapses. One open challenge per sender and friend, and one open seek
--      per sender, mode and time per move (two partial unique keys). The
--      match id is no foreign key: a claim names the match the same
--      transaction creates after it, as replay_shares names its match.
--   3. Creates public.daily_notify: one row per account, whether it wants an
--      email when a game waits on it, the token its unsubscribe link carries
--      (RFC 8058), and when it was last emailed and pushed.
--   4. Creates public.push_subscriptions: each browser's Web Push endpoint and
--      keys, one row per endpoint.
--   5. Re-creates app.live_matches() so it leaves a daily match out: the
--      reaper's ceiling (R112) and a restart's rebuild (§9.5) are a live
--      actor's, and a daily match has none.
--   6. Re-creates app.reap_stale_matches(), the reaper's SQL half, to leave
--      a daily match out too.
--
-- Every time is epoch milliseconds (bigint), as the server's clock and the
-- DailyClock's deadlines are. Every row goes with its account (0012's rule,
-- `on delete cascade`); a daily game goes with its match too.
--
-- TRUST BOUNDARY (SPEC §9.1, docs/architecture.md §3.2): a player reads the
-- daily games they hold a seat in and the invites they sent or received, and
-- writes none: the server (service_role, BYPASSRLS; src/db/pg.rs) is the only
-- writer of all four tables. An invite's `choice` is the sender's deck, hidden
-- from the other player as a co-op room's frozen choices are (0039), so it is
-- left out of the client's column grant. daily_notify (the unsubscribe token)
-- and push_subscriptions (the push keys) are the server's alone, as
-- match_checkpoints is (0042): RLS on, no policy, no grant.
--
-- Apply order: ... -> 0004 (matches, app.live_matches) -> ... -> 0038 (blocks)
-- -> ... -> 0087 -> 0088 -> 0089 (this file). This file only ADDS objects and
-- replaces the bodies of app.live_matches() (0004's) and app.reap_stale_matches()
-- (0022's). Safe to re-apply: create-if-not-exists,
-- drop-policy-if-exists then create, create-or-replace, comment-on, revoke and
-- grant.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. public.daily_games.
-- ----------------------------------------------------------------------------
create table if not exists public.daily_games (
  match_id         uuid primary key references public.matches(id) on delete cascade,
  p1_profile_id    uuid not null references public.profiles(id) on delete cascade,
  p2_profile_id    uuid not null references public.profiles(id) on delete cascade,
  mode             text not null,
  per_move_days    integer not null,
  clock            jsonb not null,
  board            jsonb not null,
  next_deadline_at bigint,
  seq              bigint not null default 0,
  created_at       bigint not null,
  ended_at         bigint,
  constraint daily_games_seq_check check (seq >= 0),
  constraint daily_games_mode_check check (mode in ('bo1', 'random')),
  constraint daily_games_per_move_days_check check (per_move_days in (1, 3, 7, 14)),
  constraint daily_games_seats_check check (p1_profile_id <> p2_profile_id),
  constraint daily_games_clock_check check (jsonb_typeof(clock) = 'object'),
  constraint daily_games_board_check check (jsonb_typeof(board) = 'object')
);

comment on table public.daily_games is
  $$SPEC §9.13, #3727: a daily game beside its match: the accounts at p1 and p2
  as it began, its mode and days per move, its clock (DailyClock) and its
  public board (DailyBoard), the earliest deadline held, the log's seq they
  are of, and when it began and ended (epoch ms). Running while ended_at is
  null. A player reads the games
  they hold a seat in; the server writes every row.$$;

-- The sweep: the running games whose earliest deadline has passed.
create index if not exists daily_games_due_idx
  on public.daily_games (next_deadline_at) where ended_at is null;
-- A player's list, each seat (and the account deletion's walk of each key, CHECK 19).
create index if not exists daily_games_p1_idx on public.daily_games (p1_profile_id, created_at);
create index if not exists daily_games_p2_idx on public.daily_games (p2_profile_id, created_at);


-- ----------------------------------------------------------------------------
-- 2. public.daily_invites.
-- ----------------------------------------------------------------------------
create table if not exists public.daily_invites (
  id            uuid primary key default gen_random_uuid(),
  kind          text not null,
  from_profile  uuid not null references public.profiles(id) on delete cascade,
  to_profile    uuid references public.profiles(id) on delete cascade,
  mode          text not null,
  per_move_days integer not null,
  choice        jsonb not null,
  state         text not null default 'open',
  match_id      uuid,
  created_at    bigint not null,
  expires_at    bigint not null,
  constraint daily_invites_kind_check check (kind in ('challenge', 'seek')),
  -- A challenge names its friend; a seek names nobody.
  constraint daily_invites_to_check check ((kind = 'challenge') = (to_profile is not null)),
  constraint daily_invites_self_check check (to_profile is null or to_profile <> from_profile),
  constraint daily_invites_mode_check check (mode in ('bo1', 'random')),
  constraint daily_invites_per_move_days_check check (per_move_days in (1, 3, 7, 14)),
  constraint daily_invites_choice_check check (jsonb_typeof(choice) = 'object'),
  constraint daily_invites_state_check
    check (state in ('open', 'accepted', 'declined', 'cancelled', 'expired', 'paired'))
);

comment on table public.daily_invites is
  $$SPEC §9.13, #3727: a daily challenge to a friend (to_profile set) or a seek
  any other player may take (to_profile null), with the sender's frozen choice,
  where it stands, the match it made and when it lapses (epoch ms). A player
  reads the invites they sent or received, never the choice; the server writes
  every row.$$;

comment on column public.daily_invites.match_id is
  $$The match an accepted challenge or a paired seek made. No foreign key: the
  claim names the match its own transaction creates next.$$;

-- One open challenge per sender and friend, and one open seek per sender, mode
-- and time per move, so two racing sends cannot both land. The server marks a
-- lapsed one expired before it adds the next.
create unique index if not exists daily_invites_open_challenge_key
  on public.daily_invites (from_profile, to_profile) where state = 'open' and kind = 'challenge';
create unique index if not exists daily_invites_open_seek_key
  on public.daily_invites (from_profile, mode, per_move_days) where state = 'open' and kind = 'seek';
-- An account's sent and received invites (and the account deletion's walk of each key, CHECK 19).
create index if not exists daily_invites_from_idx on public.daily_invites (from_profile, created_at);
create index if not exists daily_invites_to_idx
  on public.daily_invites (to_profile) where to_profile is not null;
-- The seek a new one pairs with, oldest first, and the sweep's lapsed invites.
create index if not exists daily_invites_open_seeks_idx
  on public.daily_invites (mode, per_move_days, created_at) where state = 'open' and kind = 'seek';
create index if not exists daily_invites_open_expiry_idx
  on public.daily_invites (expires_at) where state = 'open';


-- ----------------------------------------------------------------------------
-- 3. public.daily_notify.
-- ----------------------------------------------------------------------------
create table if not exists public.daily_notify (
  profile_id        uuid primary key references public.profiles(id) on delete cascade,
  email_on          boolean not null default true,
  unsubscribe_token uuid not null unique default gen_random_uuid(),
  last_email_at     bigint,
  last_push_at      bigint
);

comment on table public.daily_notify is
  $$SPEC §9.13, #3727: whether an account is emailed when a daily game waits on
  it, the token its unsubscribe link carries (RFC 8058), and when it was last
  emailed and pushed (epoch ms). The server's alone: RLS on, no policy, no
  grant.$$;


-- ----------------------------------------------------------------------------
-- 4. public.push_subscriptions.
-- ----------------------------------------------------------------------------
create table if not exists public.push_subscriptions (
  id         uuid primary key default gen_random_uuid(),
  profile_id uuid not null references public.profiles(id) on delete cascade,
  endpoint   text not null unique,
  p256dh     text not null,
  auth       text not null,
  created_at bigint not null
);

comment on table public.push_subscriptions is
  $$SPEC §9.13, #3727: a browser's Web Push subscription (RFC 8030, RFC 8291):
  its endpoint, one row each, and its keys, for the account that made it. The
  server's alone: RLS on, no policy, no grant.$$;

-- An account's devices, oldest first (and the account deletion's walk of the key, CHECK 19).
create index if not exists push_subscriptions_profile_idx
  on public.push_subscriptions (profile_id, created_at);


-- ----------------------------------------------------------------------------
-- RLS and privileges.
-- ----------------------------------------------------------------------------
alter table public.daily_games        enable row level security;
alter table public.daily_invites      enable row level security;
alter table public.daily_notify       enable row level security;
alter table public.push_subscriptions enable row level security;

drop policy if exists daily_games_select_own on public.daily_games;
create policy daily_games_select_own
  on public.daily_games
  for select
  to authenticated
  using (p1_profile_id = app.current_profile_id() or p2_profile_id = app.current_profile_id());

drop policy if exists daily_invites_select_own on public.daily_invites;
create policy daily_invites_select_own
  on public.daily_invites
  for select
  to authenticated
  using (from_profile = app.current_profile_id() or to_profile = app.current_profile_id());

-- daily_notify, push_subscriptions: no policy.

revoke all on public.daily_games        from public, anon, authenticated;
revoke all on public.daily_invites      from public, anon, authenticated;
revoke all on public.daily_notify       from public, anon, authenticated;
revoke all on public.push_subscriptions from public, anon, authenticated;

grant select on public.daily_games to authenticated;
-- Every column but the sender's frozen choice.
grant select (id, kind, from_profile, to_profile, mode, per_move_days, state, match_id, created_at, expires_at)
  on public.daily_invites to authenticated;


-- ----------------------------------------------------------------------------
-- 5. app.live_matches(), 0004's, leaving a daily match out.
-- ----------------------------------------------------------------------------
create or replace function app.live_matches() returns setof public.matches
language sql
security definer
stable
set search_path = ''
as $$
  select m.* from public.matches m
   where m.status = 'live'
     and not exists (select 1 from public.daily_games d where d.match_id = m.id);
$$;

comment on function app.live_matches() is
  $$SPEC §9.5 / ARCHITECTURE-CCG.md §2.2: what a restarting server folds
  for crash recovery -- every `live` match, to fold (seed, log) for each
  via public.match_actions -- and what the reaper reads (R112). A daily game
  (0089, SPEC §9.13) is left out: no actor holds it, and its clock is
  days per move, not the match ceiling.$$;

revoke all on function app.live_matches() from public;
grant execute on function app.live_matches() to service_role;

-- ----------------------------------------------------------------------------
-- 6. app.reap_stale_matches(), 0022's, leaving a daily match out too.
-- ----------------------------------------------------------------------------
-- The SQL half of the reaper, which no server path calls today (src/api/results.rs's
-- reap_stuck_matches reads app.live_matches() above); were it run, it would draw
-- every unranked daily game whose row's ceiling had passed.
create or replace function app.reap_stale_matches() returns int
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_match record;
  v_count int := 0;
begin
  for v_match in
    select m.id, m.p1_profile_id, m.p2_profile_id
    from public.matches m
    where m.status = 'live' and m.ceiling_at < now()
      -- See app.end_match's guard: these two are the server-side reaper's.
      and not m.ranked
      and not exists (select 1 from public.series s where s.next_match_id = m.id)
      -- 0089: a daily game has no ceiling; its clock is days per move (SPEC §9.13).
      and not exists (select 1 from public.daily_games d where d.match_id = m.id)
    for update skip locked
  loop
    -- SPEC §11 R112: turns is not tracked on public.matches (the live turn
    -- counter lives only in the match actor's in-memory GameState), so a
    -- ceiling draw the reaper resolves records turns = 0 rather than the
    -- true count. A server-driven ceilingReached action (the normal
    -- path) always has the real turn count from GameState and should be
    -- preferred whenever the actor is still alive; this reaper is the
    -- last-resort path for when it is not.
    perform app.end_match(
      v_match.id,
      null,
      'match-ceiling',
      0,
      (select rating from public.profiles where id = v_match.p1_profile_id),
      (select rating from public.profiles where id = v_match.p2_profile_id)
    );

    v_count := v_count + 1;
  end loop;

  return v_count;
end;
$$;

revoke all on function app.reap_stale_matches() from public;
grant execute on function app.reap_stale_matches() to service_role;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing on the four tables.
-- authenticated: SELECT on public.daily_games, the rows it holds a seat in;
--                SELECT on public.daily_invites' columns but `choice`, the
--                rows it sent or received; nothing on public.daily_notify or
--                public.push_subscriptions. No INSERT, UPDATE or DELETE.
-- service_role:  BYPASSRLS covers the server's writes and reads, and
--                EXECUTE on app.live_matches() and app.reap_stale_matches()
--                as before.
-- ============================================================================
