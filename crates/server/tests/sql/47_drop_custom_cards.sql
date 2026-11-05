-- Custom cards are removed (migration 0081; issues #3095 and #2956): the table 0037 made, its two write
-- paths and the cap's setting go. Runs after 46_dev_dashboard.sql, with every migration applied.
\set ON_ERROR_STOP on

-- Same rules as 01-46: every check raises on failure (`raise exception 'FAIL (#3095): …'`), each check
-- that could pass vacuously carries a guard, and every block that writes runs inside a transaction that
-- is rolled back.

\echo '### #3095: with every migration applied there is no custom_cards table, write path or cap ###'
do $$
begin
  if to_regclass('public.custom_cards') is not null then
    raise exception 'FAIL (#3095): public.custom_cards still exists';
  end if;
  if exists (select 1 from pg_policies where tablename = 'custom_cards') then
    raise exception 'FAIL (#3095): a policy on custom_cards outlived its table';
  end if;
  if exists (
    select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     where n.nspname = 'app' and p.proname in ('create_custom_card', 'update_custom_card')
  ) then
    raise exception 'FAIL (#3095): a write path for a custom card still exists';
  end if;
  if exists (select 1 from app.settings where key = 'max_custom_cards') then
    raise exception 'FAIL (#3095): app.settings still holds max_custom_cards';
  end if;
  raise notice 'OK (#3095): no custom_cards table, policy, write path or cap';
end $$;

\echo '### #3095: 0081 deletes the custom cards a database held, with their table, and re-applies over nothing ###'
begin;
-- A database that has not yet applied 0081 holds what 0037 made. 0037 re-applies, so it stands in for
-- one: the table, its two functions and the cap come back for this transaction alone.
\i /tmp/0037_custom_cards.sql
insert into auth.users (id, email, email_confirmed_at)
values ('b3095000-0000-4000-8000-000000000001', 'owner3095@example.test', now());
do $$
declare
  owner constant uuid := 'b3095000-0000-4000-8000-000000000001';
  n int;
begin
  insert into public.custom_cards (id, profile_id, frame) values
    ('custom-3095-a', owner, '{}'), ('custom-3095-b', owner, '{}');
  select count(*) into n from public.custom_cards where profile_id = owner;
  if n <> 2 then
    raise exception 'FAIL (#3095): % cards seeded for 0081 to delete, not 2; the check below would prove nothing', n;
  end if;
  if (select count(*) from pg_proc p join pg_namespace s on s.oid = p.pronamespace
       where s.nspname = 'app' and p.proname in ('create_custom_card', 'update_custom_card')) <> 2 then
    raise exception 'FAIL (#3095): 0037 did not make both write paths, so their drop proves nothing';
  end if;
  if not exists (select 1 from app.settings where key = 'max_custom_cards') then
    raise exception 'FAIL (#3095): 0037 did not seed the cap, so its delete proves nothing';
  end if;
end $$;

\i /tmp/0081_drop_custom_cards.sql

do $$
begin
  if to_regclass('public.custom_cards') is not null then
    raise exception 'FAIL (#3095): 0081 left public.custom_cards, and the cards in it';
  end if;
  if exists (
    select 1 from pg_proc p join pg_namespace n on n.oid = p.pronamespace
     where n.nspname = 'app' and p.proname in ('create_custom_card', 'update_custom_card')
  ) then
    raise exception 'FAIL (#3095): 0081 left a write path for a custom card';
  end if;
  if exists (select 1 from app.settings where key = 'max_custom_cards') then
    raise exception 'FAIL (#3095): 0081 left max_custom_cards in app.settings';
  end if;
  if not exists (select 1 from public.profiles where id = 'b3095000-0000-4000-8000-000000000001') then
    raise exception 'FAIL (#3095): 0081 deleted the cards'' owner, not only the cards';
  end if;
end $$;

-- Safe to re-apply: nothing is left for it to drop.
\i /tmp/0081_drop_custom_cards.sql
do $$
begin
  raise notice 'OK (#3095): 0081 deletes the cards with their table, keeps their owner, and re-applies over nothing';
end $$;
rollback;
