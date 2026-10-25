-- The hero portraits as account cosmetics (migration 0057, SPEC §14.2, §14.11, §14.12; R1827000,
-- R1454; issue #1827). Runs after 31_deck_size.sql. run.sh applied 0057 after 03b and 14b had signed
-- up their profiles (03b's 44444444-… active, 14b's three b1400000-… pending) and before 01 and 03
-- signed up 11111111-…, 22222222-… and 33333333-…, so 0057's grant found exactly those four people:
-- the first block checks what it made of them, and the second applies the file again
-- (/tmp/0057_portrait_cosmetics.sql, which run.sh copied there) over profiles of its own.
\set ON_ERROR_STOP on

-- Same rules as 01-31: every check raises on failure (`raise exception 'FAIL (…): …'`), each check
-- that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R1827000: 0057 wrote the six portraits and gave every person then signed up the five that are not starters ###'
begin;
do $$
declare
  legacy constant uuid := '44444444-4444-4444-4444-444444444444';
  before_usernames constant uuid[] := array['b1400000-0000-4000-8000-000000000001',
                                            'b1400000-0000-4000-8000-000000000002',
                                            'b1400000-0000-4000-8000-000000000003']::uuid[];
  later constant uuid[] := array['11111111-1111-1111-1111-111111111111',
                                 '22222222-2222-2222-2222-222222222222',
                                 '33333333-3333-3333-3333-333333333333']::uuid[];
  five constant text := 'portrait-dfender, portrait-felinors, portrait-gary, portrait-shredder, portrait-timmy';
  who    uuid;
  v_text text;
  n      bigint;
begin
  -- The registry's six, as crates/server/cosmetics.json has them: vanilla the kind's Common default,
  -- the rest Epic.
  select string_agg(format('%s %s %s default=%s', id, name, rarity, is_default::text), ', ' order by id) into v_text
    from public.cosmetics where kind = 'portrait';
  if v_text is distinct from 'portrait-dfender Big D-fender portrait Epic default=false, '
                             'portrait-felinors Duplicating Felinors portrait Epic default=false, '
                             'portrait-gary Gary the Gambler portrait Epic default=false, '
                             'portrait-shredder Jlockeed Shredder-10 portrait Epic default=false, '
                             'portrait-timmy Tempo Timmy portrait Epic default=false, '
                             'portrait-vanilla Mr. Vanilla portrait Common default=true' then
    raise exception 'FAIL (R1827000): the portrait rows of public.cosmetics are [%]', coalesce(v_text, '(none)');
  end if;

  -- The grant marked itself done, with the time it ran.
  if jsonb_typeof(app.setting('portraits_granted')) is distinct from 'string' then
    raise exception 'FAIL (R1827000): app.settings.portraits_granted reads %, expected the time 0057 ran',
      coalesce(app.setting('portraits_granted')::text, '(no row)');
  end if;

  -- Vacuity guards: the four people 0057 found are people, and 03b's account was active then.
  select count(*) into n from public.profiles
   where id = any (array[legacy] || before_usernames) and kind = 'human';
  if n <> 4 then
    raise exception 'FAIL (R1827000): % of 03b''s and 14b''s four profiles are people', n;
  end if;
  if not exists (select 1 from public.profiles where id = legacy and status = 'active') then
    raise exception 'FAIL (R1827000): 03b''s profile is not active';
  end if;

  -- Each owns exactly the five, whatever its status, each with one ledger row: no card, +1, the
  -- launch, its ref, no dev tool's.
  foreach who in array array[legacy] || before_usernames loop
    select string_agg(cosmetic_id, ', ' order by cosmetic_id) into v_text
      from public.account_cosmetics where profile_id = who;
    if v_text is distinct from five then
      raise exception 'FAIL (R1827000): % owns the account cosmetics [%], expected the five portraits',
        who, coalesce(v_text, '(none)');
    end if;
    select string_agg(format('%s card=%s %s %s %s dev=%s', cosmetic_id, coalesce(card_id, '-'), delta,
                             reason, ref, dev::text), ', ' order by cosmetic_id)
      into v_text from public.cosmetic_grants where profile_id = who;
    if v_text is distinct from (select string_agg(format('%s card=- 1 launch launch:0057 dev=false', id), ', '
                                                  order by id)
                                  from unnest(string_to_array(five, ', ')) as id) then
      raise exception 'FAIL (R1827000): %''s cosmetic ledger is [%], expected one launch:0057 row per portrait',
        who, coalesce(v_text, '(empty)');
    end if;
  end loop;

  -- Nobody else was granted anything: 0057's rows are those four people's twenty.
  select count(*) into n from public.cosmetic_grants where ref = 'launch:0057';
  if n <> 20 then
    raise exception 'FAIL (R1827000): 0057 wrote % ledger rows, expected the four people''s 20', n;
  end if;

  -- The profiles that signed up after 0057 own no portrait: vanilla is owned with no row (§14.2), and
  -- the rest are earned.
  foreach who in array later loop
    if not exists (select 1 from public.profiles where id = who and kind = 'human') then
      raise exception 'FAIL (R1827000): % is no person, so "owns no portrait" was measured over nobody', who;
    end if;
    if exists (select 1 from public.account_cosmetics where profile_id = who and cosmetic_id like 'portrait-%')
       or exists (select 1 from public.cosmetic_grants where profile_id = who and cosmetic_id like 'portrait-%') then
      raise exception 'FAIL (R1827000): %, signed up after 0057, owns a portrait', who;
    end if;
  end loop;

  raise notice 'OK (R1827000): the six portraits are in the registry''s rows, the four people signed up before 0057 own the five that are not starters with a launch row each, and the three after own none';
end $$;
rollback;

\echo '### R1827000: 0057 over the people of a database, once: pending or active, never a bot ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e1827000-0000-4000-8000-000000000001', 'r1827000-active@example.test', now()),
       ('e1827000-0000-4000-8000-000000000002', 'r1827000-pending@example.test', now());
