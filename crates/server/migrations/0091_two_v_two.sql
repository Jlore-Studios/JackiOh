-- ============================================================================
-- Migration 0091: 2v2 between people in a private lobby (SPEC §9.5, §13.9;
-- R3731000, R3731002, R3731003; issue #3731)
-- ============================================================================
-- Serves R3731000 and R3731002. A private lobby (0090) gains a third format,
-- `{"format":"2v2"}`: four people in two teams, the host and `guest` on the
-- host's team, `guest2` and `guest3` on the other. Each of the other team's
-- seats has its member, their frozen pick (R253, R3730001) and their Ready, as
-- the guest's seat has. No one sits in two of a lobby's seats.
--
-- A 2v2's Start makes a Duos match (0039) whose team B is two people rather
-- than the AI team: public.duos_matches gains `b1` and `b2`, the profiles at
-- those seats, null for the AI team's (every co-op match, before and after
-- this file). They follow 0051's rule for a1 and a2: `on delete set null`, so
-- a deleted account's seat on a finished match empties while the match, its
-- result and its log stay for the other three (R1497000), and
-- duos_matches_team_b_paired_unless_over_check holds a live match's team B to
-- both its people or neither, so a delete that would empty one seat of a live
-- 2v2 raises here, as 0051's check does for team A.
--
-- ACCESS: unchanged. Both tables stay the server's alone (0039, 0090).
-- No data is changed or lost: this file only ADDS nullable columns, columns
-- with defaults, indexes and checks every existing row already meets.
--
-- Apply order: ... -> 0039 (duos) -> ... -> 0051 (duos seats) -> ... ->
-- 0090 (lobbies) -> 0091 (this file).
-- Safe to re-apply: add-column-if-not-exists, create-index-if-not-exists and
-- drop-constraint-if-exists before each add.
-- ============================================================================

-- §1 R3731000: a 2v2 lobby's other team.
alter table public.lobbies
  add column if not exists guest2 uuid references public.profiles(id) on delete cascade;
alter table public.lobbies
  add column if not exists guest3 uuid references public.profiles(id) on delete cascade;
alter table public.lobbies add column if not exists guest2_pick jsonb;
alter table public.lobbies add column if not exists guest3_pick jsonb;
alter table public.lobbies add column if not exists guest2_ready boolean not null default false;
alter table public.lobbies add column if not exists guest3_ready boolean not null default false;
create index if not exists lobbies_guest2_idx on public.lobbies (guest2);
create index if not exists lobbies_guest3_idx on public.lobbies (guest3);
alter table public.lobbies drop constraint if exists lobbies_members_distinct_check;
alter table public.lobbies
  add constraint lobbies_members_distinct_check check (
    guest2 is distinct from host and guest3 is distinct from host
    and (guest2 is null or guest2 is distinct from guest)
    and (guest3 is null or guest3 is distinct from guest)
    and (guest3 is null or guest3 is distinct from guest2)
  );
comment on column public.lobbies.guest2 is
  'R3731000: a 2v2 lobby''s other team''s first seat (with guest3); null in any other format.';

-- §2 R3731002: a 2v2's team B people.
alter table public.duos_matches
  add column if not exists b1 uuid references public.profiles(id) on delete set null;
alter table public.duos_matches
  add column if not exists b2 uuid references public.profiles(id) on delete set null;
create index if not exists duos_matches_b1_idx on public.duos_matches (b1);
create index if not exists duos_matches_b2_idx on public.duos_matches (b2);
create index if not exists duos_matches_live_b1_idx on public.duos_matches (b1) where ended_at is null;
create index if not exists duos_matches_live_b2_idx on public.duos_matches (b2) where ended_at is null;
alter table public.duos_matches drop constraint if exists duos_matches_team_b_paired_unless_over_check;
alter table public.duos_matches
  add constraint duos_matches_team_b_paired_unless_over_check check (
    ended_at is not null or ((b1 is null) = (b2 is null))
  );
comment on column public.duos_matches.b1 is
  'R3731002: a 2v2''s team B person at b1 (with b2); null for the AI team''s seat.';
