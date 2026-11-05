-- Card proficiency, one row per account and card: its plays, its level and the stamp it wears
-- (migration 0081, SPEC §14.14, §9.1; R3013000, R3013001; issue #3013). Runs after
-- 46_dev_dashboard.sql; profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03 activated
-- them).
\set ON_ERROR_STOP on

-- Same rules as 01-46: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate (and on the constraint, the table or the message that
-- names it), each check that could pass vacuously carries a guard, and every block that writes runs
-- inside a transaction that is rolled back. Nothing here selects into the log: the do blocks raise
-- or say OK.

\echo '### R3013000: public.card_proficiency keys one row per account and card, follows its account, and holds its counts to the checks ###'
do $$
declare
  v_text text;
  want   text;
  r      record;
begin
  -- RLS on (CHECK 1 holds every table to it; this names the table so its absence is caught here too).
  if not exists (select 1 from pg_class
                  where oid = 'public.card_proficiency'::regclass and relrowsecurity) then
    raise exception 'FAIL (R3013000): public.card_proficiency has no row level security';
  end if;

  -- The columns, as the store reads and writes them.
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'card_proficiency';
  if v_text is distinct from 'profile_id uuid NO, card_id text NO, plays bigint NO, level bigint NO, '
                             'stamp bigint YES, updated_at timestamp with time zone NO' then
    raise exception 'FAIL (R3013000): public.card_proficiency has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- One row per account and card.
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.card_proficiency'::regclass and c.contype = 'p';
  if v_text is distinct from 'profile_id, card_id' then
    raise exception 'FAIL (R3013000): the primary key is (%), expected (profile_id, card_id)', coalesce(v_text, 'none');
  end if;

  -- Its one foreign key is its account's, cascading (0012's rule); the card is named by its id alone,
  -- as card_cosmetic_picks names it (0061).
  select string_agg(format('%s %s', c.confrelid::regclass, c.confdeltype), ', ') into v_text
    from pg_constraint c
   where c.conrelid = 'public.card_proficiency'::regclass and c.contype = 'f';
  if v_text is distinct from 'profiles c' then
    raise exception 'FAIL (R3013000): the foreign keys are [%], expected the profile''s alone, on delete cascade',
      coalesce(v_text, '(none)');
  end if;

  -- The four checks, each as the store's own refusals (economy::store's check_proficiency, is_stamp)
  -- mirror it.
  for r in select * from (values
      ('card_proficiency_card_check', 'CHECK ((card_id ~ ''^[a-z][a-z0-9-]{0,39}$''::text))'),
      ('card_proficiency_plays_check', 'CHECK ((plays > 0))'),
      ('card_proficiency_level_check', 'CHECK ((level >= 0))'),
      ('card_proficiency_stamp_check', 'CHECK (((stamp IS NULL) OR ((stamp >= 1) AND (stamp <= level))))'))
      as x(name, def) loop
    select pg_get_constraintdef(c.oid) into v_text
      from pg_constraint c
     where c.conrelid = 'public.card_proficiency'::regclass and c.conname = r.name and c.contype = 'c';
    if v_text is distinct from r.def then
      raise exception 'FAIL (R3013000): % is [%], expected [%]', r.name, coalesce(v_text, '(none)'), r.def;
    end if;
  end loop;
  select string_agg(conname, ', ' order by conname) into v_text
    from pg_constraint
   where conrelid = 'public.card_proficiency'::regclass and contype = 'c';
  want := 'card_proficiency_card_check, card_proficiency_level_check, card_proficiency_plays_check, card_proficiency_stamp_check';
  if v_text is distinct from want then
    raise exception 'FAIL (R3013000): the checks are [%], expected [%]', coalesce(v_text, '(none)'), want;
  end if;

  -- Current state, no ledger: no trigger keeps a row from changing.
  if exists (select 1 from pg_trigger where tgrelid = 'public.card_proficiency'::regclass and not tgisinternal) then
    raise exception 'FAIL (R3013000): public.card_proficiency has a trigger, but a game''s pay updates its rows';
  end if;

  raise notice 'OK (R3013000): card_proficiency has RLS on, its columns, one row per (profile_id, card_id), one foreign key, its account''s, cascading, its four checks and no trigger';
