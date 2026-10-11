-- ============================================================================
-- Migration 0031: the Meditative set's two endings are results (R850, R1412)
-- ============================================================================
-- Serves SPEC §2.5 (ending the game) and §9.5 for the patch that ships the
-- Meditative set (issue #553, the last part of #496, R1420). The set adds two
-- ways a game ends, both a win (R850): `alt-win`, a player holding an
-- alternative win condition met it (Meditative #8 Reach the Summit's summit,
-- #20 Aestheticize the Game's chosen condition), and `won-by-effect`, an
-- effect won the game for a player outright (Meditative #8).
--
-- Why this exists: app.end_match writes a finished game's reason into
-- public.results, and a ranked one into public.rated_games, and each table's
-- reason check lists GameOverReason's strings as they stood (0004, 0022). A
-- game the set ends would fail its result write, and R1437 keeps such a match
-- live forever. This file puts both checks back with the two endings added and
-- nothing else changed. `voided` stays out of both, as before: a Glitch-voided
-- match records no result (R679).
--
-- No row is rewritten; every row 0004 and 0022 admitted, these checks admit
-- too. Safe to apply before the set ships, and safe to re-apply:
-- drop-constraint-if-exists-then-add.
--
-- Apply order: ... -> 0029 -> 0030 -> 0031 (this file).
-- ============================================================================

alter table public.results drop constraint if exists results_reason_check;
alter table public.results add constraint results_reason_check check (
  -- GameOverReason (crates/engine/src/wire/events.rs) but `voided` (R679).
  reason in (
    'hero-death', 'both-heroes-dead', 'concede', 'draw-accepted',
    'turn-cap', 'disconnect', 'match-ceiling', 'alt-win', 'won-by-effect'
  )
);

comment on column public.results.reason is
  $$GameOverReason's strings (crates/engine/src/wire/events.rs) but `voided`,
  which records no result (R679), kept in lockstep by results_reason_check:
  0004's seven and, since 0031, the Meditative set's `alt-win` and
  `won-by-effect` (R850, R1412).$$;

alter table public.rated_games drop constraint if exists rated_games_reason_check;
alter table public.rated_games add constraint rated_games_reason_check check (
  (kind = 'match' and reason in ('hero-death', 'both-heroes-dead', 'concede',
    'draw-accepted', 'turn-cap', 'disconnect', 'match-ceiling', 'alt-win',
    'won-by-effect'))
  -- 'abandoned' is a SeriesEnd too, but an abandoned series moves nothing
  -- (R604), so no rated game is ever recorded with it.
  or (kind = 'series' and reason in ('decided', 'exhausted', 'forfeit')));

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. Both tables keep 0004's and 0022's grants and policies.
-- ============================================================================
