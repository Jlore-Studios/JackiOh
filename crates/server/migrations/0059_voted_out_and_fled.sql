-- ============================================================================
-- Migration 0059: Two new endings, voted out and fled, are results
-- (issue #1843, R1843005)
-- ============================================================================
-- Serves SPEC §2.5 (ending the game), §9.5 and §9.12 for issue #1843, the
-- Stinkyscape set's two endings, merged on main before the set ships (R1420):
--
--   * `voted-out`: a player's vote tally filled (the set's three Brains), so they
--     were removed from the game: a loss for them, a win for the other player
--     (R1843000); `both-voted-out`: both tallies filled at one check, a draw
--     (R1843001);
--   * `fled`: a player escaped the universe (the set's Edge of the Universe), a
--     win for them and a draw, not a loss, for the other player (R1843002,
--     R1843004), or a draw for both when both fled at once; `fled-decisive`:
--     the fleer chose a loss for the other player instead (the card's Radiant
--     face).
--
-- Why this exists: app.end_match writes a finished game's reason into
-- public.results, and a ranked one into public.rated_games, and each table's
-- reason check lists GameOverReason's strings as they stand (0004, 0022, 0031).
-- A game one of these endings ended would fail its result write, and R1437
-- would keep such a match live forever. This file puts both checks back with
-- the four endings added and nothing else changed, as 0031 did for the
-- Meditative set's two (R1412). `voided` stays out of both: a Glitch-voided
-- match records no result (R679).
--
-- No row is rewritten; every row 0031 admitted, these checks admit too. Safe
-- to apply before any card reaches the endings, and safe to re-apply:
-- drop-constraint-if-exists-then-add.
--
-- Apply order: ... -> 0057 -> 0058 -> 0059 (this file).
-- ============================================================================

alter table public.results drop constraint if exists results_reason_check;
alter table public.results add constraint results_reason_check check (
  -- GameOverReason (crates/engine/src/wire/events.rs) but `voided` (R679).
  reason in (
    'hero-death', 'both-heroes-dead', 'concede', 'draw-accepted',
    'turn-cap', 'disconnect', 'match-ceiling', 'alt-win', 'won-by-effect',
    'voted-out', 'both-voted-out', 'fled', 'fled-decisive'
  )
);

comment on column public.results.reason is
  $$GameOverReason's strings (crates/engine/src/wire/events.rs) but `voided`,
  which records no result (R679), kept in lockstep by results_reason_check:
  0004's seven, since 0031 the Meditative set's `alt-win` and `won-by-effect`
  (R850, R1412), and since 0059 `voted-out`, `both-voted-out`, `fled` and
  `fled-decisive` (R1843005). A `fled` row's winner won and the other player
  drew (R1843004).$$;

alter table public.rated_games drop constraint if exists rated_games_reason_check;
alter table public.rated_games add constraint rated_games_reason_check check (
  (kind = 'match' and reason in ('hero-death', 'both-heroes-dead', 'concede',
    'draw-accepted', 'turn-cap', 'disconnect', 'match-ceiling', 'alt-win',
    'won-by-effect', 'voted-out', 'both-voted-out', 'fled', 'fled-decisive'))
  -- 'abandoned' is a SeriesEnd too, but an abandoned series moves nothing
  -- (R604), so no rated game is ever recorded with it.
  or (kind = 'series' and reason in ('decided', 'exhausted', 'forfeit')));

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. Both tables keep 0004's and 0022's grants and policies.
-- ============================================================================
