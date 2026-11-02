-- ============================================================================
-- Migration 0074: The Jlorious card back, owned only by reaching Jlorious
-- (SPEC §14.2, §14.11; R2755000, R607, R608, R640000; pull request #2755)
-- ============================================================================
-- Serves R2755000: `card-back-jlorious` is a Mythic card back that no roll,
-- pack, token, Chase Shard bar, dev fill or ladder bot's wardrobe offers; a
-- rated game grants it to each person it leaves Jlorious (src/api/ranked.rs,
-- `commit_ranked_game`), with the ledger reason 'jlorious'. This file:
--
--   1. Writes its row of public.cosmetics, as crates/server/cosmetics.json
--      has it. `seed-catalog` upserts the whole registry (0045), but `release`
--      runs the migrations before it, and the grant's foreign keys need the
--      row. A row seed-catalog already wrote is left as it wrote it.
--   2. Adds 'jlorious' to cosmetic_grants' closed set of reasons (0050).
--   3. Grants it to every person who has held Jlorious before this file: a
--      human profile (a ladder bot is never Jlorious, R640000) with a season
--      row whose peak_jlorious is set (R607), in public.account_cosmetics with
--      its public.cosmetic_grants row: no card, +1, reason 'jlorious' and ref
--      'jlorious:0074'. One who owns it already is passed over and gets no
--      ledger row, so each ledger still sums to what is held (§14.11).
--
-- Apply order: ... -> 0022 (season_ranks.peak_jlorious) -> ... -> 0045
-- (public.cosmetics, account_cosmetics and cosmetic_grants) -> ... -> 0050
-- (cosmetic_grants' reasons) -> ... -> 0073 -> 0074 (this file).
-- Safe to re-apply: every statement is insert-on-conflict-do-nothing or a
-- constraint dropped and added again, and a second grant finds every holder.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The card back in the registry's projection (R2755000).
-- ----------------------------------------------------------------------------
-- MIRRORS crates/server/cosmetics.json (its `card-back-jlorious` entry).
insert into public.cosmetics (id, kind, name, rarity, is_default, chase)
values ('card-back-jlorious', 'cardBack', 'Jlorious', 'Mythic', false, false)
on conflict (id) do nothing;


-- ----------------------------------------------------------------------------
-- 2. The ledger's reason for it (§14.11).
-- ----------------------------------------------------------------------------
alter table public.cosmetic_grants drop constraint if exists cosmetic_grants_reason_check;
alter table public.cosmetic_grants add constraint cosmetic_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
             'chase_shard', 'dev', 'launch', 'jlorious'));


-- ----------------------------------------------------------------------------
-- 3. Every person who has held Jlorious owns it (R2755000, R607).
-- ----------------------------------------------------------------------------
-- Each owned row and its ledger row are written by one statement: a ledger
-- row only for a row actually inserted.
with granted as (
  insert into public.account_cosmetics (profile_id, cosmetic_id)
  select distinct r.profile_id, 'card-back-jlorious'
    from public.season_ranks r
    join public.profiles p on p.id = r.profile_id
   where r.peak_jlorious is not null
     and p.kind = 'human'
  on conflict (profile_id, cosmetic_id) do nothing
  returning profile_id, cosmetic_id
)
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
select g.profile_id, null, g.cosmetic_id, 1, 'jlorious', 'jlorious:0074'
  from granted g;
