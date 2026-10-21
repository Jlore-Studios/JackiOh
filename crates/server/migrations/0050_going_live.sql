-- ============================================================================
-- Migration 0050: Going live (SPEC §14.12; R1310000, R1310001, R1310002; issue #1310)
-- ============================================================================
-- Serves SPEC §14.12 (Going live), quoted:
--   "A new account starts with the starter cards and its tracks' first rewards
--   ... An existing account keeps every card granted to it, so nobody loses a
--   deck. Its economy starts at zero, and the ledger records the move with a
--   reason that says it was the launch. A new catalog version grants no
--   account its cards ... The full-art faces start unowned for everyone,
--   existing accounts included."
--
-- Until now every person's active account owned every card: R111's grant on
-- the pending -> active transition (0002) and R481's grant of each new catalog
-- version (0016, over people only since 0032, passing over the dev accounts
-- since 0046). This file ends both and starts every economy at zero:
--
--   1. app.settings.starter_cards, a jsonb array of card ids, the starter
--      cards (R1310000). `seed-catalog` writes it from crates/server/src/config.rs
--      STARTER_CARDS before it stamps the catalog version, so the list has one
--      source; it is empty until then.
--   2. app.grant_launch_collection, R111's activation grant, grants one copy
--      of each non-token card of the current version that starter_cards names,
--      and nothing else. Still idempotent, still reason 'launch'.
--   3. The catalog stamp's grant is dropped (R1310001): the trigger
--      settings_grant_catalog_growth, app.on_catalog_version_stamped() and
--      app.grant_launch_collection_all(). A new set's cards come from its
--      track and its packs.
--   4. Every economy ledger takes the reason 'launch' (§14.11), the
--      collection's own since 0002.
--   5. The launch (R1310002): every collection row stays, so nobody loses a
--      card or a deck. Every economy holding is zeroed by an offsetting ledger
--      row of reason 'launch' and ref 'launch:0050' and then deleted: the
--      wallets, the inventory, the counters, the set tracks (every row, so an
--      older track locks again), the held quests (a 'removed' row each) and the
--      quest schedules, and every owned full-art cosmetic (R1313000). The
--      starter cosmetics are owned with no row (§14.2); every other cosmetic
--      row is a starter's today and is left as it is. It runs once per
--      database: app.settings.economy_launched marks it done.
--
-- Apply order: ... -> 0002 (app.grant_launch_collection) -> ... -> 0016 (the
-- stamp's trigger) -> ... -> 0045 (the economy's ledgers) -> 0046 -> 0047 ->
-- 0048 -> 0049 -> 0050 (this file).
-- Safe to re-apply: insert-on-conflict-do-nothing, create-or-replace,
-- drop-if-exists, drop-constraint-if-exists before each add, and the reset
-- skipped once economy_launched is set.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The starter cards (R1310000).
-- ----------------------------------------------------------------------------
insert into app.settings (key, value)
values ('starter_cards', '[]'::jsonb)
on conflict (key) do nothing;


-- ----------------------------------------------------------------------------
-- 2. The activation grant: the starter cards only (R111, R1310000).
-- ----------------------------------------------------------------------------
-- 0002's function, as it was, but over the cards starter_cards names.
create or replace function app.grant_launch_collection(p_profile_id uuid)
returns void
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_launch_quantity int;
begin
  select (app.setting('launch_quantity'))::text::int into v_launch_quantity;

  -- Idempotency: skip any card this profile already has a 'launch' grant
  -- for, rather than computing a delta up to a target quantity (0002, R111).
  perform app.grant_cards(
    p_profile_id,
    coalesce(
      (
        select jsonb_agg(jsonb_build_object('card_id', c.id, 'delta', v_launch_quantity))
        from public.cards c
        where c.catalog_version = app.catalog_version()
          and c.token = false -- SPEC loadout rule L3: "no Token-tagged cards"
          -- R1310000: the starter cards alone.
          and coalesce(app.setting('starter_cards'), '[]'::jsonb) ? c.id
          and not exists (
            select 1
            from public.collection_grants g
            where g.profile_id = p_profile_id
              and g.card_id = c.id
              and g.reason = 'launch'
          )
      ),
      '[]'::jsonb
    ),
    'launch',
    'launch:' || app.catalog_version()
  );
end;
$$;

comment on function app.grant_launch_collection(uuid) is
  $$SPEC §14.12, R111, R1310000: grants app.setting('launch_quantity') copies
  of each non-token card of the current catalog version that
  app.setting('starter_cards') names (the starter cards, which seed-catalog
  writes) to one profile, via app.grant_cards with reason = 'launch'. Fired by
  the pending -> active transition (app.on_profile_activated). Idempotent per
  profile: a card already granted with reason 'launch' is skipped.$$;


-- ----------------------------------------------------------------------------
-- 3. A new catalog version grants nothing (R1310001; R481 and R1300001 end).
-- ----------------------------------------------------------------------------
drop trigger if exists settings_grant_catalog_growth on app.settings;
drop function if exists app.on_catalog_version_stamped();
drop function if exists app.grant_launch_collection_all();


-- ----------------------------------------------------------------------------
-- 4. Every economy ledger takes the reason 'launch' (§14.11, R1310002).
-- ----------------------------------------------------------------------------
alter table public.jlore_grants drop constraint if exists jlore_grants_reason_check;
alter table public.jlore_grants add constraint jlore_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'dev', 'launch'));

