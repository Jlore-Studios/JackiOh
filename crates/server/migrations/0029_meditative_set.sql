-- ============================================================================
-- Migration 0029: the Meditative set ships (SPEC §5, §8.8, R1411, R1420)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and the patch that ships the Meditative set (issue
-- #553, the last part of #496): its 102 cards and 30 tokens join the catalog
-- the server seeds and serves the day `SHIPPED_SETS` lists the set (R1420).
-- One of its tags is new, Wincon (Meditative #8 Reach the Summit and #20
-- Aestheticize the Game, which win the game another way).
--
-- Why this exists: `seed-catalog` copies every catalog entry of a set that
-- ships into public.cards in one transaction, and 0026's `cards_tags_check`
-- does not admit Wincon, so #8 would fail the whole catalog and the release
-- would never boot. This file puts the same check back with Wincon added and
-- nothing else changed, exactly as 0026 did for Catalyst, Prime and Acclaimed.
-- The tag list stays in the same order as the engine's `Tag` union
-- (crates/engine/src/wire/catalog_types.rs).
--
-- Every other column already admits the set (R1411): its types are §5.1's five
-- (`cards_type_check`), its rarities are the six of `cards_rarity_check`, and
-- neither `cost` (jsonb, catalog.json's value verbatim) nor `set_id` has a
-- check at all, so nothing else is widened. Its every other tag (CN, Human, Felinor, KY, Quickdraw, Acclaimed,
-- Jlockeed, Plague, Catalyst, Prime, Fruit, Call to Chaos, Token) is admitted
-- already.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0026 admitted, this check admits too. The new cards reach every active
-- account through 0016's grant when `seed-catalog` stamps the catalog version
-- that adds them (R481).
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0026 -> 0027 -> 0028 -> 0029 (this
-- file). Safe to re-apply: drop-constraint-if-exists-then-add, as 0026 does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, patch v0.2.0 Book, Pancake and AI, the v0.2.x
  -- mechanics patch Plague, patch v0.2.Y Catalyst, Prime and Acclaimed, and
  -- the Meditative set Wincon.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y and the Meditative set (R1411): every tag is one of Human, Felinor,
  KY, CN, Fruit, 'Call to Chaos', Quickdraw, Jlockeed, Book, Pancake, AI,
  Plague, Catalyst, Prime, Acclaimed, Wincon, Token. First defined in 0002;
  0010 re-added it with Jlockeed, 0015 with Book, Pancake and AI, 0020 with
  Plague, 0026 with Catalyst, Prime and Acclaimed, 0029 with Wincon.$$;

comment on column public.cards.set_id is
  $$SPEC §5 "Set": Core, Classic, Classic+ and Meditative ship (R380, patch
  v0.2.0; R1420, the Meditative set); Boss and Boss-X are reserved. A set the
  catalog holds before it ships is not seeded (R1420). An index repeats across
  sets (B2.2), so a card is its id, never its card_index alone.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
