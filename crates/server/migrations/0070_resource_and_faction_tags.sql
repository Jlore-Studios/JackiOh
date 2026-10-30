-- ============================================================================
-- Migration 0070: the Resource and Faction tags (SPEC §5, §8.10, issue #2521)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and the Stinkyscape balance patch (issue #2521),
-- which adds two tags to cards of a set that already ships: Resource (the six
-- Stinkyscape Resources, Reknite, Korrelite, Gellium, Axnit, Narcor and Red
-- Narcor, every one a token) and Faction (the five Faction cards, #95 and #159
-- to #162).
--
-- Why this exists: `seed-catalog` copies every catalog entry of a set that
-- ships into public.cards in one transaction, and 0069's `cards_tags_check`
-- does not admit the two, so Reknite would fail the whole catalog and the
-- release would never boot. This file puts the same check back with the two
-- added and nothing else changed, exactly as 0065 did for the Stinkyscape
-- set's four (R1857000). The tag list stays in the same order as the engine's
-- `Tag` union (crates/engine/src/wire/catalog_types.rs), where the two follow
-- Legacy.
--
-- Every other column already admits the cards: their types are §5.1's five
-- (`cards_type_check`), their rarities are the six of `cards_rarity_check`,
-- and neither `cost` (jsonb, catalog.json's value verbatim) nor `set_id` has a
-- check at all, so nothing else is widened. Their every other tag (Token) is
-- admitted already.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0069 admitted, this check admits too.
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0065 -> ... -> 0069 -> 0070 (this
-- file). Safe to re-apply: drop-constraint-if-exists-then-add, as 0065 does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, patch v0.2.0 Book, Pancake and AI, the v0.2.x
  -- mechanics patch Plague, patch v0.2.Y Catalyst, Prime and Acclaimed, the
  -- Meditative set Wincon, the Meditative+ set Potion, #1837 Terminus, Quest,
  -- Otherworldly, Other-Origins and Jelly Bean, the Stinkyscape set Drone
  -- Core, Weapon, War Doctrine and Swarm, #2511 Legacy, and #2521 Resource and
  -- Faction.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Terminus', 'Quest', 'Otherworldly', 'Other-Origins', 'Jelly Bean', 'Drone Core', 'Weapon', 'War Doctrine', 'Swarm', 'Legacy', 'Resource', 'Faction', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y, the Meditative set (R1411), the Meditative+ set (R1178000), #1837
  (R1837000), the Stinkyscape set (R1857000), #2511 (R2511000) and #2521:
  every tag is one of Human, Felinor, KY, CN, Fruit, 'Call to Chaos',
  Quickdraw, Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime, Acclaimed,
  Wincon, Potion, Terminus, Quest, Otherworldly, Other-Origins, 'Jelly Bean',
  'Drone Core', Weapon, 'War Doctrine', Swarm, Legacy, Resource, Faction,
  Token. First defined in 0002; 0010 re-added it with Jlockeed, 0015 with
  Book, Pancake and AI, 0020 with Plague, 0026 with Catalyst, Prime and
  Acclaimed, 0030 with Wincon, 0043 with Potion, 0058 with Terminus, Quest,
  Otherworldly, Other-Origins and Jelly Bean, 0065 with Drone Core, Weapon,
  War Doctrine and Swarm, 0069 with Legacy, 0070 with Resource and
  Faction.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
