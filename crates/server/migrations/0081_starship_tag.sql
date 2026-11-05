-- ============================================================================
-- Migration 0081: the Starship tag (SPEC §5, §8.10, issue #3038)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and issue #3038 (R3038000), which makes Starship a
-- tag, and a card holding it has Flying and Ranged. It adds the tag to one
-- card of a set that already ships: the Stinkyscape set's Dormant Starship.
--
-- Why this exists: `seed-catalog` copies every catalog entry of a set that
-- ships into public.cards in one transaction, and 0070's `cards_tags_check`
-- does not admit Starship, so the Dormant Starship would fail the whole
-- catalog and the release would never boot. This file puts the same check
-- back with the one added and nothing else changed, exactly as 0070 did for
-- Resource and Faction. The tag list stays in the same order as the engine's
-- `Tag` union (crates/engine/src/wire/catalog_types.rs), where Starship
-- follows Faction, before Token.
--
-- Every other column already admits the card: its type is §5.1's Unit
-- (`cards_type_check`), its rarity Legendary (`cards_rarity_check`), and
-- neither `cost` (jsonb, catalog.json's value verbatim) nor `set_id` has a
-- check at all, so nothing else is widened.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0070 admitted, this check admits too.
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0070 -> ... -> 0079 -> 0080 -> 0081
-- (this file). Safe to re-apply: drop-constraint-if-exists-then-add, as 0070
-- does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, patch v0.2.0 Book, Pancake and AI, the v0.2.x
  -- mechanics patch Plague, patch v0.2.Y Catalyst, Prime and Acclaimed, the
  -- Meditative set Wincon, the Meditative+ set Potion, #1837 Terminus, Quest,
  -- Otherworldly, Other-Origins and Jelly Bean, the Stinkyscape set Drone
  -- Core, Weapon, War Doctrine and Swarm, #2511 Legacy, #2521 Resource and
  -- Faction, and #3038 Starship.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Terminus', 'Quest', 'Otherworldly', 'Other-Origins', 'Jelly Bean', 'Drone Core', 'Weapon', 'War Doctrine', 'Swarm', 'Legacy', 'Resource', 'Faction', 'Starship', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y, the Meditative set (R1411), the Meditative+ set (R1178000), #1837
  (R1837000), the Stinkyscape set (R1857000), #2511 (R2511000), #2521 and
  #3038 (R3038000): every tag is one of Human, Felinor, KY, CN, Fruit, 'Call
  to Chaos', Quickdraw, Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime,
  Acclaimed, Wincon, Potion, Terminus, Quest, Otherworldly, Other-Origins,
  'Jelly Bean', 'Drone Core', Weapon, 'War Doctrine', Swarm, Legacy,
  Resource, Faction, Starship, Token. First defined in 0002; 0010 re-added it
  with Jlockeed, 0015 with Book, Pancake and AI, 0020 with Plague, 0026 with
  Catalyst, Prime and Acclaimed, 0030 with Wincon, 0043 with Potion, 0058
  with Terminus, Quest, Otherworldly, Other-Origins and Jelly Bean, 0065 with
  Drone Core, Weapon, War Doctrine and Swarm, 0069 with Legacy, 0070 with
  Resource and Faction, 0081 with Starship.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
