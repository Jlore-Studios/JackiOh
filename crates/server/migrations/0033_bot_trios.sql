-- ============================================================================
-- Migration 0033: the ladder bots' Conquest trios (SPEC §9.5, §9.12, R641000; issue #641)
-- ============================================================================
-- A ranked Conquest ticket that waits past its fallback time with no person to
-- pair with plays a series against a ladder bot (R641000), as a Best of 1 or
-- All Random ticket plays a match against one (R1453). The series freezes a
-- trio for each side (R259), so a bot needs one: three decks a player could
-- queue as a trio, sharing no card (R253), dealt by the pool (R1454) for one
-- catalog version and dealt again when it changes, as its Best-of-1 decks are.
--
--   * public.bots.trio -- the frozen trio, in the shape tickets.frozen_trio
--     holds (`{ name, decks: [{ name, cards, portrait }, x3] }`), or null
--     until the pool deals it.
--   * public.bots.trio_catalog_version -- the catalog version it was dealt
--     for, or null.
--
-- TRUST BOUNDARY: unchanged. public.bots stays server-only (0032): RLS on, no
-- policy, no grant to anon or authenticated.
--
-- Apply order: ... -> 0031 -> 0032 (`public.bots`) -> 0033 (this file).
-- Safe to re-apply: add-column-if-not-exists.
-- ============================================================================

alter table public.bots add column if not exists trio jsonb;
alter table public.bots add column if not exists trio_catalog_version text;

comment on column public.bots.trio is
  'R641000: the Conquest trio a ladder bot plays a series with: three decks '
  'sharing no card (R253), frozen as tickets.frozen_trio is, or null until the '
  'pool deals it. Server-only.';

comment on column public.bots.trio_catalog_version is
  'R641000: the catalog version the bot''s trio was dealt for, or null before '
  'it was. Server-only.';
