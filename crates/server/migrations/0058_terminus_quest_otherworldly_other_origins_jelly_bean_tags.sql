-- ============================================================================
-- Migration 0058: the Terminus, Quest, Otherworldly, Other-Origins and Jelly
-- Bean tags (SPEC §5, R1837000)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and the patch that tags shipped cards ahead of the
-- Stinkyscape set (issue #1837): five new tags, Terminus (Core #100 Ceaseless
-- Void), Quest (Classic #90 In Too Deep), Otherworldly (Classic+ #32
-- Otherworldly Removal), Other-Origins (its three tokens, Classic+ #32.1
-- Execute, #32.2 Brawl and #32.3 Blade Storm) and Jelly Bean (Core #26 to #29,
-- the four Glowy Jelly Beans). The same patch takes Felinor off Classic #32
-- Felinor Feelings and Core #62 Friend of Felinors and Fruit off Classic+ #63
-- Fruit Tree and #68 Organic Produce, which every check since 0002 admits.
--
-- Why this exists: `seed-catalog` copies every catalog entry of a set that
-- ships into public.cards in one transaction, and 0043's `cards_tags_check`
-- does not admit the five, so Core #26 would fail the whole catalog and the
-- deploy would never boot. This file puts the same check back with the five
-- added and nothing else changed, exactly as 0043 did for Potion (R1178000).
-- The tag list stays in the same order as the engine's `Tag` union
-- (crates/engine/src/wire/catalog_types.rs), where the five follow Potion and
-- come before Token.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten:
-- `seed-catalog` writes each card's new tags when it seeds the catalog version
-- that carries them. Every row 0043 admitted, this check admits too.
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0043 -> ... -> 0057 -> 0058
-- (this file). Safe to re-apply: drop-constraint-if-exists-then-add, as 0043
-- does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, patch v0.2.0 Book, Pancake and AI, the v0.2.x
  -- mechanics patch Plague, patch v0.2.Y Catalyst, Prime and Acclaimed, the
  -- Meditative set Wincon, the Meditative+ set Potion, and #1837 Terminus,
  -- Quest, Otherworldly, Other-Origins and Jelly Bean.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Terminus', 'Quest', 'Otherworldly', 'Other-Origins', 'Jelly Bean', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y, the Meditative set (R1411), the Meditative+ set (R1178000) and #1837
  (R1837000): every tag is one of Human, Felinor, KY, CN, Fruit, 'Call to
  Chaos', Quickdraw, Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime,
  Acclaimed, Wincon, Potion, Terminus, Quest, Otherworldly, Other-Origins,
  'Jelly Bean', Token. First defined in 0002; 0010 re-added it with Jlockeed,
  0015 with Book, Pancake and AI, 0020 with Plague, 0026 with Catalyst, Prime
  and Acclaimed, 0030 with Wincon, 0043 with Potion, 0058 with Terminus,
  Quest, Otherworldly, Other-Origins and Jelly Bean.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