alter table public.inventory_grants drop constraint if exists inventory_grants_reason_check;
alter table public.inventory_grants add constraint inventory_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
             'chase_shard', 'dev', 'launch'));

alter table public.economy_counter_grants drop constraint if exists economy_counter_grants_reason_check;
alter table public.economy_counter_grants add constraint economy_counter_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
             'chase_shard', 'dev', 'launch'));

alter table public.set_track_grants drop constraint if exists set_track_grants_reason_check;
alter table public.set_track_grants add constraint set_track_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
             'chase_shard', 'dev', 'launch'));

alter table public.quest_grants drop constraint if exists quest_grants_reason_check;
alter table public.quest_grants add constraint quest_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
             'chase_shard', 'dev', 'launch'));

alter table public.cosmetic_grants drop constraint if exists cosmetic_grants_reason_check;
alter table public.cosmetic_grants add constraint cosmetic_grants_reason_check check (
  reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
             'chase_shard', 'dev', 'launch'));


-- ----------------------------------------------------------------------------
-- 5. The launch: every economy at zero, every card kept (R1310002).
-- ----------------------------------------------------------------------------
-- Each holding gets its offsetting ledger row before its delete, in this one
-- transaction (migrate.rs applies a file whole or not at all), so the ledger
-- always sums to what is held (§14.11). Nothing here touches public.collection
-- or public.collection_grants.
do $$
begin
  if exists (select 1 from app.settings where key = 'economy_launched') then
    return;
  end if;

  -- §14.1: the Jlore.
  insert into public.jlore_grants (profile_id, delta, reason, ref)
  select w.profile_id, -w.jlore, 'launch', 'launch:0050'
    from public.wallets w
   where w.jlore > 0;
  delete from public.wallets;

  -- §14.3, §14.5, §14.6: the unopened packs and the unspent tokens.
  insert into public.inventory_grants (profile_id, item, key, delta, reason, ref)
  select i.profile_id, i.item, i.key, -i.count, 'launch', 'launch:0050'
    from public.inventory i
   where i.count > 0;
  delete from public.inventory;

  -- §14.4-§14.6: the pity counters, the token bars, the cosmetic token cycle and
  -- the Chase Shard bar.
  insert into public.economy_counter_grants (profile_id, counter, pack, delta, reason, ref)
  select c.profile_id, c.counter, c.pack, -c.value, 'launch', 'launch:0050'
    from public.economy_counters c
   where c.value > 0;
  delete from public.economy_counters;

  -- §14.7: every track back to its first level (MIRRORS crates/server/src/config.rs
  -- SET_TRACK_FIRST_LEVEL, 1), no XP, nothing paid; every row goes, so only the
  -- newest set's track is active again.
  insert into public.set_track_grants (profile_id, set_id, xp, levels, rewards, reason, ref)
  select t.profile_id, t.set_id, -t.xp, 1 - t.level, -t.rewards_claimed, 'launch', 'launch:0050'
    from public.set_tracks t
   where t.xp <> 0 or t.level <> 1 or t.rewards_claimed <> 0;
  delete from public.set_tracks;

  -- §14.9: the held quests, and when the next is due (a schedule, with no ledger).
  insert into public.quest_grants (profile_id, quest_id, change, reason, ref)
  select q.profile_id, q.quest_id, 'removed', 'launch', 'launch:0050'
    from public.held_quests q;
  delete from public.held_quests;
  delete from public.quest_schedules;

  -- §14.2, R1313000: no account owns a full art at launch.
  insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
  select o.profile_id, o.card_id, o.cosmetic_id, -1, 'launch', 'launch:0050'
    from public.card_cosmetics o
    join public.cosmetics k on k.id = o.cosmetic_id
   where k.kind = 'fullArt';
  delete from public.card_cosmetics o
   using public.cosmetics k
   where k.id = o.cosmetic_id and k.kind = 'fullArt';

  insert into app.settings (key, value) values ('economy_launched', to_jsonb(now()));
end $$;
