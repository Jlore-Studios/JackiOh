-- ============================================================================
-- Migration 0098: The public players list and the daily pay sums read
-- through an index (issue #4468)
-- ============================================================================
-- Three reads had no index of their shape, so each read rows it threw away:
--
--   * player_stats_public_games_idx serves player_stats_list_public
--     (src/db/pg.rs), GET /api/stats/players (R654, R3911000), open to
--     anyone. Its key is the query's `order by`, expression for expression,
--     and it is partial on the query's `is_private = false`, so a page walks
--     it in order and stops after its rows. player_stats had only its primary
--     key (0021), so every request unpacked every public row's stats and
--     sorted them all. A change to that `order by` needs a migration that
--     rebuilds this index.
--   * jlore_grants_profile_reason_at_idx serves game_jlore_totals and
--     fleet_track_grants_profile_reason_at_idx serves fleet_get (both
--     src/economy/store.rs): the day's `game` sums of a finished game's payout
--     and of the daily-cap reads (R1483000). Both ledgers had only
--     (profile_id, id) (0045, 0086), so each sum read every row the account
--     was ever granted and filtered by day.
--
-- Only indexes, no data. Each is built in this file's transaction, so without
-- `concurrently`, holding writes to its table while it builds.
--
-- Apply order: 0021, 0045, 0086 -> ... -> 0097 -> 0098 (this file).
-- Safe to re-apply: every index is `create index if not exists`.
-- ============================================================================

create index if not exists player_stats_public_games_idx
  on public.player_stats
     ((case when (stats->>'games') ~ '^[0-9]+$' then (stats->>'games')::bigint else 0 end) desc,
      updated_at desc, profile_id)
  where is_private = false;
create index if not exists jlore_grants_profile_reason_at_idx
  on public.jlore_grants (profile_id, reason, at);
create index if not exists fleet_track_grants_profile_reason_at_idx
  on public.fleet_track_grants (profile_id, reason, at);
