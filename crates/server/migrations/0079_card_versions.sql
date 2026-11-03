-- ============================================================================
-- Migration 0079: a card pick keeps a version and its augments (SPEC §14.2;
-- R2725000; issue #2725)
-- ============================================================================
-- Serves SPEC §14.2: a card wears one version, its normal face or one of the
-- registry's `version-…` entries (the Full art version, `version-full-art`,
-- first), and the augments layered on it, each an `augment-…` entry
-- (Animated, Signature), every one owned card by card like a foil (R2725000).
--
--   * public.card_cosmetic_picks gains `augments`, the augment ids the card
--     layers on its version: NULL for none, else one id or more, each
--     `augment-` and a slug's letters, digits and hyphens, 40 characters at
--     most in all (R633's PLAYER_SETTINGS_TEXT_MAX_LENGTH), none NULL: the
--     list joined by commas splits back into as many ids as it holds.
--   * The `full_art_id` check (0072, widened by 0075) widens again to take a
--     version's id, `version-` and a slug, 40 characters at most, still only
--     beside a `full_art` switch that is on.
--   * The `some` check (0061) counts an augments list as a pick, so a row of
--     only augments is kept.
--
-- Kept as written, ids the registry does not know among them: the server
-- judges each where it is worn (R1834002). No row is rewritten and nothing is
-- granted: every row 0075's checks took, these take, and the retired full arts
-- own what they imply where the server reads them (R2725002), never as rows.
--
-- Apply order: ... -> 0072 (full_art_id) -> ... -> 0075 -> 0076 -> 0077 -> 0078 -> 0079 (this file).
-- Safe to re-apply: add-column-if-not-exists, drop-constraint-if-exists then
-- add.
-- ============================================================================

alter table public.card_cosmetic_picks add column if not exists augments text[];

alter table public.card_cosmetic_picks drop constraint if exists card_cosmetic_picks_augments_check;
alter table public.card_cosmetic_picks add constraint card_cosmetic_picks_augments_check
  check (augments is null or (cardinality(augments) >= 1
    and array_to_string(augments, ',') ~ '^augment-[a-z0-9-]{1,32}(,augment-[a-z0-9-]{1,32})*$'
    and cardinality(string_to_array(array_to_string(augments, ','), ',')) = cardinality(augments)));

alter table public.card_cosmetic_picks drop constraint if exists card_cosmetic_picks_full_art_id_check;
alter table public.card_cosmetic_picks add constraint card_cosmetic_picks_full_art_id_check
  check (full_art_id is null or (full_art is true
    and full_art_id ~ '^((full-art|animated)-[a-z0-9-]{1,31}|version-[a-z0-9-]{1,32})$'));

alter table public.card_cosmetic_picks drop constraint if exists card_cosmetic_picks_some_check;
alter table public.card_cosmetic_picks add constraint card_cosmetic_picks_some_check
  check (foil is not null or finish is not null or border is not null or full_art is not null
    or augments is not null);

comment on column public.card_cosmetic_picks.full_art_id is
  $$R2655000, R2715000, R2725000: the registry id of the full art or the version
  the card's pick wears (its still Epic, its Animated Epic, its Legendary, its
  Mythic, or a `version-…` the card is offered), only beside full_art = true;
  NULL with full_art true is the card's still Epic, as before 0072. Kept as
  written, unknown ids included; ownership is judged where it is worn
  (R1834002).$$;

comment on column public.card_cosmetic_picks.augments is
  $$R2725000: the augment ids (`augment-…`) the card layers on the version it
  wears, NULL for none. Kept as written; each is judged where it is worn, owned
  on the card and offered there (R1834002, R2725001).$$;
