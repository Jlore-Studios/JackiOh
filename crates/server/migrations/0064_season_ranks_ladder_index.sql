-- ============================================================================
-- Migration 0064: The rank reads take Mythic Grape's rows through an index
-- (issue #2022)
-- ============================================================================
-- GET /api/ranked, GET /api/matches/:id/ranks and every match's play
-- telemetry read ranked_standings_subset (src/db/pg.rs): a season's rows for
-- the profiles they show, which season_ranks_pkey and season_ranks_profile_idx
-- (0022) serve, plus every row at or above the Jlorious floor (R608), the
-- rows Jlorious is drawn from. No index led with `ladder`, so that arm read
-- the season whole, and the read grew with every player the season placed.
--
-- The floor is a bind, ladder::jlorious_floor() (src/ranked/ladder.rs), never
-- written here, so the index is not partial and the two cannot drift.
--
-- Only an index, no data. It is built in this file's transaction, so without
-- `concurrently`, holding writes to season_ranks while it builds.
--
-- Apply order: 0022 (season_ranks) -> ... -> 0063 -> 0064 (this file).
-- Safe to re-apply: the index is `create index if not exists`.
-- ============================================================================

create index if not exists season_ranks_season_ladder_idx
  on public.season_ranks (season_id, ladder);
