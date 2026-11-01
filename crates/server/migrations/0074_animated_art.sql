-- ============================================================================
-- Migration 0074: a card pick keeps a card's Animated art (SPEC §14.2;
-- R2715000; issue #2715)
-- ============================================================================
-- Serves SPEC §14.2: every full-art card has a fourth full art, its Animated
-- art, an Epic whose registry id is `animated-<slug>`, beside its still Epic
-- `full-art-<slug>`, its Legendary and its Mythic (R2715000, amending
-- R2655000). A card's own pick names it by its id.
--
--   * public.card_cosmetic_picks's `full_art_id` check (0072) widens to take
--     an Animated art's id: `full-art-` or `animated-`, then a slug's letters,
--     digits and hyphens, 40 characters at most in all (R633's
--     PLAYER_SETTINGS_TEXT_MAX_LENGTH), still only beside a `full_art` switch
--     that is on. Kept as it was written, one the registry does not know among
--     them: the server judges it where it is worn (R1834002).
--
-- No row is rewritten: every row 0072's check took, this one takes.
--
-- Apply order: ... -> 0072 (full_art_id) -> 0073 -> 0074 (this file).
-- This file only replaces that check. Safe to re-apply:
-- drop-constraint-if-exists then add.
-- ============================================================================

alter table public.card_cosmetic_picks drop constraint if exists card_cosmetic_picks_full_art_id_check;
alter table public.card_cosmetic_picks add constraint card_cosmetic_picks_full_art_id_check
  check (full_art_id is null or (full_art is true and full_art_id ~ '^(full-art|animated)-[a-z0-9-]{1,31}$'));

comment on column public.card_cosmetic_picks.full_art_id is
  $$R2655000, R2715000: the registry id of the full art the card's pick wears
  (its still Epic, its Animated Epic, its Legendary or its Mythic), only beside
  full_art = true; NULL with full_art true is the card's still Epic, as before
  0072. Kept as written, unknown ids included; ownership is judged where it is
  worn (R1834002).$$;
