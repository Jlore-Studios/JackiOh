-- ============================================================================
-- Migration 0014: the tags of Classic and Classic+
-- ============================================================================
-- Serves SPEC.md §5 (tags) and patch v0.2.0 (docs/classic-sets.md B2.4): three
-- new tags, Book (every "Book of …" card), Pancake (Classic+ #12, #13 and the
-- eight Pancake tokens) and AI (the ten AI generated cards).
--
-- Why this exists: `db:seed-catalog` copies every catalog entry into
-- public.cards in one transaction, and 0010's `cards_tags_check` does not admit
-- the three new tags, so the first Book would fail the whole catalog. This file
-- puts the same check back with Book, Pancake and AI added and nothing else
-- changed, exactly as 0010 did for Jlockeed. The tag list stays in the same
-- order as the `Tag` union in packages/shared.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0010 admitted, this check admits too.
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0010 -> ... -> 0014 (this file).
-- Safe to re-apply: drop-constraint-if-exists-then-add, as 0010 does.
-- ============================================================================

alter table public.cards drop constraint if exists cards_tags_check;
alter table public.cards add constraint cards_tags_check check (
  -- SPEC §5: the tags a card may carry. BUILD M4-T1 named the first seven and
  -- Token, R278 added Jlockeed, and patch v0.2.0 Book, Pancake and AI.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278 and patch v0.2.0 (B2.4): every tag is one of Human,
  Felinor, KY, CN, Fruit, 'Call to Chaos', Quickdraw, Jlockeed, Book,
  Pancake, AI, Token. First defined in 0002; 0010 re-added it with Jlockeed,
  0014 with Book, Pancake and AI.$$;

comment on column public.cards.set_id is
  $$SPEC §5 "Set": Core, Classic and Classic+ ship (R380, patch v0.2.0); Boss
  and Boss-X are reserved. An index repeats across sets (B2.2), so a card is
  its id, never its card_index alone.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
