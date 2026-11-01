-- ============================================================================
-- Migration 0069: the Legacy tag (SPEC §5, §8.11, R2511000)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and issue #2511: Training Grounds #41 to #60, the
-- twenty frozen copies of Core cards, are the Legacy cards, each named
-- `Legacy: <its Core card's name>` and tagged Legacy beside the tags it had.
-- Legacy is a family tag no rule reads, as Drone Core and Swarm are.
--
-- Why this exists: `seed-catalog` copies every catalog entry of a set that
-- ships into public.cards in one transaction, and 0065's `cards_tags_check`
-- does not admit Legacy, so #41 would fail the whole catalog and the patch
-- would never boot. This file puts the same check back with Legacy added and
-- nothing else changed, exactly as 0065 did for the Stinkyscape set's four.
-- The tag list stays in the same order as the engine's `Tag` union
-- (crates/engine/src/wire/catalog_types.rs), where Legacy follows Swarm.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). The rows keep their ids, so no
-- collection row moves; `seed-catalog` rewrites the twenty rows' names and
-- tags when it seeds the catalog version that renames them. Every row 0065
-- admitted, this check admits too.
--
-- Apply order: ... -> 0065 -> 0066 -> 0067 -> 0068 -> 0069 (this file). Safe to
-- re-apply: drop-constraint-if-exists-then-add, as 0065 does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, patch v0.2.0 Book, Pancake and AI, the v0.2.x
  -- mechanics patch Plague, patch v0.2.Y Catalyst, Prime and Acclaimed, the
  -- Meditative set Wincon, the Meditative+ set Potion, #1837 Terminus, Quest,
  -- Otherworldly, Other-Origins and Jelly Bean, the Stinkyscape set Drone
  -- Core, Weapon, War Doctrine and Swarm, and #2511 Legacy.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Terminus', 'Quest', 'Otherworldly', 'Other-Origins', 'Jelly Bean', 'Drone Core', 'Weapon', 'War Doctrine', 'Swarm', 'Legacy', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y, the Meditative set (R1411), the Meditative+ set (R1178000), #1837
  (R1837000), the Stinkyscape set (R1857000) and #2511 (R2511000): every tag
  is one of Human, Felinor, KY, CN, Fruit, 'Call to Chaos', Quickdraw,
  Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime, Acclaimed, Wincon,
  Potion, Terminus, Quest, Otherworldly, Other-Origins, 'Jelly Bean', 'Drone
  Core', Weapon, 'War Doctrine', Swarm, Legacy, Token. First defined in 0002;
  0010 re-added it with Jlockeed, 0015 with Book, Pancake and AI, 0020 with
  Plague, 0026 with Catalyst, Prime and Acclaimed, 0030 with Wincon, 0043 with
  Potion, 0058 with Terminus, Quest, Otherworldly, Other-Origins and Jelly
  Bean, 0065 with Drone Core, Weapon, War Doctrine and Swarm, 0069 with
  Legacy.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