update public.profiles set status = 'active', activated_at = now()
 where id = 'e1827000-0000-4000-8000-000000000001';
insert into public.profiles (id, kind, status) values ('e1827000-0000-4000-8000-000000000003', 'bot', 'active');
-- The active person already owns one portrait, from a pack: 0057 passes it over and writes it no row.
insert into public.account_cosmetics (profile_id, cosmetic_id)
values ('e1827000-0000-4000-8000-000000000001', 'portrait-gary');
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason)
values ('e1827000-0000-4000-8000-000000000001', null, 'portrait-gary', 1, 'pack_open');
-- A row seed-catalog wrote stays as it wrote it.
update public.cosmetics set name = 'Seeded name' where id = 'portrait-timmy';

do $$
begin
  if exists (select 1 from public.account_cosmetics
              where profile_id in ('e1827000-0000-4000-8000-000000000002', 'e1827000-0000-4000-8000-000000000003')) then
    raise exception 'FAIL (R1827000): the pending person or the bot owns an account cosmetic before 0057 runs';
  end if;
  if not exists (select 1 from public.profiles
                  where id = 'e1827000-0000-4000-8000-000000000001' and kind = 'human' and status = 'active')
     or not exists (select 1 from public.profiles
                     where id = 'e1827000-0000-4000-8000-000000000002' and kind = 'human' and status = 'pending')
     or not exists (select 1 from public.profiles
                     where id = 'e1827000-0000-4000-8000-000000000003' and kind = 'bot') then
    raise exception 'FAIL (R1827000): the three profiles are not an active person, a pending one and a bot';
  end if;
end $$;

-- The grant, as it runs on a database these people are signed up to: its marker deleted, 0057 applied.
delete from app.settings where key = 'portraits_granted';
set local client_min_messages = warning;
\i /tmp/0057_portrait_cosmetics.sql
set local client_min_messages = notice;

do $$
declare
  active  constant uuid := 'e1827000-0000-4000-8000-000000000001';
  waiting constant uuid := 'e1827000-0000-4000-8000-000000000002';
  bot     constant uuid := 'e1827000-0000-4000-8000-000000000003';
  legacy  constant uuid := '44444444-4444-4444-4444-444444444444';
  five    constant text := 'portrait-dfender, portrait-felinors, portrait-gary, portrait-shredder, portrait-timmy';
  who     uuid;
  v_text  text;
