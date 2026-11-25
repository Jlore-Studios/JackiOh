-- ============================================================================
-- Migration 0101: The replays list, the account's record and the Glitch's
-- board sample read through an index (issue #4712, part of #4691)
-- ============================================================================
-- Three reads had no index of their shape, so each read rows it threw away:
--
--   * matches_p1_over_ended_idx and matches_p2_over_ended_idx serve
--     replays_list_with_modes (src/db/pg.rs), GET /api/replays and GET
--     /api/friends/recent. Each is a seat's profile, then the list's `order
--     by` (ended_at desc nulls last, id desc), partial on its `status =
--     'over'`, so each of the query's two arms, one per seat, walks its index
--     in order and stops after the page. Through 0004's
--     matches_p1_profile_id_idx and matches_p2_profile_id_idx, a seat's
--     index each, the one `or` over both seats read every match the player
--     ever played and sorted them all. A change to that `order by` needs a
--     migration that rebuilds both.
--   * results_p1_record_idx and results_p2_record_idx serve
--     results_record_for (src/db/pg.rs), GET /api/auth/me's record: a sum of
--     one count per seat, each read from its index alone (an index-only
--     scan), where the `or` over both seats read every result row of the
--     player from the table. The p2 twin also carries p1_profile_id, which
--     the p2 count reads to leave out a row that seats the player twice, as
--     the `or` counted such a row once (results has no check that its seats
--     differ; matches has, 0012).
--   * last_boards_server_sample_idx serves last_boards_sample_others
--     (src/db/pg.rs), a Glitch's two boards at every match start (R678),
--     partial on its filters, so the sample reads the few rows after a random
--     key instead of every non-empty server board in random order.
--
-- The reaper's read of the live matches past their ceiling
-- (matches_past_ceiling) needs no new index: 0004's
-- matches_status_ceiling_at_idx is its shape.
--
-- Only indexes, no data. Each is built in this file's transaction, so without
-- `concurrently`, holding writes to its table while it builds.
--
-- Apply order: 0004 (matches, results), 0017 (last_boards) -> ... -> 0100 ->
-- 0101 (this file).
-- Safe to re-apply: every index is `create index if not exists`.
-- ============================================================================

create index if not exists matches_p1_over_ended_idx
  on public.matches (p1_profile_id, ended_at desc nulls last, id desc) where status = 'over';
create index if not exists matches_p2_over_ended_idx
  on public.matches (p2_profile_id, ended_at desc nulls last, id desc) where status = 'over';
create index if not exists results_p1_record_idx
  on public.results (p1_profile_id) include (winner_profile_id, reason);
create index if not exists results_p2_record_idx
  on public.results (p2_profile_id) include (winner_profile_id, reason, p1_profile_id);
create index if not exists last_boards_server_sample_idx
  on public.last_boards (profile_id) where kind = 'server' and jsonb_array_length(board) > 0;
