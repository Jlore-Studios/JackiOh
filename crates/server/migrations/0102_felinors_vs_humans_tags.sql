-- ============================================================================
-- Migration 0102: the Felinors vs Humans tags (SPEC §5, issue #5235)
-- ============================================================================
-- Serves SPEC.md §5 (tags) and issue #5235 (R5235002), which declares the
-- Felinors vs Humans set's three new tags: Plagiarized (its 82 reprints of
-- Infinite Assault cards, FvH #116 to #197), Roblox-Tool (FvH #19 Trowel and
-- #20 Bomb) and Golem (FvH #100.1 Felinor Golem and #100.2 Human Golem).
--
-- Why this exists: Felinors vs Humans is a set held while wave 1 builds it
-- (R5235000), so `seed-catalog` writes none of its cards today. But 0099's
-- `cards_tags_check` does not admit the three, and `seed-catalog` copies
-- every catalog entry of a set that ships into public.cards in one
-- transaction, so the day the set ships its cards would fail the whole
-- catalog and the release would never boot. The check names the whole engine
-- `Tag` union, so that day nothing fails. This file puts the same check back
-- with the three added and nothing else changed, exactly as 0099 did for the
-- Infinite Assault. The tag list stays in the same order as the engine's
-- `Tag` union (crates/engine/src/wire/catalog_types.rs), where the three
-- follow Commander, before Token.
--
-- Nothing reads the tags at runtime: the API serves catalog.json itself, and
-- public.cards only gives the collection's foreign keys and L6 something to
-- point at (0002's comment on the table). No existing row is rewritten. Every
-- row 0099 admitted, this check admits too.
--
-- Apply order: 0002 (`public.cards`) -> ... -> 0099 -> 0100 -> 0101 -> 0102
-- (this file). Safe to re-apply: drop-constraint-if-exists-then-add, as 0099
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
  -- Faction, #3038 Starship, #3535 Mindustrial, Mineral, Automa-α,
  -- Automa-β, Automa-γ, Automa-δ, Automa-Ω, Automa-ζ and Paperclip,
  -- #4577 Hallucination, Overclock, Batch, Recursion, Jailbreak, Firewall,
  -- Scraper, Singularity and Commander, and #5235 Plagiarized, Roblox-Tool
  -- and Golem.
  tags <@ array[
    'Human', 'Felinor', 'KY', 'CN', 'Fruit', 'Call to Chaos', 'Quickdraw', 'Jlockeed', 'Book', 'Pancake', 'AI', 'Plague', 'Catalyst', 'Prime', 'Acclaimed', 'Wincon', 'Potion', 'Terminus', 'Quest', 'Otherworldly', 'Other-Origins', 'Jelly Bean', 'Drone Core', 'Weapon', 'War Doctrine', 'Swarm', 'Legacy', 'Resource', 'Faction', 'Starship', 'Mindustrial', 'Mineral', 'Automa-α', 'Automa-β', 'Automa-γ', 'Automa-δ', 'Automa-Ω', 'Automa-ζ', 'Paperclip', 'Hallucination', 'Overclock', 'Batch', 'Recursion', 'Jailbreak', 'Firewall', 'Scraper', 'Singularity', 'Commander', 'Plagiarized', 'Roblox-Tool', 'Golem', 'Token'
  ]::text[]
);

comment on constraint cards_tags_check on public.cards is
  $$SPEC §5, R278, patch v0.2.0 (B2.4), the v0.2.x mechanics patch, patch
  v0.2.Y, the Meditative set (R1411), the Meditative+ set (R1178000), #1837
  (R1837000), the Stinkyscape set (R1857000), #2511 (R2511000), #2521, #3038
  (R3038000), #3535 (R3535002), #4577 (R4577001) and #5235 (R5235002): every
  tag is one of Human, Felinor, KY, CN, Fruit, 'Call to Chaos', Quickdraw,
  Jlockeed, Book, Pancake, AI, Plague, Catalyst, Prime, Acclaimed, Wincon,
  Potion, Terminus, Quest, Otherworldly, Other-Origins, 'Jelly Bean',
  'Drone Core', Weapon, 'War Doctrine', Swarm, Legacy, Resource, Faction,
  Starship, Mindustrial, Mineral, Automa-α, Automa-β, Automa-γ, Automa-δ, Automa-Ω,
  Automa-ζ, Paperclip, Hallucination, Overclock, Batch, Recursion, Jailbreak,
  Firewall, Scraper, Singularity, Commander, Plagiarized, Roblox-Tool, Golem,
  Token. First defined in 0002; 0010 re-added it with Jlockeed, 0015 with
  Book, Pancake and AI, 0020 with Plague, 0026 with Catalyst, Prime and
  Acclaimed, 0030 with Wincon, 0043 with Potion, 0058 with Terminus, Quest,
  Otherworldly, Other-Origins and Jelly Bean, 0065 with Drone Core, Weapon,
  War Doctrine and Swarm, 0069 with Legacy, 0070 with Resource and Faction,
  0083 with Starship, 0088 with Mindustrial, Mineral, Automa-α, Automa-β,
  Automa-γ, Automa-δ, Automa-Ω, Automa-ζ and Paperclip, 0099 with
  Hallucination, Overclock, Batch, Recursion, Jailbreak, Firewall, Scraper,
  Singularity and Commander, 0102 with Plagiarized, Roblox-Tool and Golem.$$;

-- ----------------------------------------------------------------------------
-- Grants: nothing changes. public.cards keeps 0002's grants and policies.
-- ============================================================================
