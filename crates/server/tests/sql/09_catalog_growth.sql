-- A catalog that grows (migration 0016, R481; ended by 0050, R1310001). Runs after
-- 08_game_records.sql; profiles 1 and 2 are active by then and own CHECK 10's cards under 'core-1'.
\set ON_ERROR_STOP on

-- Same rules as 01-08: every check raises on failure and runs inside a rolled-back transaction.
--
-- Until going live, R481's trigger on the catalog stamp granted every active person each new
-- non-token card. 0050 dropped it (R1310001): a new set's cards come from its track and its packs.
-- So the stamp below must grant nothing, though the new card is on the starter list, which the
-- activation grant reads (R1310000): a grant that rode the stamp would have had a card to give.

\echo '### R481: since going live (R1310001) a new catalog version grants no account its cards ###'
begin;
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  grower  constant uuid := 'cccccccc-cccc-cccc-cccc-cccccccccccc';
  waiting constant uuid := 'cccccccc-0000-0000-0000-00000000000a';
  before_grants bigint;
  before_owned  text;
  n       bigint;
  who     uuid;
  v_owned text;
begin
  -- An account active under the old version, and one still pending.
  insert into auth.users (id, email, email_confirmed_at) values (grower, 'grower@example.test', now());
  update public.profiles set status = 'active', activated_at = now() where id = grower;
  insert into auth.users (id, email, email_confirmed_at) values (waiting, 'waiting@example.test', now());
  if not exists (select 1 from public.collection_grants where profile_id = grower) then
    raise exception 'FAIL (R481): activation granted no cards, so there is nothing a stamp could take back';
  end if;
  select count(*) into before_grants from public.collection_grants;
  select string_agg(profile_id || ' ' || card_id || ' x' || quantity, ', ' order by profile_id, card_id)
    into before_owned from public.collection;

  -- The new card is a starter card, so any grant on the stamp would reach it (the vacuity guard).
  update app.settings set value = app.setting('starter_cards') || '["classic-001"]'::jsonb
   where key = 'starter_cards';

  -- db:seed-catalog's order: every row of the new version, then the stamp, in one transaction.
  update public.cards set catalog_version = 'r481-v2' where catalog_version = app.catalog_version();
  insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
  values ('classic-001', '1', 'Growth Spell', 'Classic', 'Spell', '{Book}', 'Rare', false, '1'::jsonb, 'r481-v2'),
         ('classicplus-012-1', '12.1', 'Growth Token', 'Classic+', 'Spell', '{Pancake,Token}', 'Token', true,
          '0'::jsonb, 'r481-v2');
  update app.settings set value = to_jsonb('r481-v2'::text) where key = 'catalog_version';

  -- No account is granted the new card: not the account active before the stamp, not profile 1,
  -- and not the pending one.
  foreach who in array array[grower, p1, waiting] loop
    select count(*) into n from public.collection_grants
     where profile_id = who and card_id = 'classic-001';
    if n <> 0 then
      raise exception 'FAIL (R1310001): the stamp granted % the new card % time(s)', who, n;
    end if;
  end loop;
  if exists (select 1 from public.collection_grants where card_id = 'classicplus-012-1') then
    raise exception 'FAIL (R481): a token was granted (L3: no Token-tagged cards)';
  end if;

  -- The same stamp again, a round trip through another version, or another key's change grants
  -- nothing either, and nobody loses a card the stamp moved to the new version.
  update app.settings set value = to_jsonb('r481-v2'::text) where key = 'catalog_version';
  update app.settings set value = to_jsonb('r481-other'::text) where key = 'catalog_version';
  update app.settings set value = to_jsonb('r481-v2'::text) where key = 'catalog_version';
  update app.settings set value = to_jsonb(true) where key = 'redemption_enabled';
  select count(*) into n from public.collection_grants;
  if n <> before_grants then
    raise exception 'FAIL (R1310001): % collection grants after the stamps, expected the % before them', n, before_grants;
  end if;
  select string_agg(profile_id || ' ' || card_id || ' x' || quantity, ', ' order by profile_id, card_id)
    into v_owned from public.collection;
  if v_owned is distinct from before_owned then
    raise exception 'FAIL (R1310001): the stamps changed what is owned';
  end if;

  -- The card was grantable all along: the pending account's own activation grants it (R1310000).
  update public.profiles set status = 'active', activated_at = now() where id = waiting;
  select count(*) into n from public.collection_grants
   where profile_id = waiting and card_id = 'classic-001' and reason = 'launch' and ref = 'launch:r481-v2';
  if n <> 1 then
    raise exception 'FAIL (R1310000): activating after the stamp granted the new starter card % time(s), expected 1', n;
  end if;

  raise notice 'OK (R1310001): a new catalog version granted no account its cards, a starter card included; an activation still grants it';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
