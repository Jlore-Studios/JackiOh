-- ============================================================================
-- Migration 0073: friends always watch, so profiles drops its watching setting
-- (SPEC §9.5; R2615000; issue #2620)
-- ============================================================================
-- R2615000 (#2615) removed "Let friends watch my games": no server has read or
-- written public.profiles.allow_spectators since. 0038 added it, and it stayed
-- while a server on the code before R2615000 could still be serving through a
-- deploy's overlap, since that code reads it.
--
--   * public.profiles loses `allow_spectators`. Its column grant to
--     `authenticated` (0038) and its comment go with it. Nothing else depends
--     on it, so the drop is plain (RESTRICT): were something to, the file
--     fails rather than drop that too.
--
-- Apply order: ... -> 0038 (allow_spectators) -> ... -> 0072 -> 0073 (this file).
-- This file only drops that column. Safe to re-apply: drop-column-if-exists.
-- ============================================================================

alter table public.profiles drop column if exists allow_spectators;