end $$;

\echo '### R3013000, §9.1: authenticated reads its own proficiency through one select policy and writes none; anon reaches nothing ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  select string_agg(format('%s %s %s %s', policyname, cmd, array_to_string(roles, ','), qual), '; ')
    into v_text
    from pg_policies
   where schemaname = 'public' and tablename = 'card_proficiency';
  if v_text is distinct from 'card_proficiency_select_own SELECT authenticated (profile_id = app.current_profile_id())' then
    raise exception 'FAIL (R3013000): the policies are [%], expected one select-own policy to authenticated',
      coalesce(v_text, '(none)');
  end if;

  foreach who in array array['authenticated', 'anon', 'public'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.card_proficiency', op) <> (who = 'authenticated' and op = 'SELECT') then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R3013000): the client privileges on card_proficiency are wrong for [%]',
      array_to_string(wrong, ', ');
  end if;
  -- What the server's role may do is Supabase's grant, which this suite's stub leaves out
  -- (tests/db/grants.sql makes it, and tests/store/contract.rs writes the table as the server).
  raise notice 'OK (R3013000): one select-own policy to authenticated, SELECT its only client privilege, anon and public none';
end $$;

\echo '### R3013000: a row''s card is a slug, its plays at least one and its level not below 0 ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, card_id, plays, level, the check it breaks]. Each breaks that check and nothing else.
  refused constant text[][] := array[
    ['a card id that starts upper-case', 'Core-008', '3', '1', 'card_proficiency_card_check'],
    ['a card id that starts with a digit', '8-core', '3', '1', 'card_proficiency_card_check'],
    ['an empty card id', '', '3', '1', 'card_proficiency_card_check'],
    ['a card id with an underscore', 'core_008', '3', '1', 'card_proficiency_card_check'],
    ['a card id of 41 characters', 'a' || repeat('b', 40), '3', '1', 'card_proficiency_card_check'],
    ['no play', 'core-008', '0', '0', 'card_proficiency_plays_check'],
    ['plays below 0', 'core-008', '-1', '0', 'card_proficiency_plays_check'],
    ['a level below 0', 'core-008', '3', '-1', 'card_proficiency_level_check']];
  i     int;
  v_con text;
begin
  if not exists (select 1 from public.profiles where id = p1) then
    raise exception 'FAIL (R3013000): profile 1 is missing, so every insert below would fail on its foreign key';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.card_proficiency (profile_id, card_id, plays, level)
      values (p1, refused[i][2], refused[i][3]::bigint, refused[i][4]::bigint);
      raise exception 'FAIL (R3013000): the schema took %', refused[i][1];
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from refused[i][5] then
          raise exception 'FAIL (R3013000): % was refused by %, not %', refused[i][1], v_con, refused[i][5];
        end if;
    end;
  end loop;

  -- The vacuity guard: what the store writes is taken, at the bounds: one play at level 0, a card id
  -- of 40 characters, a row at a high level.
  insert into public.card_proficiency (profile_id, card_id, plays, level)
  values (p1, 'core-008', 1, 0), (p1, 'a' || repeat('b', 39), 3, 1), (p1, 'core-009', 200, 5);
  if (select count(*) from public.card_proficiency where profile_id = p1) <> 3 then
    raise exception 'FAIL (R3013000): the three rows the checks allow were not all written';
  end if;
  -- Once per account and card: a second row conflicts on the key.
  begin
    insert into public.card_proficiency (profile_id, card_id, plays, level) values (p1, 'core-008', 2, 0);
    raise exception 'FAIL (R3013000): a second row of one card for one account was taken';
  exception
    when unique_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'card_proficiency_pkey' then
        raise exception 'FAIL (R3013000): the second row was refused by %, not card_proficiency_pkey', v_con;
      end if;
  end;
  raise notice 'OK (R3013000): % rows the checks refuse are refused, each by its own check, and the rows the store writes are taken once per card',
    array_length(refused, 1);
end $$;
rollback;

\echo '### R3013001: a stamp above its row''s level is refused ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  v_con text;
  n     bigint;
  held  bigint;
begin
  insert into public.card_proficiency (profile_id, card_id, plays, level) values (p1, 'core-008', 15, 2);
  -- [stamp]: above the level, and below the first stamp.
  for n in select unnest(array[3, 5, 0, -1]) loop
    begin
      update public.card_proficiency set stamp = n where profile_id = p1 and card_id = 'core-008';
      raise exception 'FAIL (R3013001): a row at level 2 took the stamp %', n;
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from 'card_proficiency_stamp_check' then
          raise exception 'FAIL (R3013001): the stamp % was refused by %, not card_proficiency_stamp_check', n, v_con;
        end if;
    end;
  end loop;
  -- A new row's stamp is held to its level too.
  begin
    insert into public.card_proficiency (profile_id, card_id, plays, level, stamp) values (p1, 'core-009', 2, 0, 1);
    raise exception 'FAIL (R3013001): a row at level 0 took a stamp';
  exception
    when check_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'card_proficiency_stamp_check' then
        raise exception 'FAIL (R3013001): the stamped row at level 0 was refused by %, not card_proficiency_stamp_check', v_con;
      end if;
  end;

  -- The vacuity guard, through the store's own statement (economy::store's card_proficiency_stamp): a
  -- stamp at or below the level is kept, one above it updates no row, and none clears it.
  update public.card_proficiency set stamp = 2
   where profile_id = p1 and card_id = 'core-008' and (2::bigint is null or 2::bigint <= level);
  get diagnostics n = row_count;
  if n <> 1 then
    raise exception 'FAIL (R3013001): the store''s stamp at the row''s level wrote % row(s), expected one', n;
  end if;
  update public.card_proficiency set stamp = 4
   where profile_id = p1 and card_id = 'core-008' and (4::bigint is null or 4::bigint <= level);
  get diagnostics n = row_count;
  select stamp into held from public.card_proficiency where profile_id = p1 and card_id = 'core-008';
  if n <> 0 or held is distinct from 2 then
    raise exception 'FAIL (R3013001): the store''s stamp above the level wrote % row(s), the row wears %', n, held;
  end if;
  -- A level put back below the stamp the row wears is refused (economy::store's put, which never
  -- lowers one, would roll back).
  begin
    insert into public.card_proficiency (profile_id, card_id, plays, level) values (p1, 'core-008', 16, 1)
    on conflict (profile_id, card_id) do update set plays = excluded.plays, level = excluded.level;
    raise exception 'FAIL (R3013001): a level below the stamp its row wears was taken';
  exception
    when check_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'card_proficiency_stamp_check' then
        raise exception 'FAIL (R3013001): the lowered level was refused by %, not card_proficiency_stamp_check', v_con;
      end if;
  end;
  -- The store's put of plays and level keeps the stamp.
  insert into public.card_proficiency (profile_id, card_id, plays, level) values (p1, 'core-008', 40, 3)
  on conflict (profile_id, card_id) do update set plays = excluded.plays, level = excluded.level;
  select stamp into held from public.card_proficiency where profile_id = p1 and card_id = 'core-008';
  if held is distinct from 2 then
    raise exception 'FAIL (R3013001): a put of plays and level left the stamp %, not 2', held;
  end if;
  update public.card_proficiency set stamp = null where profile_id = p1 and card_id = 'core-008';
  select stamp into held from public.card_proficiency where profile_id = p1 and card_id = 'core-008';
  if held is not null then
    raise exception 'FAIL (R3013001): clearing the stamp left %', held;
  end if;
  raise notice 'OK (R3013001): a stamp above its row''s level or below 1 is refused, one at the level is kept, a put keeps it and a lowered level under it is refused';
