-- Standard deck size 25 (migration 0055, SPEC §2.6, §9.4 D2, §14.12; R1705000, R1705001, R1705002;
-- issue #1705). Runs after 30_seat_epoch.sql; profile 1 is active and owns all 75 non-Token cards of
-- 'core-1' (01's CHECK 10 and 03's CHECK 2), and app.settings.starter_cards is 01's
-- ["core-001","core-002"], so 0055's union and grant found nothing to do when run.sh applied it: the
-- R1705002 checks apply the file again (/tmp/0055_deck_size_25.sql, which run.sh copied there) over
-- rows that need it.
\set ON_ERROR_STOP on

-- Same rules as 01-30: every check raises on failure (`raise exception 'FAIL (…): …'`), each check
-- that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R1705000: deck_size is 25; a 25-card deck saves and a 26-card one fails D2 ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  r  text;
  n  bigint;
begin
  if app.setting('deck_size') is distinct from to_jsonb(25) then
    raise exception 'FAIL (R1705000): app.settings.deck_size reads %, expected 25',
      coalesce(app.setting('deck_size')::text, '(no row)');
  end if;
  -- Vacuity guard: the bulk cards the decks below name are cards of the catalog.
  select count(*) into n from public.cards
   where id in (select format('bulk-%s', to_char(k, 'FM000')) from generate_series(1, 26) k)
     and catalog_version = 'core-1' and not token;
  if n <> 26 then
    raise exception 'FAIL (R1705000): % of bulk-001..bulk-026 are non-Token cards of core-1', n;
  end if;

  r := app.upsert_deck(p1, 'a1705000-0000-4000-8000-000000000001', 'Twenty-five',
         (select jsonb_agg(format('bulk-%s', to_char(k, 'FM000')) order by k) from generate_series(1, 25) k),
         'core-1', now(), 100);
  if r is distinct from 'created' then
    raise exception 'FAIL (R1705000): a 25-card deck answered %, expected created', r;
  end if;
  select jsonb_array_length(cards) into n from public.decks
   where id = 'a1705000-0000-4000-8000-000000000001';
  if n is distinct from 25 then
    raise exception 'FAIL (R1705000): the saved deck holds % cards, expected 25', n;
  end if;

  begin
    perform app.upsert_deck(p1, 'a1705000-0000-4000-8000-000000000002', 'Twenty-six',
      (select jsonb_agg(format('bulk-%s', to_char(k, 'FM000')) order by k) from generate_series(1, 26) k),
      'core-1', now(), 100);
    raise exception 'FAIL (R1705000): a 26-card deck saved (D2)';
  exception when others then
    if sqlerrm like 'FAIL%' then raise; end if;
    if sqlerrm <> 'deck: D2 26 cards, at most 25 allowed' then
      raise exception 'FAIL (R1705000): the 26-card deck raised "%" (%), expected D2''s refusal', sqlerrm, sqlstate;
    end if;
  end;
  if exists (select 1 from public.decks where id = 'a1705000-0000-4000-8000-000000000002') then
    raise exception 'FAIL (R1705000): the refused 26-card deck wrote a row';
  end if;

  raise notice 'OK (R1705000): deck_size is 25; a 25-card deck saves and a 26-card one is refused by D2';
end $$;
rollback;

\echo '### R1705001: a 20-card deck still saves, as a draft ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  r  text;
  n  bigint;
begin
  -- A deck saved under 20-card decks is a draft now (D2 is "at most"): saving it again, as a
  -- player renaming it would, keeps its 20 cards. Queueing it is refused by the server's L2.
  r := app.upsert_deck(p1, 'a1705001-0000-4000-8000-000000000001', 'Old twenty',
         (select jsonb_agg(format('bulk-%s', to_char(k, 'FM000')) order by k) from generate_series(1, 20) k),
         'core-1', now(), 100);
  if r is distinct from 'created' then
    raise exception 'FAIL (R1705001): a 20-card deck answered %, expected created', r;
  end if;
  r := app.upsert_deck(p1, 'a1705001-0000-4000-8000-000000000001', 'Old twenty, renamed',
         (select cards from public.decks where id = 'a1705001-0000-4000-8000-000000000001'),
         'core-1', now(), 100);
  if r is distinct from 'updated' then
    raise exception 'FAIL (R1705001): saving the 20-card deck again answered %, expected updated', r;
  end if;
  select jsonb_array_length(cards) into n from public.decks
   where id = 'a1705001-0000-4000-8000-000000000001';
  if n is distinct from 20 then
    raise exception 'FAIL (R1705001): the saved draft holds % cards, expected its 20', n;
  end if;

  raise notice 'OK (R1705001): a 20-card deck saves and saves again unchanged, a draft';
end $$;
rollback;

\echo '### R1705002: 0055 adds the eleven starter cards and grants them to every active account once ###'
begin;
-- The eleven, as cards of the catalog (01 and 03 seeded none of them), and a starter list as it
-- stood before 0055: 01's two.
insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
select s.id, (1000 + row_number() over (order by s.id))::text, 'Starter ' || s.id, 'Core', 'Unit',
       '{}'::text[], 'Common', false, to_jsonb(1), 'core-1'
  from unnest(array['core-002', 'core-003', 'core-010', 'core-031', 'core-034', 'core-043', 'core-047',
                    'core-072', 'core-080', 'core-089', 'core-091']) as s(id)
 where not exists (select 1 from public.cards c where c.id = s.id);
-- An account that went live under the old list, one still pending and a ladder bot.
insert into public.invite_codes (code_hash, label, max_uses) values ('hash-r1705002', 'r1705002', 1);
insert into auth.users (id, email, email_confirmed_at)
values ('e1705002-0000-4000-8000-000000000001', 'r1705002-active@example.test', now()),
       ('e1705002-0000-4000-8000-000000000002', 'r1705002-pending@example.test', now());
do $$
declare
  newbie constant uuid := 'e1705002-0000-4000-8000-000000000001';
  bot    constant uuid := 'e1705002-0000-4000-8000-000000000003';
  v_text text;
begin
  insert into public.profiles (id, kind, status) values (bot, 'bot', 'active');
  v_text := app.redeem_invite_code(newbie, 'hash-r1705002', 'ip-r1705002');
  if v_text <> 'ok' then
    raise exception 'FAIL (R1705002): the redemption returned "%", expected ok', v_text;
  end if;
  select string_agg(card_id, ', ' order by card_id) into v_text
    from public.collection_grants where profile_id = newbie and reason = 'launch';
  if v_text is distinct from 'core-001, core-002' then
    raise exception 'FAIL (R1705002): before 0055 the account''s launch rows are [%], expected 01''s two',
      coalesce(v_text, '(none)');
  end if;
end $$;

\i /tmp/0055_deck_size_25.sql

do $$
declare
  newbie  constant uuid := 'e1705002-0000-4000-8000-000000000001';
  waiting constant uuid := 'e1705002-0000-4000-8000-000000000002';
  bot     constant uuid := 'e1705002-0000-4000-8000-000000000003';
  added   constant text[] := array['core-003', 'core-010', 'core-031', 'core-034', 'core-043', 'core-047',
                                   'core-072', 'core-080', 'core-089', 'core-091'];
  v_text  text;
  n       bigint;
begin
  -- The list is the sorted union: 01's two (core-002 among the eleven, held once) and the rest.
  if app.setting('starter_cards') is distinct from
     (select jsonb_agg(id order by id) from unnest(array['core-001', 'core-002'] || added) as id) then
    raise exception 'FAIL (R1705002): starter_cards reads % after 0055', app.setting('starter_cards')::text;
  end if;

  -- The account owns one copy of each, each a launch row, and its old rows are as they were.
  select string_agg(card_id || ' x' || quantity, ', ' order by card_id) into v_text
    from public.collection where profile_id = newbie;
  if v_text is distinct from (select string_agg(id || ' x1', ', ' order by id)
                                from unnest(array['core-001', 'core-002'] || added) as id) then
    raise exception 'FAIL (R1705002): after 0055 the account owns [%]', coalesce(v_text, '(nothing)');
  end if;
  select count(*) into n from public.collection_grants where profile_id = newbie and reason = 'launch';
  if n <> 12 then
    raise exception 'FAIL (R1705002): the account holds % launch rows, expected 12', n;
  end if;

  -- Profile 1 owned every card it could already; it gains the ten new ids it lacked.
  select count(*) into n from public.collection
   where profile_id = '11111111-1111-1111-1111-111111111111' and card_id = any (added);
  if n <> 10 then
    raise exception 'FAIL (R1705002): profile 1 owns % of the ten added cards', n;
  end if;

  -- A pending account and a ladder bot own nothing (§9.4, R1454).
  if exists (select 1 from public.collection where profile_id in (waiting, bot)) then
    raise exception 'FAIL (R1705002): a pending account or a ladder bot was granted cards';
  end if;
  perform set_config('r1705002.grants', (select count(*) from public.collection_grants)::text, true);
end $$;

-- Applied again, 0055 grants nothing more.
\i /tmp/0055_deck_size_25.sql

do $$
declare
  n bigint;
begin
  select count(*) into n from public.collection_grants;
  if n::text <> current_setting('r1705002.grants') then
    raise exception 'FAIL (R1705002): applying 0055 again wrote % ledger rows, expected %',
      n, current_setting('r1705002.grants');
  end if;
  if app.grant_starter_cards_to_active() < 2 then
    raise exception 'FAIL (R1705002): app.grant_starter_cards_to_active() visited fewer than the two active people';
  end if;

  -- An empty list is seed-catalog's to write: 0055 leaves it empty.
  update app.settings set value = '[]'::jsonb where key = 'starter_cards';
end $$;
\i /tmp/0055_deck_size_25.sql
do $$
begin
  if app.setting('starter_cards') is distinct from '[]'::jsonb then
    raise exception 'FAIL (R1705002): 0055 wrote % over an empty starter list', app.setting('starter_cards')::text;
  end if;
  raise notice 'OK (R1705002): 0055 adds the eleven to the starter list and grants them once to every active person, a second run grants nothing, and an empty list stays empty';
end $$;
rollback;

\echo '### R1705002: a client cannot run the grant ###'
begin;
do $$
begin
  if has_function_privilege('authenticated', 'app.grant_starter_cards_to_active()', 'execute')
     or has_function_privilege('anon', 'app.grant_starter_cards_to_active()', 'execute') then
    raise exception 'FAIL (R1705002): a client role may execute app.grant_starter_cards_to_active()';
  end if;
  if not has_function_privilege('service_role', 'app.grant_starter_cards_to_active()', 'execute') then
    raise exception 'FAIL (R1705002): service_role may not execute app.grant_starter_cards_to_active()';
  end if;
  raise notice 'OK (R1705002): only service_role executes the grant';
end $$;
rollback;
