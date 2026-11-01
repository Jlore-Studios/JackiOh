-- ============================================================================
-- Migration 0072: a card pick keeps which full art it wears (SPEC §14.2;
-- R2655000; issue #2655)
-- ============================================================================
-- Serves SPEC §14.2: every full-art card has three full arts, an Epic (the
-- still face), a Legendary (animated) and a Mythic (signed), and a card's own
-- pick chooses which owned one it wears (R2655000, amending R1655000).
--
--   * public.card_cosmetic_picks gains `full_art_id` -- the registry id of the
--     full art a card's pick names, beside a `full_art` switch that is on. A
--     NULL id with the switch on is the card's Epic full art, as every row
--     before this file means it; the switch off holds no id. Kept as it was
--     written, one the registry does not know among them, as 0061 keeps a
--     foil's: the server judges it where it is worn (R1834002). Its shape is a
--     full art's id: `full-art-`, then a slug's letters, digits and hyphens, 40
--     characters at most in all (R633's PLAYER_SETTINGS_TEXT_MAX_LENGTH).
--
-- No row is rewritten: a row from before this file holds no id, and reads as
-- it did.
--
-- Apply order: ... -> 0061 (card_cosmetic_picks) -> ... -> 0071 -> 0072 (this file).
-- This file only ADDS objects. Safe to re-apply: add-column-if-not-exists,
-- drop-constraint-if-exists then add.
-- ============================================================================

alter table public.card_cosmetic_picks add column if not exists full_art_id text;
alter table public.card_cosmetic_picks drop constraint if exists card_cosmetic_picks_full_art_id_check;
alter table public.card_cosmetic_picks add constraint card_cosmetic_picks_full_art_id_check
  check (full_art_id is null or (full_art is true and full_art_id ~ '^full-art-[a-z0-9-]{1,31}$'));

comment on column public.card_cosmetic_picks.full_art_id is
  $$R2655000: the registry id of the full art the card's pick wears (its Epic,
  Legendary or Mythic), only beside full_art = true; NULL with full_art true is
  the card's Epic, as before 0072. Kept as written, unknown ids included;
  ownership is judged where it is worn (R1834002).$$;
