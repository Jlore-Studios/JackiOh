-- ============================================================================
-- Migration 0051: A deleted account empties its co-op seat (R1497000, issue #1497)
-- ============================================================================
-- Replaces 0039 §2's `on delete cascade` on public.duos_matches.a1 and a2.
-- That cascade took a co-op match, its result and its log with either of its
-- people, so deleting one account erased every finished co-op game its
-- teammate had played with it, and a match that started between
-- `DELETE /api/account`'s refusal and its delete vanished mid-game.
--
-- A co-op seat now follows 0012's rule for a 1v1 seat (0012 §§2-3):
--
--   1. a1 and a2 are nullable, and their keys are `on delete set null`, so a
--      deleted account's seat on a finished co-op match empties while the
--      match, its result and its log stay for the teammate;
--   2. duos_matches_seats_filled_unless_over_check: only a match that has
--      ended may have an empty seat, so a delete that would empty a seat of a
--      live co-op match raises here.
--
-- `DELETE /api/account` refuses a person in a live co-op match first
-- (src/api/auth.rs, `remove_account`); the check is the backstop should a
-- match start in between. A co-op room still goes with either of its people
-- (0039 §1's `on delete cascade` on duos_rooms.host and guest).
--
-- Apply order: ... -> 0012 (the 1v1 seats) -> ... -> 0039 (the co-op tables)
-- -> ... -> 0049 -> 0050 -> 0051 (this file).
-- Safe to re-apply: drop-not-null, drop-constraint-if-exists before each add.
-- ============================================================================

alter table public.duos_matches alter column a1 drop not null;
alter table public.duos_matches alter column a2 drop not null;
alter table public.duos_matches drop constraint if exists duos_matches_a1_fkey;
alter table public.duos_matches
  add constraint duos_matches_a1_fkey
  foreign key (a1) references public.profiles(id) on delete set null;
alter table public.duos_matches drop constraint if exists duos_matches_a2_fkey;
alter table public.duos_matches
  add constraint duos_matches_a2_fkey
  foreign key (a2) references public.profiles(id) on delete set null;
alter table public.duos_matches drop constraint if exists duos_matches_seats_filled_unless_over_check;
alter table public.duos_matches
  add constraint duos_matches_seats_filled_unless_over_check check (
    ended_at is not null or (a1 is not null and a2 is not null)
  );
