-- ============================================================================
-- Migration 0025: the Catalyst, Prime and Acclaimed tags
-- ============================================================================
-- Serves SPEC.md §5 (tags) and patch v0.2.Y (#322): three new tags, Catalyst
-- (C+ #38 Solarius, C+ #46 Felinor Flagbearer), Prime (their Prime tokens,
-- C+ #38.1 and C+ #46.1) and Acclaimed (C #80 BOOM! Big Max, C+ #37 Wardrum).
--
-- Why this exists: `db:seed-catalog` copies every catalog entry into
-- public.cards in one transaction, and 0020's `cards_tags_check` does not admit
-- the new tags, so the first such card would fail the whole catalog. This file
-- puts the same check back with the three added and nothing else changed,
-- exactly as 0020 did for Plague. The tag list stays in the same order as the
-- `Tag` union in packages/shared.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0020 admitted, this check admits too.
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0020 -> ... -> 0025 (this file).
-- Safe to re-apply: drop-constraint-if-exists-then-add, as 0020 does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, patch v0.2.0 Book, Pancake and AI, the v0.2.x
  -- mechanics patch Plague, and patch v0.2.Y Catalyst, Prime and Acclaimed.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch and patch
  v0.2.Y: every tag is one of Human, Felinor, KY, CN, Fruit, 'Call to Chaos',
  Quickdraw, Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime, Acclaimed,
  Token. First defined in 0002; 0010 re-added it with Jlockeed, 0015 with Book,
  Pancake and AI, 0020 with Plague, 0025 with Catalyst, Prime and Acclaimed.$$;
