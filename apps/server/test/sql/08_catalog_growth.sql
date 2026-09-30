-- A catalog that grows (migration 0015, R481). Runs after 07_retention_purge.sql; profiles 1 and 2
-- are active by then and own CHECK 10's cards under 'core-1'.
\set ON_ERROR_STOP on

-- Same rules as 01-07: every check raises on failure and runs inside a rolled-back transaction.

\echo '=== R481: stamping a new catalog version grants its new cards to every active account, once ==='
begin;
do $$
declare
  grower  constant uuid := 'cccccccc-cccc-cccc-cccc-cccccccccccc';
  waiting constant uuid := 'cccccccc-0000-0000-0000-00000000000a';
  before_core int;
  n       int;
  v_qty   int;
begin
  -- An account active under the old version, and one still pending.
  insert into auth.users (id, email, email_confirmed_at) values (grower, 'grower@example.test', now());
  update public.profiles set status = 'active', activated_at = now() where id = grower;
  insert into auth.users (id, email, email_confirmed_at) values (waiting, 'waiting@example.test', now());
  select count(*) into before_core from public.collection_grants where profile_id = grower;
  if before_core = 0 then
    raise exception 'FAIL (R481): activation granted no cards, so there is nothing to grow from';
  end if;

  -- db:seed-catalog's order: every row of the new version, then the stamp, in one transaction.
  update public.cards set catalog_version = 'r481-v2' where catalog_version = app.catalog_version();
  insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
  values ('classic-001', '1', 'Growth Spell', 'Classic', 'Spell', '{Book}', 'Rare', false, '1'::jsonb, 'r481-v2'),
         ('classicplus-012-1', '12.1', 'Growth Token', 'Classic+', 'Spell', '{Pancake,Token}', 'Token', true,
          '0'::jsonb, 'r481-v2');
  update app.settings set value = to_jsonb('r481-v2'::text) where key = 'catalog_version';

  -- The new card, once, as a launch grant of the new version; the token never.
  select count(*) into n from public.collection_grants
   where profile_id = grower and card_id = 'classic-001' and reason = 'launch' and ref = 'launch:r481-v2';
  if n <> 1 then
    raise exception 'FAIL (R481): % launch grant(s) of the new card, expected exactly 1', n;
  end if;
  select quantity into v_qty from public.collection where profile_id = grower and card_id = 'classic-001';
  if v_qty is distinct from 1 then
    raise exception 'FAIL (R481): the new card''s collection row holds %, expected 1', v_qty;
  end if;
  if exists (select 1 from public.collection_grants where card_id = 'classicplus-012-1') then
    raise exception 'FAIL (R481): a token was granted (L3: no Token-tagged cards)';
  end if;
  -- Cards the account already owned are not granted again.
  select count(*) into n from public.collection_grants where profile_id = grower;
  if n <> before_core + 1 then
    raise exception 'FAIL (R481): % grants after the stamp, expected % (the old ones and the new card)', n, before_core + 1;
  end if;
  -- Every active account grows; a pending one waits for its own activation (R111).
  if not exists (select 1 from public.collection_grants
                  where profile_id = '11111111-1111-1111-1111-111111111111' and card_id = 'classic-001') then
    raise exception 'FAIL (R481): active profile 1 did not receive the new card';
  end if;
  if exists (select 1 from public.collection_grants where profile_id = waiting) then
    raise exception 'FAIL (R481): a pending account was granted cards';
  end if;

  -- The same stamp again, or a round trip through another version, grants nothing twice.
  update app.settings set value = to_jsonb('r481-v2'::text) where key = 'catalog_version';
  update app.settings set value = to_jsonb('r481-other'::text) where key = 'catalog_version';
  update app.settings set value = to_jsonb('r481-v2'::text) where key = 'catalog_version';
  select count(*) into n from public.collection_grants where profile_id = grower;
  if n <> before_core + 1 then
    raise exception 'FAIL (R481): restamping granted again (% grants, expected %)', n, before_core + 1;
  end if;

  -- Another key's change is not a new catalog.
  update app.settings set value = to_jsonb(true) where key = 'redemption_enabled';
  select count(*) into n from public.collection_grants where profile_id = grower;
  if n <> before_core + 1 then
    raise exception 'FAIL (R481): an unrelated setting granted cards';
  end if;

  raise notice 'OK (R481): the new card granted once to every active account, the token never, a restamp nothing';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