end $$;
rollback;

\echo '### R3013000: an account reads only its own proficiency and writes none of it ###'
begin;

-- Rows for EACH profile, as the owner: profile 2's are the rows to hide from profile 1, and
-- profile 1's from profile 2. Raw inserts, as superuser, because the point is what RLS shows.
insert into public.card_proficiency (profile_id, card_id, plays, level, stamp) values
  ('11111111-1111-1111-1111-111111111111', 'core-008', 15, 2, 2),
  ('11111111-1111-1111-1111-111111111111', 'core-009', 3, 1, null),
  ('22222222-2222-2222-2222-222222222222', 'core-008', 40, 3, 1);

-- Preflight: the writes the client is about to be refused are ones the owner can make, inside a
-- subtransaction rolled back at once.
do $$
begin
  begin
    insert into public.card_proficiency (profile_id, card_id, plays, level)
    values ('11111111-1111-1111-1111-111111111111', 'core-010', 1, 0);
    update public.card_proficiency set stamp = 1
     where profile_id = '11111111-1111-1111-1111-111111111111' and level >= 1;
    delete from public.card_proficiency where profile_id = '11111111-1111-1111-1111-111111111111';
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception
        'FAIL (R3013000): the owner could not make the writes the client is about to be refused ("%", %) — the refusals below would prove nothing about privileges',
        sqlerrm, sqlstate;
    end if;
  end;
  perform set_config('r3013000.total', (select count(*) from public.card_proficiency)::text, true);
