-- Dev accounts (migration 0046, SPEC §14.10, §14.11; issue #1300). Runs after 25_economy.sql;
-- profiles 1 and 2 are active by then (03 activated them), and core-001 is in public.cards (01's
-- CHECK 10).
\set ON_ERROR_STOP on

-- Same rules as 01-25: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a guard,
-- and every block runs inside a transaction that is rolled back.

\echo '### 0046: the migration re-applies over itself ###'
begin;
-- What run.sh's apply_migration filters, a re-run's "does not exist, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0046_dev_accounts.sql
\i /tmp/0046_dev_accounts.sql
set local client_min_messages = notice;
do $$
begin
  if not exists (select 1 from pg_constraint where conname = 'collection_grants_dev_check') then
    raise exception 'FAIL (0046): collection_grants_dev_check is gone after a re-apply';
  end if;
  raise notice 'OK (0046): the migration re-applies over itself';
end $$;
rollback;

\echo '=== R1300001: since going live (R1310001) a new catalog version grants nobody, dev account or not ==='
-- R1300001's pass-over is moot since 0050 dropped the stamp's grant: no account is granted a new
-- version's cards, so what a dev account cleared (R1300000) stays cleared whether or not the stamp
-- is handed the dev accounts. The activation grant (R111) treats a dev account like anyone.
begin;
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  dev_id  constant uuid := 'd0000000-0000-4000-8000-000000001300';
  player  constant uuid := 'd0000000-0000-4000-8000-000000001301';
  bot     constant uuid := 'd0000000-0000-4000-8000-000000001302';
  before_grants bigint;
  n       int;
begin
  -- A dev account and a player, both active under the old version, and a ladder bot (R1454).
  insert into auth.users (id, email, email_confirmed_at) values (dev_id, 'dev@example.test', now());
  update public.profiles set status = 'active', activated_at = now() where id = dev_id;
  insert into auth.users (id, email, email_confirmed_at) values (player, 'player@example.test', now());
  update public.profiles set status = 'active', activated_at = now() where id = player;
  insert into public.profiles (id, kind, status) values (bot, 'bot', 'active');
  -- Vacuity guard: R111's activation granted the dev account its starter cards like anyone.
  if not exists (select 1 from public.collection_grants where profile_id = dev_id and reason = 'launch') then
    raise exception 'FAIL (R1300001): activation granted the dev account nothing, unlike anyone else';
  end if;
  select count(*) into before_grants from public.collection_grants;

  -- The new card is a starter card, so a grant on the stamp would have had it to give.
  update app.settings set value = app.setting('starter_cards') || '["classic-001"]'::jsonb
   where key = 'starter_cards';

  -- db:seed-catalog's order as it was: the dev accounts for this transaction, every row of the new
  -- version, then the stamp.
  perform set_config('jackioh.dev_profile_ids', dev_id::text, true);
  update public.cards set catalog_version = 'r1300001-v2' where catalog_version = app.catalog_version();
  insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
  values ('classic-001', '1', 'Growth Spell', 'Classic', 'Spell', '{Book}', 'Rare', false, '1'::jsonb,
          'r1300001-v2');
  update app.settings set value = to_jsonb('r1300001-v2'::text) where key = 'catalog_version';

  -- And a stamp handed no dev accounts.
  perform set_config('jackioh.dev_profile_ids', '', true);
  update public.cards set catalog_version = 'r1300001-v3' where catalog_version = 'r1300001-v2';
  update app.settings set value = to_jsonb('r1300001-v3'::text) where key = 'catalog_version';

  select count(*) into n from public.collection_grants
   where card_id = 'classic-001' and profile_id in (player, p1, dev_id, bot);
  if n <> 0 then
    raise exception 'FAIL (R1300001): the stamps granted the new card % time(s) to the player, profile 1 or the dev account', n;
  end if;
  select count(*) into n from public.collection_grants;
  if n <> before_grants then
    raise exception 'FAIL (R1310001): % collection grants after the stamps, expected the % before them', n, before_grants;
  end if;
  if exists (select 1 from public.collection_grants where profile_id = bot) then
    raise exception 'FAIL (R1454): the ladder bot holds collection grants';
  end if;
  raise notice 'OK (R1300001): no stamp granted anyone the new card, the dev account included, whether it was handed the dev accounts or not';
end $$;
rollback;

\echo '=== R1300000: collection_grants takes the reason dev only marked dev ==='
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
begin
  -- Vacuity guard: a marked dev row is taken.
  insert into public.collection_grants (profile_id, card_id, delta, reason, dev)
  values (p1, 'core-001', -1, 'dev', true);
  begin
    insert into public.collection_grants (profile_id, card_id, delta, reason, dev)
    values (p1, 'core-001', 1, 'dev', false);
    raise exception 'FAIL (R1300000): a dev-reason collection grant without the dev mark was written';
  exception
    when check_violation then
      if sqlerrm not like '%collection_grants_dev_check%' then
        raise exception 'FAIL (R1300000): the refusal "%" does not name collection_grants_dev_check', sqlerrm;
      end if;
  end;
  raise notice 'OK (R1300000): a dev-reason collection grant is taken marked dev and refused unmarked';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
