-- ============================================================================
-- Migration 0053: the top ladder bots' grind (SPEC §9.12, R1555000; issue #1555)
-- ============================================================================
-- When the season's people outrank every ladder bot by more than
-- BOT_GRIND_GAP, the pool's refresh (R1454) sets the top BOT_GRINDERS active
-- bots grinding (R1555000): they are on more hours, a player rated above every
-- bot may be paired with one, they play rated games against each other while
-- the queue is empty, and each steps up one strength every
-- BOT_GRIND_STEP_GAMES rated games it wins while grinding.
--
--   * public.bots.grinding -- whether the bot grinds now; false until a
--     refresh sets it.
--   * public.bots.grind_wins -- the rated games it has won since it started
--     grinding or last stepped up, back to 0 at each step, start and stop.
--   * rated_games_bot_ended_idx -- the rated games with a bot side, by when
--     they ended: the queue's sweep reads those of the last few minutes to
--     space its matches between two bots BOT_VS_BOT_MIN_GAP_SECONDS apart,
--     and without it that read scans every rated game.
--
-- TRUST BOUNDARY: unchanged. public.bots stays server-only (0032): RLS on, no
-- policy, no grant to anon or authenticated.
--
-- Apply order: ... -> 0032 (`public.bots`) -> ... -> 0051 -> 0052 -> 0053
-- (this file).
-- Safe to re-apply: add-column-if-not-exists, create-index-if-not-exists.
-- ============================================================================

alter table public.bots add column if not exists grinding boolean not null default false;
alter table public.bots add column if not exists grind_wins integer not null default 0;

create index if not exists rated_games_bot_ended_idx
  on public.rated_games (ended_at)
  where p1_bot_id is not null or p2_bot_id is not null;

comment on column public.bots.grinding is
  'R1555000: whether the ladder bot grinds now, set and cleared by the pool''s '
  'refresh while people outrank every bot. Server-only.';

comment on column public.bots.grind_wins is
  'R1555000: the rated games the bot has won since it started grinding or last '
  'stepped up a strength; back to 0 at each step, start and stop. Server-only.';
