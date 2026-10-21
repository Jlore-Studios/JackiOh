-- ============================================================================
-- Migration 0052: a bot's rating names the AI it was earned on (R1560000; issue #1560)
-- ============================================================================
-- Serves R1560000 (amends R610): a ladder bot's rating row remembers the AI
-- generation (`crates/ai/generation.json`) it was last rated on. A rated game
-- that seats a bot whose row is from an older generation widens its deviation
-- first, by `BOT_PROMOTION_DEVIATION_BOOST` in quadrature per promotion and
-- capped at a new player's, so its rating re-settles on the promoted AI; the
-- game then stamps the row with the server's generation.
--
-- A row from before this migration has none, which the server reads as one
-- promotion behind. Nothing is backfilled: the server alone writes the column
-- (public.bot_ratings is server-only, 0022).
--
-- Apply order: 0001 -> ... -> 0022 (`public.bot_ratings`) -> ... -> 0051 ->
-- 0052 (this file).
-- Safe to re-apply: add-column-if-not-exists and comment-on.
-- ============================================================================

alter table public.bot_ratings add column if not exists ai_generation integer;

comment on column public.bot_ratings.ai_generation is
  $$R1560000: the AI generation (crates/ai/generation.json) this bot was last
  rated on; null for a row not rated since R1560000. A rated game from a newer
  generation widens the bot's deviation before rating it.$$;
