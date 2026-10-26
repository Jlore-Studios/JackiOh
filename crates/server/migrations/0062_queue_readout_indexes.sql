-- ============================================================================
-- Migration 0062: The queue readout and the refile sweep read their window
-- through an index (issue #1996)
-- ============================================================================
-- Three reads count or list the rows of a recent window, and each had no
-- index leading with its time column, so each read its table whole:
--
--   * tickets_claimed_at_idx serves tickets_recent_waits (src/db/pg.rs), the
--     tickets claimed in the queue readout's last hour (R640002), partial on
--     its `status = 'claimed'`;
--   * results_ended_at_idx serves results_count_since (src/db/pg.rs), the
--     games that ended in that hour;
--   * matches_over_ended_at_idx serves game_records_unfiled (src/db/pg.rs),
--     the refile sweep's finished matches of the last day (R1596000), partial
--     on its `status = 'over'`.
--
-- GET /api/queue/population runs the first two on every waiting client's
-- poll. No purge in api/retention.rs shortens the three tables, so without
-- these each read grows with every game ever played. Before this file no
-- index led with any of the three columns: results.ended_at sat only behind
-- a seat's profile (0004), and tickets.claimed_at and matches.ended_at were
-- in none.
--
-- Only indexes, no data. Each is built in this file's transaction, so without
-- `concurrently`, holding writes to its table while it builds.
--
-- Apply order: 0004 (matches, tickets, results) -> ... -> 0061 -> 0062 (this
-- file).
-- Safe to re-apply: every index is `create index if not exists`.
-- ============================================================================

create index if not exists tickets_claimed_at_idx
  on public.tickets (claimed_at) where status = 'claimed';
create index if not exists results_ended_at_idx
  on public.results (ended_at);
create index if not exists matches_over_ended_at_idx
  on public.matches (ended_at) where status = 'over';