end $$;

set local role authenticated;
do $$ begin perform set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true); end $$;

do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  total  bigint := coalesce(nullif(current_setting('r3013000.total', true), ''), '-1')::bigint;
  seen   text;
  probe  text;
  probes constant text[][] := array[
    ['card_proficiency INSERT',
     $q$insert into public.card_proficiency (profile_id, card_id, plays, level)
        values ('11111111-1111-1111-1111-111111111111', 'core-010', 200, 5)$q$],
    ['card_proficiency UPDATE plays', $q$update public.card_proficiency set plays = 200, level = 5$q$],
    ['card_proficiency UPDATE stamp', $q$update public.card_proficiency set stamp = 1$q$],
    ['card_proficiency DELETE', $q$delete from public.card_proficiency$q$],
    ['card_proficiency TRUNCATE', $q$truncate public.card_proficiency$q$]];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R3013000): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  if total <> 3 then
    raise exception 'FAIL (R3013000): % card_proficiency rows to measure against, expected 3 (two of profile 1, one of profile 2)', total;
  end if;

  select string_agg(format('%s %s %s %s', card_id, plays, level, coalesce(stamp::text, '-')), ', ' order by card_id)
    into seen from public.card_proficiency;
  if seen is distinct from 'core-008 15 2 2, core-009 3 1 -'
     or exists (select 1 from public.card_proficiency where profile_id <> caller) then
    raise exception 'FAIL (R3013000): profile 1 read the rows [%] of % — expected its own two and no other account''s',
      coalesce(seen, '(none)'), total;
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R3013000): % succeeded as a client — proficiency is written by the server alone', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%card_proficiency%' then
          raise exception 'FAIL (R3013000): % was refused by "%" (%), which does not name card_proficiency',
            probe, sqlerrm, sqlstate;
        end if;
        raise notice 'OK (R3013000): % refused — % (%)', probe, sqlerrm, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R3013000): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;

  raise notice 'OK (R3013000): profile 1 reads its own two rows and not profile 2''s, and % writes were refused',
    array_length(probes, 1);
end $$;

do $$ begin perform set_config('request.jwt.claim.sub', '22222222-2222-2222-2222-222222222222', true); end $$;
do $$
declare
  seen text;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R3013000): running as %, not authenticated', current_user;
  end if;
  select string_agg(format('%s %s %s', profile_id, card_id, plays), ', ') into seen from public.card_proficiency;
  if seen is distinct from '22222222-2222-2222-2222-222222222222 core-008 40' then
    raise exception 'FAIL (R3013000): profile 2 read the rows [%], expected only its own one', coalesce(seen, '(none)');
  end if;
  raise notice 'OK (R3013000): profile 2 reads only its own row';
