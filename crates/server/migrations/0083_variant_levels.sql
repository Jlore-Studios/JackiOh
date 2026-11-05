-- ============================================================================
-- Migration 0083: card variants as ladders of levels (SPEC §14.2; R3015000,
-- R3015006; issue #3015)
-- ============================================================================
-- Serves R3015000: every card look is a level of a variant, each a `version-…`
-- entry of the registry, and owning a level means owning every level below it
-- in its variant (R3015003). The Animated and Signature augments and the 84
-- full arts are retired. This file:
--
--   1. Writes the 15 new levels' rows of public.cosmetics, as
--      crates/server/cosmetics.json has them. `seed-catalog` upserts the whole
--      registry (0045), but `release` runs the migrations before it, and the
--      grants below need the rows' foreign keys. A row seed-catalog already
--      wrote is left as it wrote it.
--   2. Turns every combination an account owns on a card into the levels it
--      covers (R3015006), each with its public.cosmetic_grants row: the card,
--      +1, reason 'launch' and ref 'levels:0083'. Per account and card it reads
--      three flags off the rows it holds there:
--        * full art: `version-full-art`, or any retired full art (`full-art-…`);
--        * animated: `augment-animated`, an Animated art (`animated-…`), or a
--          Legendary or Mythic retired full art;
--        * signed: `augment-signature`, or a Mythic retired full art;
--      and grants Default's `version-animated` where animated; Full art's
--      `version-full-art` where full art, `version-full-art-animated` where full
--      art and animated, and `version-full-art-signed` where all three; Alt
--      art's `version-alt-art` where `version-alt-full-art` is held (its level
--      2), and `version-alt-full-art-animated` where that and animated; and each
--      one-art version's `version-<v>-animated` where `version-<v>` and
--      animated. A level that moves is never granted on a Legacy card, whose
--      picture never moves (R2724000). Signature without both Full art and
--      Animated owns no level, and Alt art or Sketch with Animated keeps its
--      first level beside Default's animated one.
--
-- It only inserts: no row is deleted or rewritten, so every retired row stays
-- with its owner and still counts as the levels it maps to where the server
-- reads it (`cosmetics::implied_by`). A row already held is passed over and gets
-- no ledger row, so each ledger still sums to what is held (§14.11).
--
-- Apply order: ... -> 0045 (public.cosmetics, card_cosmetics, cosmetic_grants)
-- -> ... -> 0050 (the reason 'launch') -> ... -> 0079 (versions and augments)
-- -> ... -> 0082 -> 0083 (this file).
-- Safe to re-apply: every statement is insert-on-conflict-do-nothing, and a
-- second run finds every level held.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The new levels in the registry's projection (R3015000).
-- ----------------------------------------------------------------------------
-- MIRRORS crates/server/cosmetics.json (its last fifteen entries).
insert into public.cosmetics (id, kind, name, rarity, is_default, chase)
values ('version-animated', 'version', 'Animated', 'Epic', false, false),
       ('version-full-art-animated', 'version', 'Full art, animated', 'Mythic', false, true),
       ('version-full-art-signed', 'version', 'Full art, animated and signed', 'Mythic', false, true),
       ('version-alt-full-art-animated', 'version', 'Alt full art, animated', 'Mythic', false, true),
       ('version-alt-full-art-signed', 'version', 'Alt full art, animated and signed', 'Mythic', false, true),
       ('version-sketch-full-art', 'version', 'Sketch full art', 'Epic', false, false),
       ('version-sketch-full-art-animated', 'version', 'Sketch full art, animated', 'Legendary', false, true),
       ('version-sketch-full-art-signed', 'version', 'Sketch full art, animated and signed', 'Mythic', false, true),
       ('version-extended-art-animated', 'version', 'Extended art, animated', 'Legendary', false, true),
       ('version-pixel-animated', 'version', 'Pixel, animated', 'Legendary', false, true),
       ('version-comic-animated', 'version', 'Comic, animated', 'Legendary', false, true),
       ('version-set-showcase-animated', 'version', 'Set showcase, animated', 'Mythic', false, true),
       ('version-out-of-frame-animated', 'version', 'Out of frame, animated', 'Mythic', false, true),
       ('version-immersive-animated', 'version', 'Immersive, animated', 'Mythic', false, true),
       ('version-prismatic-animated', 'version', 'Prismatic, animated', 'Mythic', false, true)
on conflict (id) do nothing;


-- ----------------------------------------------------------------------------
-- 2. Each combination held becomes its levels (R3015006).
-- ----------------------------------------------------------------------------
-- The held rows, the flags read off them, the levels they cover, and each
-- owned row with its ledger row written by one statement: a ledger row only for
-- a row actually inserted.
with held as (
  select cc.profile_id, cc.card_id,
         bool_or(cc.cosmetic_id = 'version-full-art' or cc.cosmetic_id like 'full-art-%') as full_art,
         bool_or(cc.cosmetic_id = 'augment-animated' or cc.cosmetic_id like 'animated-%'
                 or (cc.cosmetic_id like 'full-art-%' and c.rarity in ('Legendary', 'Mythic'))) as animated,
         bool_or(cc.cosmetic_id = 'augment-signature'
                 or (cc.cosmetic_id like 'full-art-%' and c.rarity = 'Mythic')) as signed,
         array_agg(cc.cosmetic_id) as ids
    from public.card_cosmetics cc
    left join public.cosmetics c on c.id = cc.cosmetic_id
   group by cc.profile_id, cc.card_id
),
levels as (
  select h.profile_id, h.card_id, l.cosmetic_id, l.moves
    from held h
   cross join lateral (
     values ('version-animated', h.animated, true),
            ('version-full-art', h.full_art, false),
            ('version-full-art-animated', h.full_art and h.animated, true),
            ('version-full-art-signed', h.full_art and h.animated and h.signed, true),
            ('version-alt-art', 'version-alt-full-art' = any(h.ids), false),
            ('version-alt-full-art-animated', 'version-alt-full-art' = any(h.ids) and h.animated, true),
            ('version-extended-art-animated', 'version-extended-art' = any(h.ids) and h.animated, true),
            ('version-pixel-animated', 'version-pixel' = any(h.ids) and h.animated, true),
            ('version-comic-animated', 'version-comic' = any(h.ids) and h.animated, true),
            ('version-set-showcase-animated', 'version-set-showcase' = any(h.ids) and h.animated, true),
            ('version-out-of-frame-animated', 'version-out-of-frame' = any(h.ids) and h.animated, true),
            ('version-immersive-animated', 'version-immersive' = any(h.ids) and h.animated, true),
            ('version-prismatic-animated', 'version-prismatic' = any(h.ids) and h.animated, true)
   ) as l(cosmetic_id, covered, moves)
   where l.covered
),
granted as (
  insert into public.card_cosmetics (profile_id, card_id, cosmetic_id)
  select l.profile_id, l.card_id, l.cosmetic_id
    from levels l
    join public.cards k on k.id = l.card_id
   where not (l.moves and 'Legacy' = any(k.tags))
  on conflict (profile_id, card_id, cosmetic_id) do nothing
  returning profile_id, card_id, cosmetic_id
)
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
select g.profile_id, g.card_id, g.cosmetic_id, 1, 'launch', 'levels:0083'
  from granted g;
