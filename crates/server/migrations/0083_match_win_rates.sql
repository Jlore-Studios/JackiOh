-- ============================================================================
-- Migration 0083: a match keeps the win-rate snapshot it started with
-- (SPEC §8.8 row 50, §8.9 row 2, §9.3; R3049000; issue #3049)
-- ============================================================================
-- Serves R3049000: M #50 CN Tech and M+ #2 NA Tech read the server's cached
-- card stats (R1367000), not a table compiled into the build. The server takes
-- a snapshot of them as a match starts and hands it to the game as setup, as
-- it hands it the last boards (0017) and the Glitch boards (0024); the game
-- keeps the rows a pick can name, and the match row keeps the same, so every
-- rebuild, replay and game record folds the same picks (§9.3).
--
--   * public.matches gains `win_rates`, the game's kept snapshot:
--     { patch, source, cards: [{ id, wins, games }] }, a jsonb object, or NULL
--     on a match handed none and on every match started before this file,
--     which folds against the table its version shipped with (R1266000).
--     Public aggregate figures, never part of PlayerView.
--
-- No row is rewritten and nothing is granted: service_role reads and writes
-- matches as before, and app.live_matches() returns the whole row.
--
-- Apply order: ... -> 0004 (matches) -> ... -> 0080 -> 0081 -> 0082 -> 0083 (this file).
-- Safe to re-apply: add-column-if-not-exists, drop-constraint-if-exists then
-- add.
-- ============================================================================

alter table public.matches add column if not exists win_rates jsonb;
alter table public.matches drop constraint if exists matches_win_rates_check;
alter table public.matches add constraint matches_win_rates_check
  check (win_rates is null or jsonb_typeof(win_rates) = 'object');

comment on column public.matches.win_rates is
  $$R3049000: the win-rate snapshot the match started with, as the game kept it
  ({ patch, source, cards: [{ id, wins, games }] }), which every fold of the
  match is handed; NULL on a match handed none and on every match from before
  migration 0083, which folds against its version's frozen table (R1266000).$$;
