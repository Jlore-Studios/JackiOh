-- ============================================================================
-- Migration 0081: custom cards are removed, so the custom_cards table goes
-- (SPEC §12.9; issues #3095 and #2956)
-- ============================================================================
-- Custom cards (R915008; issue #918) are removed from the game (#2956): the
-- card creator went in #3094 and `/api/custom-cards` in this change, so no
-- code reads or writes what 0037 added. Players' saved custom cards are deleted
-- with no export (#2956, "Decided"), and a stored game that used one is no
-- longer supported.
--
--   * public.custom_cards is dropped. Its row-level security policy
--     (custom_cards_select_own), its checks, its index and its grants go with
--     it, and so do the rows: that is the deletion.
--   * app.create_custom_card and app.update_custom_card, the two write paths
--     0037 made, are dropped.
--   * app.settings loses `max_custom_cards`, the cap the create read.
--
-- Nothing else depends on the table: no foreign key points at it, and a saved
-- deck that names a custom id keeps only that id (decks are drafts, R250, so
-- the id is text in the deck's own row). Every drop is plain (RESTRICT): were
-- something to depend on one, the file fails rather than drop that too.
--
-- 0037 stays where it is: a migration is history, and a database that has not
-- applied it yet makes the table and drops it again.
--
-- Apply order: ... -> 0037 (custom_cards) -> ... -> 0080 -> 0081 (this file).
-- This file only drops. Safe to re-apply: every drop is `if exists`.
-- ============================================================================

drop function if exists app.create_custom_card(uuid, text, jsonb, text, jsonb, timestamptz, int);
drop function if exists app.update_custom_card(uuid, text, int, jsonb, text, jsonb, timestamptz);

drop table if exists public.custom_cards;

delete from app.settings where key = 'max_custom_cards';
