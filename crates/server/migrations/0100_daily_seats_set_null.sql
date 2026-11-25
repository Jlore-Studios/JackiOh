-- ============================================================================
-- Migration 0100: A deleted account empties its daily seat (R4621000, issue #4621)
-- ============================================================================
-- Replaces 0089 §1's `on delete cascade` on public.daily_games.p1_profile_id
-- and p2_profile_id. 0089's header calls that cascade 0012's rule, but 0012
-- §§2-3 empties a deleted account's seat on a finished match, so the other
-- player's history stays whole. The cascade took every finished daily game
-- with either of its players: it dropped off the other player's Daily screen
-- and its board answered 404, while its match and its result stayed and
-- counted. 0051 fixed the same mistake for the co-op seats (R1497000).
--
-- A daily seat now follows 0012's rule for a 1v1 seat: both are nullable, and
-- their keys are `on delete set null`, so a deleted account's seat on a
-- finished daily game empties while the game stays for the other player.
--
-- This file adds no check: a daily game's match is a public.matches row, so
-- 0012's matches_seats_filled_unless_over_check already refuses a delete that
-- would empty a running game's seat (R3727008), and the rule stays in one
-- place. daily_games_seats_check (p1 <> p2) passes a null seat.
--
-- Apply order: ... -> 0012 (the 1v1 seats) -> ... -> 0089 (the daily tables)
-- -> ... -> 0098 -> 0099 -> 0100 (this file).
-- Safe to re-apply: drop-not-null, drop-constraint-if-exists before each add,
-- comment-on.
-- ============================================================================

alter table public.daily_games alter column p1_profile_id drop not null;
alter table public.daily_games alter column p2_profile_id drop not null;
alter table public.daily_games drop constraint if exists daily_games_p1_profile_id_fkey;
alter table public.daily_games
  add constraint daily_games_p1_profile_id_fkey
  foreign key (p1_profile_id) references public.profiles(id) on delete set null;
alter table public.daily_games drop constraint if exists daily_games_p2_profile_id_fkey;
alter table public.daily_games
  add constraint daily_games_p2_profile_id_fkey
  foreign key (p2_profile_id) references public.profiles(id) on delete set null;
comment on column public.daily_games.p1_profile_id is
  'The account at p1 as the game began; null once that account is deleted, which only a finished game allows (R4621000).';
comment on column public.daily_games.p2_profile_id is
  'The account at p2 as the game began; null once that account is deleted, which only a finished game allows (R4621000).';