begin
  -- Both people own the five, each with one ledger row: the launch's, but the active person's Gary,
  -- whose row is still its pack's alone.
  foreach who in array array[active, waiting] loop
    select string_agg(cosmetic_id, ', ' order by cosmetic_id) into v_text
      from public.account_cosmetics where profile_id = who;
    if v_text is distinct from five then
      raise exception 'FAIL (R1827000): % owns [%] after 0057, expected the five portraits', who, coalesce(v_text, '(none)');
    end if;
  end loop;
  select string_agg(format('%s %s %s', profile_id, cosmetic_id, reason || coalesce(' ' || ref, '')), ', '
                    order by profile_id, cosmetic_id)
    into v_text from public.cosmetic_grants where profile_id in (active, waiting);
  if v_text is distinct from format('%1$s portrait-dfender launch launch:0057, %1$s portrait-felinors launch launch:0057, '
                                    '%1$s portrait-gary pack_open, %1$s portrait-shredder launch launch:0057, '
                                    '%1$s portrait-timmy launch launch:0057, '
                                    '%2$s portrait-dfender launch launch:0057, %2$s portrait-felinors launch launch:0057, '
                                    '%2$s portrait-gary launch launch:0057, %2$s portrait-shredder launch launch:0057, '
                                    '%2$s portrait-timmy launch launch:0057', active, waiting) then
    raise exception 'FAIL (R1827000): the two people''s cosmetic ledgers are [%]', coalesce(v_text, '(empty)');
  end if;

  -- A ladder bot owns nothing (R1454).
  if exists (select 1 from public.account_cosmetics where profile_id = bot)
     or exists (select 1 from public.cosmetic_grants where profile_id = bot) then
    raise exception 'FAIL (R1454): 0057 gave the ladder bot a portrait';
  end if;

  -- 03b's account, granted when run.sh applied 0057, still holds its five rows and its five ledger rows.
  if (select count(*) from public.account_cosmetics where profile_id = legacy) <> 5
     or (select count(*) from public.cosmetic_grants where profile_id = legacy) <> 5 then
    raise exception 'FAIL (R1827000): a second grant changed 03b''s five portraits or their five ledger rows';
  end if;

  -- Every person of the database owns the five now, and each portrait's ledger sums to what is held
  -- (§14.11).
  if exists (select 1 from public.profiles p
              cross join (values ('portrait-gary'), ('portrait-timmy'), ('portrait-dfender'),
                                 ('portrait-felinors'), ('portrait-shredder')) as c(id)
              where p.kind = 'human'
                and not exists (select 1 from public.account_cosmetics o
                                 where o.profile_id = p.id and o.cosmetic_id = c.id)) then
    raise exception 'FAIL (R1827000): a person lacks a portrait after 0057';
  end if;
  select string_agg(format('%s %s', g.profile_id, g.cosmetic_id), ', ') into v_text
    from (select profile_id, cosmetic_id, sum(delta) as held from public.cosmetic_grants
           where card_id is null and cosmetic_id like 'portrait-%' group by profile_id, cosmetic_id) g
   where g.held <> case when exists (select 1 from public.account_cosmetics o
                                      where o.profile_id = g.profile_id and o.cosmetic_id = g.cosmetic_id)
                        then 1 else 0 end;
  if v_text is not null then
    raise exception 'FAIL (§14.11): the portrait ledgers of [%] do not sum to what is held', v_text;
  end if;

  if (select name from public.cosmetics where id = 'portrait-timmy') is distinct from 'Seeded name' then
    raise exception 'FAIL (R1827000): 0057 wrote over a portrait row seed-catalog had written';
  end if;
  if app.setting('portraits_granted') is null then
    raise exception 'FAIL (R1827000): the grant did not mark itself done';
  end if;
  raise notice 'OK (R1827000): 0057 gave the active and the pending person the five portraits with a launch row each, passing over one already owned, the bot none, and 03b''s account nothing more';
end $$;

-- Applied once more, it writes nothing: the grant marked itself done.
create temp table r1827000_rows (tbl text primary key, n bigint not null) on commit drop;
insert into r1827000_rows
select 'cosmetics', count(*) from public.cosmetics
union all select 'account_cosmetics', count(*) from public.account_cosmetics
union all select 'cosmetic_grants', count(*) from public.cosmetic_grants;
do $$
begin
  perform set_config('r1827000.marker', app.setting('portraits_granted')::text, true);
end $$;
set local client_min_messages = warning;
\i /tmp/0057_portrait_cosmetics.sql
set local client_min_messages = notice;

-- And a person who signs up after the grant owns no portrait.
insert into auth.users (id, email, email_confirmed_at)
values ('e1827000-0000-4000-8000-000000000004', 'r1827000-later@example.test', now());
update public.profiles set status = 'active', activated_at = now()
 where id = 'e1827000-0000-4000-8000-000000000004';

do $$
declare
  later constant uuid := 'e1827000-0000-4000-8000-000000000004';
  r record;
  n bigint;
begin
  for r in select x.tbl, x.n as before from r1827000_rows x loop
    execute format('select count(*) from public.%I', r.tbl) into n;
    if n <> r.before then
      raise exception 'FAIL (R1827000): a second apply changed public.% from % rows to %', r.tbl, r.before, n;
    end if;
  end loop;
  if app.setting('portraits_granted')::text is distinct from current_setting('r1827000.marker') then
    raise exception 'FAIL (R1827000): a second apply moved the marker from % to %',
      current_setting('r1827000.marker'), app.setting('portraits_granted')::text;
  end if;

  if not exists (select 1 from public.profiles where id = later and kind = 'human' and status = 'active') then
    raise exception 'FAIL (R1827000): the later signup is no active person';
  end if;
  if exists (select 1 from public.account_cosmetics where profile_id = later)
     or exists (select 1 from public.cosmetic_grants where profile_id = later) then
    raise exception 'FAIL (R1827000): a person who signed up after the grant owns a portrait';
  end if;
  raise notice 'OK (R1827000): 0057 re-applies over its own grant and writes nothing, and a later signup owns no portrait';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