end $$;

-- Signed in as nobody: the policy's app.current_profile_id() is null, so no row is anybody's.
do $$ begin perform set_config('request.jwt.claim.sub', '', true); end $$;
do $$
begin
  if app.current_profile_id() is not null then
    raise exception 'FAIL (R3013000): the cleared claim still names profile %', app.current_profile_id();
  end if;
  if exists (select 1 from public.card_proficiency) then
    raise exception 'FAIL (R3013000): a client with no profile read % rows', (select count(*) from public.card_proficiency);
  end if;
  raise notice 'OK (R3013000): a client with no profile reads no row';
end $$;

reset role;
set local role anon;
do $$
begin
  if current_user <> 'anon' then
    raise exception 'FAIL (R3013000): running as %, not anon', current_user;
  end if;
  begin
    perform count(*) from public.card_proficiency;
    raise exception 'FAIL (R3013000): anon read card_proficiency';
  exception
    when insufficient_privilege then null;
  end;
  raise notice 'OK (R3013000): anon reads no proficiency';
end $$;

reset role;
rollback;

\echo '### R3013000: a deleted account takes its proficiency with it, and nobody else''s ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3013000-0000-4000-8000-000000000001', 'r3013000-leaver@example.test', now());
do $$
declare
  gone constant uuid := 'e3013000-0000-4000-8000-000000000001';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R3013000): the leaver has no profile; the cascade below would prove nothing';
  end if;
  insert into public.card_proficiency (profile_id, card_id, plays, level, stamp)
  values (gone, 'core-008', 15, 2, 2), (gone, 'core-009', 3, 1, null), (p1, 'core-008', 3, 1, 1);

  delete from auth.users where id = gone;

  if exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R3013000): the leaver''s profile outlived its auth.users row';
  end if;
  if exists (select 1 from public.card_proficiency where profile_id = gone) then
    raise exception 'FAIL (R3013000): a deleted account''s proficiency outlived it';
  end if;
  if not exists (select 1 from public.card_proficiency where profile_id = p1 and card_id = 'core-008') then
    raise exception 'FAIL (R3013000): deleting one account took another''s proficiency';
  end if;
  raise notice 'OK (R3013000): deleting an account deletes its proficiency and no other account''s';
end $$;
rollback;

\echo '### R3013000: 0081 re-applies over rows of its own and changes none ###'
begin;
insert into public.card_proficiency (profile_id, card_id, plays, level, stamp)
values ('11111111-1111-1111-1111-111111111111', 'core-008', 15, 2, 2);
create temp table r3013000_rows (n bigint not null) on commit drop;
insert into r3013000_rows select count(*) from public.card_proficiency;
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0081_card_proficiency.sql
set local client_min_messages = notice;
do $$
declare
  before bigint := (select n from r3013000_rows);
  held   text;
begin
  if before <> 1 then
    raise exception 'FAIL (R3013000): % rows before the re-apply, expected the one written', before;
  end if;
  if (select count(*) from public.card_proficiency) <> before then
    raise exception 'FAIL (R3013000): re-applying 0081 changed public.card_proficiency from % rows to %',
      before, (select count(*) from public.card_proficiency);
  end if;
  select format('%s %s %s', plays, level, stamp) into held
    from public.card_proficiency where card_id = 'core-008';
  if held is distinct from '15 2 2' then
    raise exception 'FAIL (R3013000): re-applying 0081 left the row as [%]', coalesce(held, '(none)');
  end if;
  if (select count(*) from pg_policies where schemaname = 'public' and tablename = 'card_proficiency') <> 1 then
    raise exception 'FAIL (R3013000): re-applying 0081 left other than one policy on card_proficiency';
  end if;
  raise notice 'OK (R3013000): 0081 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
