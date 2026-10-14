-- "More cards from" picks a set (migration 0036, SPEC §9.5, R1372, R924000; issue #924). Runs after
-- 19_replays.sql.
\set ON_ERROR_STOP on

-- Same rules as 01-19: every check raises on failure (`raise exception 'FAIL (R…): …'`), every
-- expected refusal is matched on its sqlstate and on the object it names, each check that could pass
-- vacuously carries a guard, and every block runs inside a transaction that is rolled back.

\echo '### R924000: 0036 turns a stored "newest set" into the pick "newest", and no lean into none ###'
begin;
-- Rows as a server from before 0036 wrote them: a ticket and an open All Random room that lean on
-- the newest set, a ticket that does not, and one ticket that already holds a pick (0036 applied
-- again must leave it). 0036 is applied over them as a database that held them when it first ran.
insert into auth.users (id, email, email_confirmed_at) values
  ('b9240000-0000-4000-8000-000000000001', 'lean1@example.test', now()),
  ('b9240000-0000-4000-8000-000000000002', 'lean2@example.test', now()),
  ('b9240000-0000-4000-8000-000000000003', 'lean3@example.test', now());
insert into public.tickets (id, profile_id, rating, mode, frozen_deck, catalog_version, status, lean_newest)
values
  ('a9240000-0000-4000-8000-000000000001', 'b9240000-0000-4000-8000-000000000001', 1000, 'random', '[]', 'core-1', 'cancelled', true),
  ('a9240000-0000-4000-8000-000000000002', 'b9240000-0000-4000-8000-000000000002', 1000, 'random', '[]', 'core-1', 'cancelled', false);
insert into public.tickets (id, profile_id, rating, mode, frozen_deck, catalog_version, status, lean_newest, lean_set)
values
  ('a9240000-0000-4000-8000-000000000003', 'b9240000-0000-4000-8000-000000000003', 1000, 'random', '[]', 'core-1', 'cancelled', false, 'Classic');
insert into public.matches (id, room_code, room_mode, status, seed, p1_profile_id, p1_deck, catalog_version,
                            ceiling_at, room_lean_newest)
values ('c9240000-0000-4000-8000-000000000001', 'LEANNW', 'random', 'open', '',
        'b9240000-0000-4000-8000-000000000001', '[]', 'core-1', now() + interval '1 hour', true);
\i /tmp/0036_lean_set.sql
-- Twice: a second application changes nothing more.
\i /tmp/0036_lean_set.sql
do $$
declare
  v text;
  n int;
begin
  select count(*) into n from public.tickets where id::text like 'a9240000-%';
  if n <> 3 then
    raise exception 'FAIL (R924000): % seeded tickets, not 3; the checks below would prove nothing', n;
  end if;
  select lean_set into v from public.tickets where id = 'a9240000-0000-4000-8000-000000000001';
  if v is distinct from 'newest' then
    raise exception 'FAIL (R924000): a ticket stored with lean_newest = true reads lean_set %, not ''newest''', v;
  end if;
  select lean_set into v from public.tickets where id = 'a9240000-0000-4000-8000-000000000002';
  if v is not null then
    raise exception 'FAIL (R924000): a ticket stored with lean_newest = false reads lean_set %, not null', v;
  end if;
  select lean_set into v from public.tickets where id = 'a9240000-0000-4000-8000-000000000003';
  if v is distinct from 'Classic' then
    raise exception 'FAIL (R924000): applying 0036 again rewrote a ticket''s pick to %', v;
  end if;
  select room_lean_set into v from public.matches where id = 'c9240000-0000-4000-8000-000000000001';
  if v is distinct from 'newest' then
    raise exception 'FAIL (R924000): a room stored with room_lean_newest = true reads room_lean_set %, not ''newest''', v;
  end if;
  raise notice 'OK (R924000): true became ''newest'', false became none, and a pick already made stays';
end $$;
rollback;

\echo '### R924000: a stored pick is "newest" or a set''s name, nothing else ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('b9240000-0000-4000-8000-000000000004', 'lean4@example.test', now());
do $$
declare
  refused_by text;
  junk text;
begin
  foreach junk in array array['', 'Classic; drop table x', '42', 'a set name far too long to be any set at all'] loop
    begin
      insert into public.tickets (profile_id, rating, mode, frozen_deck, catalog_version, status, lean_set)
      values ('b9240000-0000-4000-8000-000000000004', 1000, 'random', '[]', 'core-1', 'cancelled', junk);
      raise exception 'FAIL (R924000): a ticket took the lean %', quote_literal(junk);
    exception
      when check_violation then
        get stacked diagnostics refused_by = constraint_name;
        if refused_by <> 'tickets_lean_set_check' then
          raise exception 'FAIL (R924000): % was refused by %, not tickets_lean_set_check', quote_literal(junk), refused_by;
        end if;
    end;
  end loop;
  insert into public.tickets (profile_id, rating, mode, frozen_deck, catalog_version, status, lean_set)
  values ('b9240000-0000-4000-8000-000000000004', 1000, 'random', '[]', 'core-1', 'cancelled', 'Classic+');
  raise notice 'OK (R924000): junk is refused and a set''s name, Classic+ among them, is taken';
end $$;
rollback;

\echo '### R924000: a player reads their own ticket''s pick ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('b9240000-0000-4000-8000-000000000005', 'lean5@example.test', now());
insert into public.tickets (id, profile_id, rating, mode, frozen_deck, catalog_version, status, lean_set)
values ('a9240000-0000-4000-8000-000000000005', 'b9240000-0000-4000-8000-000000000005', 1000, 'random', '[]',
        'core-1', 'cancelled', 'Core');
set local role authenticated;
select set_config('request.jwt.claim.sub', 'b9240000-0000-4000-8000-000000000005', true);
do $$
declare
  v text;
begin
  select lean_set into v from public.tickets where id = 'a9240000-0000-4000-8000-000000000005';
  if v is distinct from 'Core' then
    raise exception 'FAIL (R924000): the ticket''s owner reads its pick as %, not Core', v;
  end if;
  raise notice 'OK (R924000): the owner reads their pick';
end $$;
rollback;
