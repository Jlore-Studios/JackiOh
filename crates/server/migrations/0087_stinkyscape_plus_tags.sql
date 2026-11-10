-- ============================================================================
-- Migration 0087: the Stinkyscape+ tags (SPEC §5, issue #3535)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and issue #3535 (R3535002), which declares the
-- held Stinkyscape+ set's family tags: Mindustrial, Mineral, Automa-α,
-- Automa-β, Automa-γ, Automa-δ, Automa-Ω, Automa-ζ and Paperclip. No rule
-- reads them yet; the engine parts (SP3 E1–E9) build each one's behavior.
--
-- Why this exists: `seed-catalog` copies every catalog entry of a set that
-- ships into public.cards in one transaction, and 0083's `cards_tags_check`
-- does not admit the nine, so a held set's entries would fail the whole
-- catalog once it ships and the release would never boot. This file puts the
-- same check back with the nine added and nothing else changed, exactly as
-- 0083 did for Starship. The tag list stays in the same order as the
-- engine's `Tag` union (crates/engine/src/wire/catalog_types.rs), where the
-- nine follow Starship, before Token.
--
-- Every other column already admits the cards: their types are §5.1's
-- (`cards_type_check`), their rarities Legendary or below
-- (`cards_rarity_check`), and neither `cost` (jsonb, catalog.json's value
-- verbatim) nor `set_id` has a check at all, so nothing else is widened.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0083 admitted, this check admits too.
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0083 -> ... -> 0086 -> 0087
-- (this file). Safe to re-apply: drop-constraint-if-exists-then-add, as 0083
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
  -- Faction, #3038 Starship, and #3535 Mindustrial, Mineral, Automa-α,
  -- Automa-β, Automa-γ, Automa-δ, Automa-Ω, Automa-ζ and Paperclip.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Terminus', 'Quest', 'Otherworldly', 'Other-Origins', 'Jelly Bean', 'Drone Core', 'Weapon', 'War Doctrine', 'Swarm', 'Legacy', 'Resource', 'Faction', 'Starship', 'Mindustrial', 'Mineral', 'Automa-α', 'Automa-β', 'Automa-γ', 'Automa-δ', 'Automa-Ω', 'Automa-ζ', 'Paperclip', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y, the Meditative set (R1411), the Meditative+ set (R1178000), #1837
  (R1837000), the Stinkyscape set (R1857000), #2511 (R2511000), #2521, #3038
  (R3038000) and #3535 (R3535002): every tag is one of Human, Felinor, KY, CN,
  Fruit, 'Call to Chaos', Quickdraw, Jlockeed, Book, Pancake, AI, Plague,
  Catalyst, Prime, Acclaimed, Wincon, Potion, Terminus, Quest, Otherworldly,
  Other-Origins, 'Jelly Bean', 'Drone Core', Weapon, 'War Doctrine', Swarm,
  Legacy, Resource, Faction, Starship, Mindustrial, Mineral, Automa-α,
  Automa-β, Automa-γ, Automa-δ, Automa-Ω, Automa-ζ, Paperclip, Token. First
  defined in 0002; 0010 re-added it with Jlockeed, 0015 with Book, Pancake and
  AI, 0020 with Plague, 0026 with Catalyst, Prime and Acclaimed, 0030 with
  Wincon, 0043 with Potion, 0058 with Terminus, Quest, Otherworldly,
  Other-Origins and Jelly Bean, 0065 with Drone Core, Weapon, War Doctrine
  and Swarm, 0069 with Legacy, 0070 with Resource and Faction, 0083 with
  Starship, 0087 with Mindustrial, Mineral, Automa-α, Automa-β, Automa-γ,
  Automa-δ, Automa-Ω, Automa-ζ and Paperclip.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
