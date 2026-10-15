-- ============================================================================
-- Migration 0037: Duos co-op (issue #936, #946) — rooms, matches and their log
-- ============================================================================
-- Serves the fullsend SPEC's S6 (Play together, Duos co-op): two people play
-- one team against an AI team, two 1v1 duels side by side, through
-- crates/server/src/duos/. Three tables:
--
--   * public.duos_rooms -- a co-op room by its 6-character code: the host,
--     the guest once one joins, the AI team's difficulty, each person's
--     frozen choice (a saved deck, or a deal from the match seed), the match
--     the join started, and when an unjoined room expires (DUOS_ROOM_TTL_MS,
--     src/config.rs). The server's sweeper deletes a room past its expiry.
--   * public.duos_matches -- one row per co-op match: its seed, the two
--     people (a1, the host; a2, the guest), the difficulty and the options it
--     was created with (every seat's deck, the dealt seats, the teams' rules
--     and the AI seats' handicaps), and, written once at the end, its result
--     and ended_at. A match with no ended_at is live: it is what
--     GET /api/duos/live names (B42).
--   * public.duos_match_actions -- the append-only log (B40): one row per
--     action the engine applied, gapless by seq, deduped by nonce. The actor
--     rebuilds a crashed match by folding (seed, options, log).
--
-- Unlike public.results, a co-op game writes no results row and moves no
-- rating: it is unranked, and its result lives on its own row, which names
-- both people (S6).
--
-- TRUST BOUNDARY (0004's): the seed together with the frozen decks
-- reconstructs every library and the AI team's hands, and the log replays
-- them, so no client role may read public.duos_matches or
-- public.duos_match_actions at all, as none may read public.matches or
-- public.match_actions: no grant and no policy (RLS on with no policy denies
-- every row, should a grant ever be added). A room holds nothing hidden but
-- the frozen choices, so its host and guest may read its other columns (the
-- one select policy here, S6). The server (service_role, BYPASSRLS) is the
-- only reader of the rest and the only writer of anything. A client learns
-- about its co-op match only through the server's routes and
-- `duos_view_for` over /ws/duos.
--
-- Account deletion (0012's rule: deleting an account must never be refused
-- by a foreign key): a room goes with either person (S6), and a co-op match
-- goes with either of its people, with its log (`on delete cascade`, set in
-- section 2 over the columns S6 declares). Not in SPEC: S6 declares a1 and a2
-- `not null`, so set-null, which 0012 gives a 1v1 seat, cannot hold them.
--
-- Apply order: ... -> 0035 -> 0036 -> 0037 (this file).
-- Safe to re-apply: create-if-not-exists, drop-and-add of two constraints,
-- drop-and-create of the policy, comment-on, revoke and grant.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The tables (S6).
-- ----------------------------------------------------------------------------
create table if not exists public.duos_rooms (
  code         text primary key,
  host         uuid not null references public.profiles(id) on delete cascade,
  guest        uuid references public.profiles(id) on delete cascade,
  difficulty   text not null check (difficulty in ('easy', 'medium', 'hard')),
  host_choice  jsonb not null,
  guest_choice jsonb,
  match_id     uuid,
  created_at   timestamptz not null default now(),
  expires_at   timestamptz not null
);

comment on table public.duos_rooms is
  $$S6, B33, B34: a Duos co-op room by its code (the room-code alphabet,
  R104). The host made it; the guest's join claims it once (guest is null
  until then) and starts match_id. Unjoined past expires_at it is gone.$$;

comment on column public.duos_rooms.host_choice is
  $$The host's frozen choice: {"kind":"deck","deck":{name,cards,portrait?}}
  or {"kind":"random","leanNewest":bool}. Server-only (TRUST BOUNDARY).$$;

create index if not exists duos_rooms_expires_at_idx on public.duos_rooms (expires_at);

create table if not exists public.duos_matches (
  id         uuid primary key,
  seed       text not null,
  a1         uuid not null references public.profiles(id),
  a2         uuid not null references public.profiles(id),
  difficulty text not null,
  options    jsonb not null,
  created_at timestamptz not null default now(),
  ended_at   timestamptz,
  result     jsonb
);

comment on table public.duos_matches is
  $$S6, B40, B42: one Duos co-op match. a1 is the room's host and a2 its
  guest; the AI plays b1 and b2. ended_at and result are written once, when
  the game ends; until then the match is live.$$;

comment on column public.duos_matches.options is
  $$Everything the match was created with besides its seed: every seat's
  deck in library order, the seats whose deck was dealt (R433), both teams'
  rules and the AI seats' handicaps (§9.9). With the seed, the whole hidden
  game: server-only (TRUST BOUNDARY).$$;

-- live_match_of (B42, B47): a person's live co-op match, from either seat.
create index if not exists duos_matches_live_a1_idx on public.duos_matches (a1) where ended_at is null;
create index if not exists duos_matches_live_a2_idx on public.duos_matches (a2) where ended_at is null;

create table if not exists public.duos_match_actions (
  match_id   uuid not null references public.duos_matches(id) on delete cascade,
  seq        bigint not null,
  nonce      text not null,
  action     jsonb not null,
  created_at timestamptz not null default now(),
  primary key (match_id, seq),
  unique (match_id, nonce)
);

comment on table public.duos_match_actions is
  $$S6, B40: a co-op match's append-only log, one DuosAction
  ({seat, nonce, body}) per applied step, gapless by seq. (seed, options,
  log) folds the match back (duos_fold). The actions are server-only
  (TRUST BOUNDARY).$$;


-- ----------------------------------------------------------------------------
-- 2. Account deletion: a co-op match goes with either of its people.
-- ----------------------------------------------------------------------------
-- PostgreSQL names the inline keys of section 1 `<table>_<column>_fkey`.
alter table public.duos_matches drop constraint if exists duos_matches_a1_fkey;
alter table public.duos_matches
  add constraint duos_matches_a1_fkey
  foreign key (a1) references public.profiles(id) on delete cascade;
alter table public.duos_matches drop constraint if exists duos_matches_a2_fkey;
alter table public.duos_matches
  add constraint duos_matches_a2_fkey
  foreign key (a2) references public.profiles(id) on delete cascade;


-- ----------------------------------------------------------------------------
-- 3. Row-level security: a room's people may read it; a match and its log
--    are server-only (TRUST BOUNDARY), so they get no policy at all.
-- ----------------------------------------------------------------------------
alter table public.duos_rooms enable row level security;
alter table public.duos_matches enable row level security;
alter table public.duos_match_actions enable row level security;

drop policy if exists duos_rooms_select_member on public.duos_rooms;
create policy duos_rooms_select_member
  on public.duos_rooms
  for select
  to authenticated
  using (host = app.current_profile_id() or guest = app.current_profile_id());

-- Supabase grants every new table to the client roles; take that back, then
-- grant a room's people its columns but the frozen choices. No client role
-- reads a co-op match or its log (TRUST BOUNDARY), and none writes anything.
revoke all on public.duos_rooms from public, anon, authenticated;
revoke all on public.duos_matches from public, anon, authenticated;
revoke all on public.duos_match_actions from public, anon, authenticated;

grant select (code, host, guest, difficulty, match_id, created_at, expires_at)
  on public.duos_rooms to authenticated;

-- service_role: BYPASSRLS covers reads/writes to these tables (the project's
-- own grants, as for every table); the server (src/duos/store.rs) is the
-- only writer.
