-- ============================================================================
-- Migration 0043: the Meditative+ set ships (SPEC §5, §8.9, R1178000, R1420)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and the patch that ships the Meditative+ set (issue
-- #1187, the last part of #1173): its 26 cards and 4 tokens join the catalog
-- the server seeds and serves the day `SHIPPED_SETS` lists the set (R1420).
-- One of its tags is new, Potion (Meditative+ #11 Living Mana Potion, #12
-- Living Armor Potion and #13 Living Health Potion, and the potions #16 Jazakus
-- and #16.1 Jazakus Prime craft, R1178000).
--
-- Why this exists: `seed-catalog` copies every catalog entry of a set that
-- ships into public.cards in one transaction, and 0030's `cards_tags_check`
-- does not admit Potion, so #11 would fail the whole catalog and the release
-- would never boot. This file puts the same check back with Potion added and
-- nothing else changed, exactly as 0030 did for Wincon (R1411). The tag list
-- stays in the same order as the engine's `Tag` union
-- (crates/engine/src/wire/catalog_types.rs), where Potion follows Wincon.
--
-- Every other column already admits the set: its types are three of §5.1's five
-- (`cards_type_check`), its rarities are the six of `cards_rarity_check`, and
-- neither `cost` (jsonb, catalog.json's value verbatim) nor `set_id` has a
-- check at all, so nothing else is widened. Its every other tag (Quickdraw, AI,
-- Jlockeed, Human, Catalyst, Prime, Book, KY, CN, Fruit, Token) is admitted
-- already.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0030 admitted, this check admits too. The new cards reach every active
-- account through 0016's grant when `seed-catalog` stamps the catalog version
-- that adds them (R481).
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0030 -> ... -> 0042 -> 0043 (this
-- file). Safe to re-apply: drop-constraint-if-exists-then-add, as 0030 does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, patch v0.2.0 Book, Pancake and AI, the v0.2.x
  -- mechanics patch Plague, patch v0.2.Y Catalyst, Prime and Acclaimed, the
  -- Meditative set Wincon, and the Meditative+ set Potion.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y, the Meditative set (R1411) and the Meditative+ set (R1178000): every
  tag is one of Human, Felinor, KY, CN, Fruit, 'Call to Chaos', Quickdraw,
  Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime, Acclaimed, Wincon,
  Potion, Token. First defined in 0002; 0010 re-added it with Jlockeed, 0015
  with Book, Pancake and AI, 0020 with Plague, 0026 with Catalyst, Prime and
  Acclaimed, 0030 with Wincon, 0043 with Potion.$$;

comment on column public.cards.set_id is
  $$SPEC §5 "Set": Core, Classic, Classic+, Meditative and Meditative+ ship
  (R380, patch v0.2.0; R1420, the Meditative set; R1178000, the Meditative+
  set); Boss and Boss-X are reserved. A set the catalog holds before it ships
  is not seeded (R1420). An index repeats across sets (B2.2), so a card is its
  id, never its card_index alone.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
