-- ============================================================================
-- Migration 0028: online replays (issue #510) — a finished match's final hash
-- ============================================================================
-- Serves SPEC §9.3 and SPEC §11 R768 (what a replay shows).
--
--   * public.matches.final_hash -- R768: hash_state of the match's final
--     state, which its replay's fold must reach before any step is served.
--     The server writes it with the match's result, in the same transaction
--     (src/api/results.rs), and never changes it after. It is null for a match
--     that ended before 0028, for one the reaper resolved (R112: the reaper
--     reads no state) and for one not over yet; such a replay is held to its
--     results row instead (the same winner, reason and turn count).
--
-- TRUST BOUNDARY: unchanged. public.matches stays unreachable for client roles
-- (0004): no grant, no policy, only the server reads the column.
--
-- Apply order: ... -> 0025 -> 0026 -> 0027 -> 0028.
-- Safe to re-apply: add-column-if-not-exists, comment-on.
-- ============================================================================

alter table public.matches add column if not exists final_hash text;

comment on column public.matches.final_hash is
  $$R768, SPEC §9.3: hash_state of the final state, written with the result and
  never changed. Null before 0028, for a reaped match (R112) and while the match
  is not over; a replay of such a match is held to its results row.$$;
