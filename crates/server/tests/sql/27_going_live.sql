-- Going live (migration 0048, SPEC §14.12, §14.2, §14.11; R1310000, R1310001, R1310002; issue #1310).
-- Runs after 26_dev_accounts.sql; profiles 1 and 2 are active by then (03 activated them), profile 1
-- owns all 60 non-Token cards of 'core-1' (01's CHECK 10 and 03's CHECK 2), 03b's legacy profile has
-- owned its 60 'core-0' cards since before run.sh applied 0048, and app.settings.starter_cards is
-- 01's ["core-001","core-002"], written where seed-catalog would write it.
\set ON_ERROR_STOP on

-- Same rules as 01-26: every check raises on failure (`raise exception 'FAIL (…): …'`), each check
-- that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R1310000: a new account owns exactly the starter cards ###'
begin;
-- A starter list naming every kind of id the activation grant must pass over: two non-Token cards of
-- the current version (01's core-001, 03's bulk-001), a Token of it (01's tok-001), a card of another
-- version (03b's legacy-001, of 'core-0') and an id no card has.
update app.settings set value = '["core-001","bulk-001","tok-001","legacy-001","gone-001"]'::jsonb
 where key = 'starter_cards';
insert into public.invite_codes (code_hash, label, max_uses) values ('hash-r1310000', 'r1310000', 1);
insert into auth.users (id, email, email_confirmed_at)
values ('e1310000-0000-4000-8000-000000000001', 'r1310000@example.test', now());
do $$
declare
  newbie   constant uuid := 'e1310000-0000-4000-8000-000000000001';
  version  constant text := app.catalog_version();
  v_status text;
  v_owned  text;
  v_grants text;
  n        bigint;
begin
  -- Vacuity guards: each id on the list is what it stands for, and the catalog holds more
  -- non-Token cards than the two starters, so "exactly the starters" is not "every card".
  select count(*) into n from public.cards
   where id in ('core-001', 'bulk-001') and not token and catalog_version = version;
  if n <> 2 then
    raise exception 'FAIL (R1310000): % of core-001 and bulk-001 are non-Token cards of %', n, version;
  end if;
  if not exists (select 1 from public.cards where id = 'tok-001' and token and catalog_version = version) then
    raise exception 'FAIL (R1310000): tok-001 is no Token of %', version;
  end if;
  if not exists (select 1 from public.cards
                  where id = 'legacy-001' and not token and catalog_version <> version) then
    raise exception 'FAIL (R1310000): legacy-001 is no non-Token card of another version';
  end if;
  if exists (select 1 from public.cards where id = 'gone-001') then
    raise exception 'FAIL (R1310000): gone-001 is a card, so it cannot stand for an id no card has';
  end if;
  select count(*) into n from public.cards where not token and catalog_version = version;
  if n <= 2 then
    raise exception 'FAIL (R1310000): % holds % non-Token cards, so every card and the starters are the same', version, n;
  end if;

  -- A signup is pending and owns nothing (§9.4).
  select status into v_status from public.profiles where id = newbie;
  if v_status is distinct from 'pending' then
    raise exception 'FAIL (R1310000): the new signup is %, not pending', coalesce(v_status, '(no profile)');
  end if;
  if exists (select 1 from public.collection where profile_id = newbie) then
    raise exception 'FAIL (R1310000): a pending account owns cards';
  end if;

  -- The redemption the server makes, and the activation grant it fires (R111).
  v_status := app.redeem_invite_code(newbie, 'hash-r1310000', 'ip-r1310000');
  if v_status <> 'ok' then
    raise exception 'FAIL (R1310000): the redemption returned "%", expected ok', v_status;
  end if;

  -- One copy of each starter card of the current version, and nothing else: not the Token, not
  -- the other version's card, not the id no card has, not the rest of the catalog.
  select string_agg(card_id || ' x' || quantity, ', ' order by card_id) into v_owned
    from public.collection where profile_id = newbie;
  if v_owned is distinct from 'bulk-001 x1, core-001 x1' then
    raise exception 'FAIL (R1310000): the new account owns [%], expected [bulk-001 x1, core-001 x1]',
      coalesce(v_owned, '(nothing)');
  end if;
  -- Each written to the ledger as the launch (§14.11), no dev tool's.
  select string_agg(format('%s %s %s %s dev=%s', card_id, delta, reason, ref, dev::text), ', ' order by card_id)
    into v_grants from public.collection_grants where profile_id = newbie;
  if v_grants is distinct from format('bulk-001 1 launch launch:%1$s dev=false, core-001 1 launch launch:%1$s dev=false',
                                      version) then
    raise exception 'FAIL (R1310000): the new account''s ledger is [%], expected one launch row per starter card',
      coalesce(v_grants, '(empty)');
  end if;

  -- §14.2: the starter cosmetics are owned without a row, so the activation writes none.
  select (select count(*) from public.card_cosmetics where profile_id = newbie)
       + (select count(*) from public.account_cosmetics where profile_id = newbie)
       + (select count(*) from public.cosmetic_grants where profile_id = newbie)
    into n;
  if n <> 0 then
    raise exception 'FAIL (§14.2): the activation wrote % cosmetic row(s); the starter cosmetics need none', n;
  end if;

  -- A re-activation, a ban lifted, and the grant called again add nothing (R111's idempotency).
  update public.profiles set status = 'banned' where id = newbie;
  update public.profiles set status = 'active' where id = newbie;
  perform app.grant_launch_collection(newbie);
  select string_agg(card_id || ' x' || quantity, ', ' order by card_id) into v_owned
    from public.collection where profile_id = newbie;
  select string_agg(format('%s %s %s %s dev=%s', card_id, delta, reason, ref, dev::text), ', ' order by card_id)
    into v_status from public.collection_grants where profile_id = newbie;
  if v_owned is distinct from 'bulk-001 x1, core-001 x1' or v_status is distinct from v_grants then
    raise exception 'FAIL (R1310000): a re-activation changed the account to [%] with ledger [%]', v_owned, v_status;
  end if;

  raise notice 'OK (R1310000): a new account owns one copy of each starter card of the catalog, each a launch row, and a re-activation adds nothing';
end $$;
rollback;

\echo '### R1310001: a new catalog version grants no account its cards ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e1310000-0000-4000-8000-000000000011', 'r1310001-active@example.test', now()),
       ('e1310000-0000-4000-8000-000000000012', 'r1310001-pending@example.test', now());
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  active  constant uuid := 'e1310000-0000-4000-8000-000000000011';
  waiting constant uuid := 'e1310000-0000-4000-8000-000000000012';
  bot     constant uuid := 'e1310000-0000-4000-8000-000000000013';
  before_grants bigint;
  before_owned  text;
  v_text  text;
  n       bigint;
begin
  -- 0048 dropped the stamp's grant (0016, R481): its trigger and both of its functions. The only
  -- trigger left on app.settings is 0001's updated_at.
  select string_agg(tgname, ', ' order by tgname) into v_text
    from pg_trigger where tgrelid = 'app.settings'::regclass and not tgisinternal;
  if v_text is distinct from 'settings_set_updated_at' then
    raise exception 'FAIL (R1310001): app.settings has the triggers [%], expected settings_set_updated_at alone', v_text;
  end if;
  if to_regprocedure('app.on_catalog_version_stamped()') is not null
     or to_regprocedure('app.grant_launch_collection_all()') is not null then
    raise exception 'FAIL (R1310001): app.on_catalog_version_stamped or app.grant_launch_collection_all still exists';
  end if;

  -- An account active under the old version, one still pending, and a ladder bot.
  update public.profiles set status = 'active', activated_at = now() where id = active;
  insert into public.profiles (id, kind, status) values (bot, 'bot', 'active');
  if not exists (select 1 from public.collection_grants where profile_id = active) then
    raise exception 'FAIL (R1310001): the activation granted no starter card, so a stamp has nothing to keep';
  end if;
  select count(*) into before_grants from public.collection_grants;
  select string_agg(profile_id || ' ' || card_id || ' x' || quantity, ', ' order by profile_id, card_id)
    into before_owned from public.collection;

  -- The new card is a starter card, so any grant riding the stamp would have it to give.
  update app.settings set value = app.setting('starter_cards') || '["r1310001-new"]'::jsonb
   where key = 'starter_cards';

  -- db:seed-catalog's order: every row of the new version, then the stamp, in one transaction.
  update public.cards set catalog_version = 'r1310001-v2' where catalog_version = app.catalog_version();
  insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
  values ('r1310001-new', '1310', 'New Set Card', 'Classic', 'Spell', '{}', 'Rare', false, '1'::jsonb, 'r1310001-v2');
  update app.settings set value = to_jsonb('r1310001-v2'::text) where key = 'catalog_version';
  -- The same stamp again and a round trip through another version.
  update app.settings set value = to_jsonb('r1310001-v2'::text) where key = 'catalog_version';
  update app.settings set value = to_jsonb('r1310001-other'::text) where key = 'catalog_version';
  update app.settings set value = to_jsonb('r1310001-v2'::text) where key = 'catalog_version';

  select count(*) into n from public.collection_grants where card_id = 'r1310001-new';
  if n <> 0 then
    raise exception 'FAIL (R1310001): the stamp granted the new card % time(s)', n;
  end if;
  select count(*) into n from public.collection_grants;
  if n <> before_grants then
    raise exception 'FAIL (R1310001): % collection grants after the stamps, expected the % before them', n, before_grants;
  end if;
  -- And nobody loses a card the new version moved: profile 1, the account active before the stamp
  -- and 03b's legacy account own what they owned.
  select string_agg(profile_id || ' ' || card_id || ' x' || quantity, ', ' order by profile_id, card_id)
    into v_text from public.collection;
  if v_text is distinct from before_owned then
    raise exception 'FAIL (R1310001): the stamps changed what is owned';
  end if;
  if not exists (select 1 from public.collection where profile_id = p1) then
    raise exception 'FAIL (R1310001): profile 1 owns nothing, so "nobody lost a card" was measured over nobody';
  end if;

  -- The new card was grantable all along: the pending account's own activation grants it, with
  -- the starters the stamp moved, under the new version (R1310000).
  update public.profiles set status = 'active', activated_at = now() where id = waiting;
  select string_agg(card_id || ' ' || ref, ', ' order by card_id) into v_text
    from public.collection_grants where profile_id = waiting and reason = 'launch';
  if v_text is distinct from 'core-001 launch:r1310001-v2, core-002 launch:r1310001-v2, r1310001-new launch:r1310001-v2' then
    raise exception 'FAIL (R1310000): the activation after the stamp granted [%], expected the three starter cards of the new version',
      coalesce(v_text, '(nothing)');
  end if;
  if exists (select 1 from public.collection_grants where profile_id = bot) then
    raise exception 'FAIL (R1454): the ladder bot holds collection grants';
  end if;

  raise notice 'OK (R1310001): the stamp has no trigger, and a new version granted no account its cards, a starter card included';
end $$;
rollback;

\echo '### R1310002: an existing account keeps every card and starts its economy at zero ###'
begin;
-- public.cosmetics is the registry's projection, which seed-catalog writes and this suite does not:
-- a full art, a foil and an account cosmetic stand in for it.
insert into public.cosmetics (id, kind, name, rarity)
values ('test-full-art', 'fullArt', 'Test full art', 'Epic'),
       ('test-foil', 'foil', 'Test foil', 'Rare'),
       ('test-table', 'table', 'Test table', 'Epic');

do $$
declare
  legacy constant uuid := '44444444-4444-4444-4444-444444444444';
  n bigint;
  t text;
begin
  -- The launch itself ran when run.sh applied 0048: its marker is set, and 03b's account, which
  -- owned its cards before that, still owns all 60 with their 60 launch rows.
  if not exists (select 1 from app.settings where key = 'economy_launched') then
    raise exception 'FAIL (R1310002): app.settings.economy_launched is unset after 0048 applied';
  end if;
  select count(*) into n from public.collection
   where profile_id = legacy and quantity = 1 and card_id like 'legacy-%';
  if n <> 60 then
    raise exception 'FAIL (R1310002): 03b''s account owns % of the 60 cards it held before 0048', n;
  end if;
  select count(*) into n from public.collection_grants
   where profile_id = legacy and reason = 'launch' and delta = 1 and card_id like 'legacy-%';
  if n <> 60 then
    raise exception 'FAIL (R1310002): 03b''s account holds % of its 60 launch rows', n;
  end if;

  -- Vacuity guard for the ledger sums below: 01-26 left no economy row behind.
  foreach t in array array['wallets', 'jlore_grants', 'inventory', 'inventory_grants', 'economy_counters',
                           'economy_counter_grants', 'set_tracks', 'set_track_grants', 'held_quests',
                           'quest_schedules', 'quest_grants', 'card_cosmetics', 'account_cosmetics',
                           'cosmetic_grants'] loop
    execute format('select count(*) from public.%I', t) into n;
    if n <> 0 then
      raise exception 'FAIL (R1310002): public.% holds % row(s) before the seed', t, n;
    end if;
  end loop;
end $$;

-- Profile 1's economy as the store writes it, every holding beside the ledger rows that sum to it,
-- and profile 2's: a wallet spent to zero and a full art of its own.
insert into public.wallets (profile_id, jlore)
values ('11111111-1111-1111-1111-111111111111', 120), ('22222222-2222-2222-2222-222222222222', 0);
insert into public.jlore_grants (profile_id, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 100, 'game'),
       ('11111111-1111-1111-1111-111111111111', 20, 'quest'),
       ('22222222-2222-2222-2222-222222222222', 7, 'game'),
       ('22222222-2222-2222-2222-222222222222', -7, 'pack_purchase');
insert into public.inventory (profile_id, item, key, count)
values ('11111111-1111-1111-1111-111111111111', 'set_pack', 'Core', 2),
       ('11111111-1111-1111-1111-111111111111', 'card_token', 'Rare', 1);
insert into public.inventory_grants (profile_id, item, key, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 'set_pack', 'Core', 3, 'pack_purchase'),
       ('11111111-1111-1111-1111-111111111111', 'set_pack', 'Core', -1, 'pack_open'),
       ('11111111-1111-1111-1111-111111111111', 'card_token', 'Rare', 1, 'token_bar');
insert into public.economy_counters (profile_id, counter, pack, value)
values ('11111111-1111-1111-1111-111111111111', 'pity_epic', '', 3),
       ('11111111-1111-1111-1111-111111111111', 'card_token_bar', '', 40);
insert into public.economy_counter_grants (profile_id, counter, pack, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 'pity_epic', '', 3, 'pack_open'),
       ('11111111-1111-1111-1111-111111111111', 'card_token_bar', '', 40, 'pack_open');
-- Core's track at level 3 with both levels paid, and Classic's at its first level with nothing.
insert into public.set_tracks (profile_id, set_id, xp, level, rewards_claimed)
values ('11111111-1111-1111-1111-111111111111', 'Core', 250, 3, 2),
       ('11111111-1111-1111-1111-111111111111', 'Classic', 0, 1, 0);
insert into public.set_track_grants (profile_id, set_id, xp, levels, rewards, reason)
values ('11111111-1111-1111-1111-111111111111', 'Core', 250, 2, 0, 'game'),
       ('11111111-1111-1111-1111-111111111111', 'Core', 0, 0, 2, 'track');
insert into public.held_quests (profile_id, quest_id, progress, arrived_at)
values ('11111111-1111-1111-1111-111111111111', 'win-games', 1, now());
insert into public.quest_schedules (profile_id, next_due_at)
values ('11111111-1111-1111-1111-111111111111', now() + interval '1 hour');
insert into public.quest_grants (profile_id, quest_id, change, progress, reason)
values ('11111111-1111-1111-1111-111111111111', 'win-games', 'arrived', 0, 'quest'),
       ('11111111-1111-1111-1111-111111111111', 'win-games', 'progressed', 1, 'game');
insert into public.card_cosmetics (profile_id, card_id, cosmetic_id)
values ('11111111-1111-1111-1111-111111111111', 'core-001', 'test-full-art'),
       ('11111111-1111-1111-1111-111111111111', 'core-001', 'test-foil'),
       ('22222222-2222-2222-2222-222222222222', 'core-002', 'test-full-art');
insert into public.account_cosmetics (profile_id, cosmetic_id)
values ('11111111-1111-1111-1111-111111111111', 'test-table');
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason)
values ('11111111-1111-1111-1111-111111111111', 'core-001', 'test-full-art', 1, 'pack_open'),
       ('11111111-1111-1111-1111-111111111111', 'core-001', 'test-foil', 1, 'pack_open'),
       ('11111111-1111-1111-1111-111111111111', null, 'test-table', 1, 'token_redeem'),
       ('22222222-2222-2222-2222-222222222222', 'core-002', 'test-full-art', 1, 'pack_open');

create temp table r1310002_rows (tbl text primary key, n bigint not null) on commit drop;
do $$
declare
  t text;
  n bigint;
begin
  foreach t in array array['wallets', 'jlore_grants', 'inventory', 'inventory_grants', 'economy_counters',
                           'economy_counter_grants', 'set_tracks', 'set_track_grants', 'held_quests',
                           'quest_schedules', 'quest_grants', 'card_cosmetics', 'account_cosmetics',
                           'cosmetic_grants', 'collection', 'collection_grants'] loop
    execute format('select count(*) from public.%I', t) into n;
    insert into r1310002_rows values (t, n);
  end loop;
  perform set_config('r1310002.collection',
    (select string_agg(profile_id || ' ' || card_id || ' x' || quantity, ', ' order by profile_id, card_id)
       from public.collection), true);
end $$;

-- Applied again over a database that has gone live, 0048 resets nothing: the launch runs once.
set local client_min_messages = warning;
\i /tmp/0048_going_live.sql
set local client_min_messages = notice;
do $$
declare
  r record;
  n bigint;
begin
  for r in select x.tbl, x.n as before from r1310002_rows x loop
    execute format('select count(*) from public.%I', r.tbl) into n;
    if n <> r.before then
      raise exception 'FAIL (R1310002): re-applying 0048 after the launch changed public.% from % rows to %',
        r.tbl, r.before, n;
    end if;
  end loop;
  raise notice 'OK (R1310002): re-applied after the launch, 0048 leaves every holding as it was';
end $$;

-- The launch, as it ran on the live database: its marker deleted, 0048 applied over these holdings.
delete from app.settings where key = 'economy_launched';
set local client_min_messages = warning;
\i /tmp/0048_going_live.sql
set local client_min_messages = notice;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  t  text;
  n  bigint;
  v_text text;
  ledgers constant text[] := array['jlore_grants', 'inventory_grants', 'economy_counter_grants',
                                   'set_track_grants', 'quest_grants', 'cosmetic_grants'];
begin
  -- Every card kept: the collection and its ledger are as they were.
  select string_agg(profile_id || ' ' || card_id || ' x' || quantity, ', ' order by profile_id, card_id)
    into v_text from public.collection;
  if v_text is distinct from current_setting('r1310002.collection') then
    raise exception 'FAIL (R1310002): the launch changed what is owned';
  end if;
  select count(*) into n from public.collection where profile_id = p1;
  if n <> 60 then
    raise exception 'FAIL (R1310002): profile 1 owns % of its 60 cards after the launch', n;
  end if;
  select count(*) into n from public.collection_grants;
  if n <> (select r.n from r1310002_rows r where r.tbl = 'collection_grants') then
    raise exception 'FAIL (R1310002): the launch wrote % collection grants, expected none', n;
  end if;

  -- Every economy holding gone, but the cosmetics that are no full art.
  foreach t in array array['wallets', 'inventory', 'economy_counters', 'set_tracks', 'held_quests',
                           'quest_schedules'] loop
    execute format('select count(*) from public.%I', t) into n;
    if n <> 0 then
      raise exception 'FAIL (R1310002): public.% still holds % row(s) after the launch', t, n;
    end if;
  end loop;
  select string_agg(profile_id || ' ' || card_id || ' ' || cosmetic_id, ', ' order by profile_id, card_id, cosmetic_id)
    into v_text from public.card_cosmetics;
  if v_text is distinct from p1 || ' core-001 test-foil' then
    raise exception 'FAIL (R1310002): the per-card cosmetics read [%] after the launch, expected profile 1''s foil alone',
      coalesce(v_text, '(none)');
  end if;
  select string_agg(profile_id || ' ' || cosmetic_id, ', ') into v_text from public.account_cosmetics;
  if v_text is distinct from p1 || ' test-table' then
    raise exception 'FAIL (R1310002): the account cosmetics read [%] after the launch, expected profile 1''s table',
      coalesce(v_text, '(none)');
  end if;
  -- §14.2, R1313000: no account owns a full art.
  if exists (select 1 from public.card_cosmetics o join public.cosmetics k on k.id = o.cosmetic_id
              where k.kind = 'fullArt') then
    raise exception 'FAIL (R1310002): an account owns a full art after the launch';
  end if;

  -- One launch row for each holding zeroed, and none for a holding that was already at zero (profile
  -- 2's wallet, the Classic track) or a cosmetic kept.
  select string_agg(format('%s %s', profile_id, delta), ', ' order by profile_id) into v_text
    from public.jlore_grants where reason = 'launch';
  if v_text is distinct from format('%s -120', p1) then
    raise exception 'FAIL (R1310002): the Jlore launch rows are [%], expected profile 1''s -120', v_text;
  end if;
  select string_agg(format('%s %s/%s %s', profile_id, item, key, delta), ', ' order by item, key) into v_text
    from public.inventory_grants where reason = 'launch';
  if v_text is distinct from format('%1$s card_token/Rare -1, %1$s set_pack/Core -2', p1) then
    raise exception 'FAIL (R1310002): the inventory launch rows are [%]', v_text;
  end if;
  select string_agg(format('%s %s/%s %s', profile_id, counter, pack, delta), ', ' order by counter) into v_text
    from public.economy_counter_grants where reason = 'launch';
  if v_text is distinct from format('%1$s card_token_bar/ -40, %1$s pity_epic/ -3', p1) then
    raise exception 'FAIL (R1310002): the counter launch rows are [%]', v_text;
  end if;
  select string_agg(format('%s %s xp=%s levels=%s rewards=%s', profile_id, set_id, xp, levels, rewards), ', ')
    into v_text from public.set_track_grants where reason = 'launch';
  if v_text is distinct from format('%s Core xp=-250 levels=-2 rewards=-2', p1) then
    raise exception 'FAIL (R1310002): the track launch rows are [%], expected Core''s alone back to its first level', v_text;
  end if;
  select string_agg(format('%s %s %s', profile_id, quest_id, change), ', ') into v_text
    from public.quest_grants where reason = 'launch';
  if v_text is distinct from format('%s win-games removed', p1) then
    raise exception 'FAIL (R1310002): the quest launch rows are [%]', v_text;
  end if;
  select string_agg(format('%s %s %s %s', profile_id, card_id, cosmetic_id, delta), ', ' order by profile_id)
    into v_text from public.cosmetic_grants where reason = 'launch';
  if v_text is distinct from format('%s core-001 test-full-art -1, %s core-002 test-full-art -1', p1, p2) then
    raise exception 'FAIL (R1310002): the cosmetic launch rows are [%], expected one per full art and none for the foil or the table',
      v_text;
  end if;
  -- Each says it was the launch, and no dev tool wrote it (§14.11).
  foreach t in array ledgers loop
    execute format($q$select count(*) from public.%I
                       where reason = 'launch' and (ref is distinct from 'launch:0048' or dev)$q$, t) into n;
    if n <> 0 then
      raise exception 'FAIL (R1310002): % launch row(s) of public.% are not ref launch:0048 or are marked dev', n, t;
    end if;
  end loop;

  -- The ledger still sums to what is held (§14.11): to nothing, but the cosmetics kept.
  if exists (select 1 from public.jlore_grants group by profile_id having sum(delta) <> 0) then
    raise exception 'FAIL (R1310002): a Jlore ledger sums to more than the empty wallet';
  end if;
  if exists (select 1 from public.inventory_grants group by profile_id, item, key having sum(delta) <> 0) then
    raise exception 'FAIL (R1310002): an inventory ledger sums to more than the empty inventory';
  end if;
  if exists (select 1 from public.economy_counter_grants group by profile_id, counter, pack having sum(delta) <> 0) then
    raise exception 'FAIL (R1310002): a counter ledger sums to more than the empty counter';
  end if;
  if exists (select 1 from public.set_track_grants group by profile_id, set_id
              having sum(xp) <> 0 or sum(levels) <> 0 or sum(rewards) <> 0) then
    raise exception 'FAIL (R1310002): a track ledger sums to more than a track at its first level';
  end if;
  if exists (select 1 from public.quest_grants group by profile_id, quest_id
              having count(*) filter (where change = 'arrived')
                     <> count(*) filter (where change in ('completed', 'removed'))) then
    raise exception 'FAIL (R1310002): a quest ledger still holds a quest';
  end if;
  select string_agg(format('%s %s %s', g.profile_id, g.card_id, g.cosmetic_id), ', ') into v_text
    from (select profile_id, card_id, cosmetic_id from public.cosmetic_grants
           group by profile_id, card_id, cosmetic_id having sum(delta) <> 0) g
   where not exists (select 1 from public.card_cosmetics o
                      where o.profile_id = g.profile_id and o.card_id = g.card_id and o.cosmetic_id = g.cosmetic_id)
     and not exists (select 1 from public.account_cosmetics o
                      where g.card_id is null and o.profile_id = g.profile_id and o.cosmetic_id = g.cosmetic_id);
  if v_text is not null then
    raise exception 'FAIL (R1310002): the cosmetic ledger holds [%], which no account owns', v_text;
  end if;

  if not exists (select 1 from app.settings where key = 'economy_launched') then
    raise exception 'FAIL (R1310002): the launch did not mark itself done';
  end if;
  raise notice 'OK (R1310002): every card kept, every economy holding and full art zeroed by a launch row, the foil and the table kept';
end $$;

-- And applied once more, it writes nothing: the launch marked itself done.
create temp table r1310002_launch (tbl text primary key, n bigint not null) on commit drop;
insert into r1310002_launch
select 'jlore_grants', count(*) from public.jlore_grants
union all select 'inventory_grants', count(*) from public.inventory_grants
union all select 'economy_counter_grants', count(*) from public.economy_counter_grants
union all select 'set_track_grants', count(*) from public.set_track_grants
union all select 'quest_grants', count(*) from public.quest_grants
union all select 'cosmetic_grants', count(*) from public.cosmetic_grants
union all select 'card_cosmetics', count(*) from public.card_cosmetics
union all select 'account_cosmetics', count(*) from public.account_cosmetics;
set local client_min_messages = warning;
\i /tmp/0048_going_live.sql
set local client_min_messages = notice;
do $$
declare
  r record;
  n bigint;
begin
  for r in select x.tbl, x.n as before from r1310002_launch x loop
    execute format('select count(*) from public.%I', r.tbl) into n;
    if n <> r.before then
      raise exception 'FAIL (R1310002): a second apply changed public.% from % rows to %', r.tbl, r.before, n;
    end if;
  end loop;
  raise notice 'OK (R1310002): 0048 re-applies over its own launch and writes nothing';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
