-- Usernames (migration 0028, R1434; issue #579). Runs last, after 14_patch_retcon.sql. Four profiles
-- predate 0028: 03b's 44444444-… and 14b's b1400000-…-1, -2 and -3, so 0028's backfill named those
-- four, and every profile 01-14 signed up since was named by 0028's trigger.
\set ON_ERROR_STOP on

-- Same rules as 01-14: every check raises on failure, every expected refusal is matched on its
-- sqlstate and, where it has one, on its constraint or column name, and every block that writes runs
-- inside a transaction that is rolled back, so nothing here changes what another check sees.

\echo '=== CHECK 1 (R1434): 0028 named every profile that predates it Player#n in order of sign-up, unprompted ==='
select id, created_at, username_base, username_key, username_tag, username_changed_at, username_prompted
  from public.profiles
 where id in ('b1400000-0000-4000-8000-000000000001', 'b1400000-0000-4000-8000-000000000002',
              'b1400000-0000-4000-8000-000000000003', '44444444-4444-4444-4444-444444444444')
 order by created_at, id;
do $$
declare
  -- In order of sign-up: 14b's three in 2020, the newest of them inserted first and holding the
  -- lowest id, then 03b's, signed up as this run started, whose id sorts before all three.
  expected constant text[][] := array[
    ['b1400000-0000-4000-8000-000000000003', '1'],
    ['b1400000-0000-4000-8000-000000000002', '2'],
    ['b1400000-0000-4000-8000-000000000001', '3'],
    ['44444444-4444-4444-4444-444444444444', '4']];
  columns_seen text;
  r            record;
  bad          text;
  i            int;
begin
  -- The five columns, their types and their nullability: a bare name has no tag, a default has
  -- never been changed, and every profile has a base and a key.
  select string_agg(format('%s %s %s %s', column_name, data_type, is_nullable,
                           coalesce(column_default, '-')), '; ' order by column_name)
    into columns_seen
    from information_schema.columns
   where table_schema = 'public' and table_name = 'profiles' and column_name like 'username%';
  if columns_seen is distinct from
     'username_base text NO -; username_changed_at timestamp with time zone YES -; '
     'username_key text NO -; username_prompted boolean NO false; username_tag integer YES -' then
    raise exception 'FAIL (CHECK 1): the username columns of public.profiles are %', columns_seen;
  end if;

  -- Vacuity guard: the input is what 14b seeded. Without its three 2020 sign-ups the order below
  -- would be insertion order or id order as much as sign-up order.
  if (select count(*) from public.profiles
       where id in ('b1400000-0000-4000-8000-000000000001', 'b1400000-0000-4000-8000-000000000002',
                    'b1400000-0000-4000-8000-000000000003')
         and created_at < timestamptz '2020-01-04 00:00:00+00') <> 3 then
    raise exception 'FAIL (CHECK 1): 14b''s three 2020 profiles are not all there, so the backfill order is untested';
  end if;

  for i in 1 .. array_length(expected, 1) loop
    select username_base, username_key, username_tag, username_changed_at, username_prompted
      into r from public.profiles where id = expected[i][1]::uuid;
    if not found then
      raise exception 'FAIL (CHECK 1): profile % is gone', expected[i][1];
    end if;
    if r.username_base is distinct from 'Player' or r.username_key is distinct from 'player'
       or r.username_tag is distinct from expected[i][2]::int then
      raise exception 'FAIL (CHECK 1): profile % (sign-up % of 4) holds %/%/#%, expected Player/player/#%',
        expected[i][1], i, r.username_base, r.username_key, r.username_tag, expected[i][2];
    end if;
    if r.username_prompted or r.username_changed_at is not null then
      raise exception 'FAIL (CHECK 1): profile % is prompted=% changed_at=% after the backfill, expected unprompted and never changed',
        expected[i][1], r.username_prompted, r.username_changed_at;
    end if;
  end loop;

  -- Every profile 01-14 signed up after 0028 was named by the trigger, after the backfill's four:
  -- Player#5 and up, unprompted, never changed. (None of 01-14 renames anyone.)
  select string_agg(format('%s %s/%s/#%s prompted=%s changed=%s', id, username_base, username_key,
                           username_tag, username_prompted, username_changed_at), '; ' order by id)
    into bad
    from public.profiles
   where id not in ('b1400000-0000-4000-8000-000000000001', 'b1400000-0000-4000-8000-000000000002',
                    'b1400000-0000-4000-8000-000000000003', '44444444-4444-4444-4444-444444444444')
     and (username_base <> 'Player' or username_key <> 'player' or username_tag is null
          or username_tag <= 4 or username_prompted or username_changed_at is not null);
  if bad is not null then
    raise exception 'FAIL (CHECK 1): a profile signed up after 0028 is not a later default Player#n: %', bad;
  end if;
  if not exists (select 1 from public.profiles where id = '11111111-1111-1111-1111-111111111111') then
    raise exception 'FAIL (CHECK 1): profile 1 is gone, so no profile signed up after 0028 was checked';
  end if;

  raise notice 'OK (CHECK 1): sign-ups 2020-01-01, 2020-01-02, 2020-01-03 and 03b''s are Player#1-#4, unprompted; % later sign-ups hold Player#5 and up',
    (select count(*) from public.profiles where username_key = 'player' and username_tag > 4);
end $$;

\echo '=== CHECK 2 (R1434): one bare holder of a key and one holder of each tag (profiles_username_unique) ==='
select pg_get_indexdef(i.indexrelid) as indexdef, i.indisunique, i.indpred is null as whole_table
  from pg_index i join pg_class ic on ic.oid = i.indexrelid
 where ic.relname = 'profiles_username_unique' and i.indrelid = 'public.profiles'::regclass;
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('c1500000-0000-4000-8000-000000000021', 'clash1@example.test', now()),
       ('c1500000-0000-4000-8000-000000000022', 'clash2@example.test', now()),
       ('c1500000-0000-4000-8000-000000000023', 'clash3@example.test', now());
do $$
declare
  a          constant uuid := 'c1500000-0000-4000-8000-000000000021';
  b          constant uuid := 'c1500000-0000-4000-8000-000000000022';
  c          constant uuid := 'c1500000-0000-4000-8000-000000000023';
  def        text;
  is_unique  boolean;
  whole      boolean;
  held       int;
  refused_by text;
begin
  select pg_get_indexdef(i.indexrelid), i.indisunique, i.indpred is null into def, is_unique, whole
    from pg_index i join pg_class ic on ic.oid = i.indexrelid
   where ic.relname = 'profiles_username_unique' and i.indrelid = 'public.profiles'::regclass;
  if not found then
    raise exception 'FAIL (CHECK 2): public.profiles has no index profiles_username_unique';
  end if;
  if not is_unique or not whole or def not like '%(username_key, COALESCE(username_tag, 0))' then
    raise exception 'FAIL (CHECK 2): profiles_username_unique is not a unique index on (username_key, coalesce(username_tag, 0)) over every row: % (unique %, whole table %)',
      def, is_unique, whole;
  end if;

  -- A holds bare Max. B asks for the same key, written in another case: the key clashes, whatever
  -- the base's display form (the server computes the key; R1434).
  update public.profiles set username_base = 'Max', username_key = 'max', username_tag = null where id = a;
  begin
    update public.profiles set username_base = 'MAX', username_key = 'max', username_tag = null where id = b;
    raise exception 'FAIL (CHECK 2): two bare holders of the key max held';
  exception when unique_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'profiles_username_unique' then
      raise exception 'FAIL (CHECK 2): a second bare max was refused by %, not profiles_username_unique', refused_by;
    end if;
  end;

  -- The same key with different tags is three different usernames: Max, max#1, Max#2.
  update public.profiles set username_base = 'max', username_key = 'max', username_tag = 1 where id = b;
  update public.profiles set username_base = 'Max', username_key = 'max', username_tag = 2 where id = c;
  if (select count(*) from public.profiles where username_key = 'max') <> 3 then
    raise exception 'FAIL (CHECK 2): Max, max#1 and Max#2 did not all hold';
  end if;

  -- A second holder of a tag is refused.
  begin
    update public.profiles set username_base = 'Max', username_key = 'max', username_tag = 1 where id = c;
    raise exception 'FAIL (CHECK 2): two holders of max#1 held';
  exception when unique_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'profiles_username_unique' then
      raise exception 'FAIL (CHECK 2): a second max#1 was refused by %, not profiles_username_unique', refused_by;
    end if;
  end;

  -- A tag clashes only under its own key: #1 of another name is free.
  update public.profiles set username_base = 'Ana', username_key = 'ana', username_tag = 1 where id = c;

  -- A default is a username like any other: a Player#n already held cannot be taken twice.
  select username_tag into held from public.profiles where id = '11111111-1111-1111-1111-111111111111';
  if held is null then
    raise exception 'FAIL (CHECK 2): profile 1 holds no Player tag to clash with';
  end if;
  begin
    update public.profiles set username_base = 'Player', username_key = 'player', username_tag = held
     where id = c;
    raise exception 'FAIL (CHECK 2): a second holder of Player#% held', held;
  exception when unique_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'profiles_username_unique' then
      raise exception 'FAIL (CHECK 2): a second Player#% was refused by %, not profiles_username_unique', held, refused_by;
    end if;
  end;

  raise notice 'OK (CHECK 2): a second bare max and a second max#1 and Player#% refused; Max, max#1, Max#2 and Ana#1 hold together', held;
end $$;
rollback;

\echo '=== CHECK 3 (R1434): a tag is 1 or more, and every profile has a base and a key ==='
begin;
do $$
declare
  p1         constant uuid := '11111111-1111-1111-1111-111111111111';
  t          int;
  col        text;
  refused_by text;
begin
  foreach t in array array[0, -1, -2147483648] loop
    begin
      update public.profiles set username_base = 'Zed', username_key = 'zed', username_tag = t where id = p1;
      raise exception 'FAIL (CHECK 3): the tag % held', t;
    exception when check_violation then
      get stacked diagnostics refused_by = constraint_name;
      if refused_by is distinct from 'profiles_username_tag_positive' then
        raise exception 'FAIL (CHECK 3): the tag % was refused by %, not profiles_username_tag_positive', t, refused_by;
      end if;
    end;
  end loop;

  -- Control: the same write with tag 1, and bare, holds, so the refusals above were the tag's.
  update public.profiles set username_base = 'Zed', username_key = 'zed', username_tag = 1 where id = p1;
  update public.profiles set username_tag = null where id = p1;

  foreach col in array array['username_base', 'username_key', 'username_prompted'] loop
    begin
      execute format('update public.profiles set %I = null where id = $1', col) using p1;
      raise exception 'FAIL (CHECK 3): % = null held', col;
    exception when not_null_violation then
      get stacked diagnostics refused_by = column_name;
      if refused_by is distinct from col then
        raise exception 'FAIL (CHECK 3): % = null was refused on column %', col, refused_by;
      end if;
    end;
  end loop;

  raise notice 'OK (CHECK 3): tags 0, -1 and -2147483648 refused by profiles_username_tag_positive, #1 and bare held; a null base, key or prompted refused';
end $$;
rollback;

\echo '=== CHECK 4 (R1434): a new account starts as the lowest free Player#n under the lock a claim of Player takes, and a freed tag goes to the next one ==='
begin;
do $$
declare
  fresh    constant uuid := 'c1500000-0000-4000-8000-000000000041';
  -- The server's lock on the key player (USERNAME_LOCK_SQL in src/db/pg.rs, bound with 'player').
  k        constant bigint := hashtextextended('jackioh.username:' || 'player', 0);
  expected int;
  r        record;
begin
  -- On the database as 01-14 left it: the lowest n from 1 that no profile holds as Player#n.
  select min(n)::int into expected
    from generate_series(1, (select count(*) from public.profiles where username_key = 'player')::int + 1) as n
   where n not in (select username_tag from public.profiles
                    where username_key = 'player' and username_tag is not null);
  if exists (select 1 from pg_locks where locktype = 'advisory' and pid = pg_backend_pid()) then
    raise exception 'FAIL (CHECK 4): an advisory lock is held before the signup, so the one below proves nothing';
  end if;

  insert into auth.users (id, email, email_confirmed_at) values (fresh, 'fresh@example.test', now());
  select username_base, username_key, username_tag, username_changed_at, username_prompted
    into r from public.profiles where id = fresh;
  if r.username_base is distinct from 'Player' or r.username_key is distinct from 'player'
     or r.username_tag is distinct from expected or r.username_prompted or r.username_changed_at is not null then
    raise exception 'FAIL (CHECK 4): a new account holds %/%/#% prompted=% changed=%, expected Player/player/#%, unprompted, never changed',
      r.username_base, r.username_key, r.username_tag, r.username_prompted, r.username_changed_at, expected;
  end if;

  -- pg_locks shows a bigint advisory key as its high half (classid) and low half (objid), with
  -- objsubid 1. The trigger took the server's lock and holds it until this transaction ends.
  if not exists (select 1 from pg_locks
                  where locktype = 'advisory' and pid = pg_backend_pid() and granted
                    and mode = 'ExclusiveLock'
                    and classid = ((k >> 32) & 4294967295)::oid
                    and objid = (k & 4294967295)::oid
                    and objsubid = 1) then
    raise exception 'FAIL (CHECK 4): the signup did not hold the advisory lock on jackioh.username:player, the one a claim of Player takes';
  end if;

  raise notice 'OK (CHECK 4): a new account is Player#%, the lowest free, under the server''s lock on player', expected;
end $$;
rollback;

-- The same rule on a Player namespace this check controls: everyone holding a Player tag is moved to
-- another name first, inside the transaction, so every number below is known in advance.
begin;
update public.profiles p
   set username_base = 'Moved', username_key = 'moved', username_tag = moved.n
  from (select id, row_number() over (order by id)::int as n
          from public.profiles where username_key = 'player') as moved
 where p.id = moved.id;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  a  constant uuid := 'c1500000-0000-4000-8000-000000000042';
  b  constant uuid := 'c1500000-0000-4000-8000-000000000043';
  c  constant uuid := 'c1500000-0000-4000-8000-000000000044';
  d  constant uuid := 'c1500000-0000-4000-8000-000000000045';
  e  constant uuid := 'c1500000-0000-4000-8000-000000000046';
  f  constant uuid := 'c1500000-0000-4000-8000-000000000047';
  got int;
begin
  if exists (select 1 from public.profiles where username_key = 'player') then
    raise exception 'FAIL (CHECK 4): a Player name is still held after the move';
  end if;

  -- Profile 1 picks bare Player, which a default never is: it takes no tag from anyone.
  update public.profiles set username_base = 'Player', username_key = 'player', username_tag = null where id = p1;

  insert into auth.users (id, email, email_confirmed_at) values (a, 'a-0028@example.test', now());
  insert into auth.users (id, email, email_confirmed_at) values (b, 'b-0028@example.test', now());
  insert into auth.users (id, email, email_confirmed_at) values (c, 'c-0028@example.test', now());
  if (select array_agg(username_tag order by username_tag) from public.profiles where id in (a, b, c))
     is distinct from array[1, 2, 3] then
    raise exception 'FAIL (CHECK 4): three sign-ups beside a bare Player hold %, expected Player#1, #2, #3',
      (select array_agg(username_tag order by username_tag) from public.profiles where id in (a, b, c));
  end if;

  -- B deletes their account, the delete the Supabase dashboard and the admin API make (06): the
  -- profile goes with it, and so does its tag.
  delete from auth.users where id = b;
  if exists (select 1 from public.profiles where id = b) then
    raise exception 'FAIL (CHECK 4): deleting the auth user left its profile';
  end if;
  insert into auth.users (id, email, email_confirmed_at) values (d, 'd-0028@example.test', now());
  select username_tag into got from public.profiles where id = d;
  if got is distinct from 2 then
    raise exception 'FAIL (CHECK 4): the sign-up after Player#2''s account was deleted is Player#%, expected #2 again', got;
  end if;

  -- Profile 2 picks Player#4, as a claim would: the next default steps over it.
  update public.profiles set username_base = 'Player', username_key = 'player', username_tag = 4 where id = p2;
  insert into auth.users (id, email, email_confirmed_at) values (e, 'e-0028@example.test', now());
  select username_tag into got from public.profiles where id = e;
  if got is distinct from 5 then
    raise exception 'FAIL (CHECK 4): with #1-#4 held the next sign-up is Player#%, expected #5', got;
  end if;

  -- A rename frees a tag the same way: C moves to Cee, and #3 is the next default.
  update public.profiles set username_base = 'Cee', username_key = 'cee', username_tag = null where id = c;
  insert into auth.users (id, email, email_confirmed_at) values (f, 'f-0028@example.test', now());
  select username_tag into got from public.profiles where id = f;
  if got is distinct from 3 then
    raise exception 'FAIL (CHECK 4): after Player#3 renamed, the next sign-up is Player#%, expected #3', got;
  end if;

  raise notice 'OK (CHECK 4): beside a bare Player, sign-ups took #1, #2, #3; a deleted #2 and a renamed #3 went to the next sign-ups, and a claimed #4 was stepped over to #5';
end $$;
rollback;

\echo '=== CHECK 5 (R1434): authenticated reads its own username and writes none of it ==='
begin;
-- Vacuity guard: the owner can run every write the client is about to be refused, so each refusal
-- below is about privileges, not about a malformed statement or a constraint.
do $$
begin
  begin
    update public.profiles set username_base = 'Hijack' where id = '11111111-1111-1111-1111-111111111111';
    update public.profiles set username_key = 'hijack' where id = '11111111-1111-1111-1111-111111111111';
    update public.profiles set username_tag = 999999 where id = '11111111-1111-1111-1111-111111111111';
    update public.profiles set username_changed_at = now() where id = '11111111-1111-1111-1111-111111111111';
    update public.profiles set username_prompted = true where id = '11111111-1111-1111-1111-111111111111';
    update public.profiles
       set username_base = 'Hijack', username_key = 'hijack', username_tag = null,
           username_changed_at = null, username_prompted = true
     where id = '11111111-1111-1111-1111-111111111111';
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception 'FAIL (CHECK 5): the owner could not run the writes the client is about to be refused ("%", %)',
        sqlerrm, sqlstate;
    end if;
  end;
end $$;

set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
\echo '-- profile 1''s own username: must be 1 row'
select username_base, username_key, username_tag, username_changed_at, username_prompted from public.profiles;
do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  col    text;
  stmt   text;
  seen   bigint;
  r      record;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (CHECK 5): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;

  -- Reads: its own row's five columns, and no other row (0001's profiles_select_own).
  select count(*) into seen from public.profiles;
  select id, username_base, username_key, username_tag, username_changed_at, username_prompted
    into r from public.profiles where id = caller;
  if seen <> 1 or r.id is distinct from caller or r.username_base is null or r.username_key is null then
    raise exception 'FAIL (CHECK 5): profile 1 saw % row(s), and its own username as %/%', seen, r.username_base, r.username_key;
  end if;

  -- Writes: none, of any one column or all five, each refused by the privilege system.
  foreach stmt in array array[
    'update public.profiles set username_base = ''Hijack'' where id = auth.uid()',
    'update public.profiles set username_key = ''hijack'' where id = auth.uid()',
    'update public.profiles set username_tag = 999999 where id = auth.uid()',
    'update public.profiles set username_changed_at = now() where id = auth.uid()',
    'update public.profiles set username_prompted = true where id = auth.uid()',
    'update public.profiles set username_base = ''Hijack'', username_key = ''hijack'', username_tag = null, '
      || 'username_changed_at = null, username_prompted = true where id = auth.uid()'] loop
    begin
      execute stmt;
      raise exception 'FAIL (CHECK 5): authenticated ran "%" — a username is written only through the API', stmt;
    exception
      when insufficient_privilege then
        null;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (CHECK 5): "%" raised "%" (%), not insufficient_privilege', stmt, sqlerrm, sqlstate;
    end;
  end loop;

  foreach col in array array['username_base', 'username_key', 'username_tag', 'username_changed_at',
                             'username_prompted'] loop
    if not has_column_privilege('public.profiles', col, 'SELECT') then
      raise exception 'FAIL (CHECK 5): authenticated may not select profiles.%', col;
    end if;
    if has_column_privilege('public.profiles', col, 'INSERT')
       or has_column_privilege('public.profiles', col, 'UPDATE') then
      raise exception 'FAIL (CHECK 5): authenticated holds a write grant on profiles.%', col;
    end if;
  end loop;

  raise notice 'OK (CHECK 5): profile 1 reads its own five username columns and no other row; 6 writes refused (insufficient_privilege)';
end $$;
reset role;
rollback;

\echo '=== CHECK 6 (R1434): service_role names a new profile and writes a username, as the server does ==='
begin;
-- This database grants service_role no table privileges (07); a Supabase project grants it all of
-- them on public, which is what tests/db/grants.sql repeats for the store suite. Granted here for
-- this transaction only, and rolled back with it. 0028 grants and revokes service_role nothing.
grant all on public.profiles to service_role;
insert into auth.users (id, email, email_confirmed_at)
values ('c1500000-0000-4000-8000-000000000061', 'server-made@example.test', now()),
       ('c1500000-0000-4000-8000-000000000062', 'claimer@example.test', now());
-- profiles.create's case (src/db/pg.rs): an auth user whose profile the signup trigger never made.
delete from public.profiles where id = 'c1500000-0000-4000-8000-000000000061';
do $$
begin
  perform set_config('r1434.expected_default',
    (select min(n)::text
       from generate_series(1, (select count(*) from public.profiles where username_key = 'player')::int + 1) as n
      where n not in (select username_tag from public.profiles
                       where username_key = 'player' and username_tag is not null)), true);
end $$;
-- The session every store call opens (SESSION_SQL in src/db/pg.rs).
select set_config('role', 'service_role', true),
       set_config('request.jwt.claim.sub', 'c1500000-0000-4000-8000-000000000061', true);
do $$
declare
  made     constant uuid := 'c1500000-0000-4000-8000-000000000061';
  claimer  constant uuid := 'c1500000-0000-4000-8000-000000000062';
  expected int := nullif(current_setting('r1434.expected_default', true), '')::int;
  r        record;
begin
  if current_user <> 'service_role' then
    raise exception 'FAIL (CHECK 6): running as %, not service_role', current_user;
  end if;
  if expected is null then
    raise exception 'FAIL (CHECK 6): no expected default was computed';
  end if;

  -- profiles.create: the server writes no username, and the trigger names the row.
  insert into public.profiles (id, status, rating, created_at)
  values (made, 'pending', 1000::double precision, to_timestamp(1767225600000::double precision / 1000.0));
  select username_base, username_key, username_tag, username_prompted into r from public.profiles where id = made;
  if r.username_base is distinct from 'Player' or r.username_key is distinct from 'player'
     or r.username_tag is distinct from expected or r.username_prompted then
    raise exception 'FAIL (CHECK 6): the server''s insert holds %/%/#% prompted=%, expected Player/player/#%',
      r.username_base, r.username_key, r.username_tag, r.username_prompted, expected;
  end if;

  -- profiles.claimUsername: the lock on the key, the row locked, then the five columns.
  perform pg_advisory_xact_lock(hashtextextended('jackioh.username:' || 'zoe', 0));
  perform username_changed_at from public.profiles where id = made for update;
  update public.profiles
     set username_base = 'Zoe', username_key = 'zoe', username_tag = null,
         username_changed_at = to_timestamp(1767225600000::double precision / 1000.0), username_prompted = true
   where id = made;
  perform set_config('request.jwt.claim.sub', claimer::text, true);
  perform pg_advisory_xact_lock(hashtextextended('jackioh.username:' || 'zoe', 0));
  perform username_changed_at from public.profiles where id = claimer for update;
  update public.profiles
     set username_base = 'ZOE', username_key = 'zoe', username_tag = 1,
         username_changed_at = to_timestamp(1767225600000::double precision / 1000.0), username_prompted = true
   where id = claimer;
  -- profiles.answerUsernamePrompt, on a profile that already answered it: a no-op that still writes.
  update public.profiles set username_prompted = true where id = claimer;

  if (select string_agg(format('%s#%s %s %s', username_base, coalesce(username_tag::text, '-'),
                               username_prompted, username_changed_at is not null), ', ' order by id)
        from public.profiles where id in (made, claimer))
     is distinct from 'Zoe#- t t, ZOE#1 t t' then
    raise exception 'FAIL (CHECK 6): the server''s claims left %',
      (select string_agg(format('%s#%s %s %s', username_base, coalesce(username_tag::text, '-'),
                                username_prompted, username_changed_at is not null), ', ' order by id)
         from public.profiles where id in (made, claimer));
  end if;

  raise notice 'OK (CHECK 6): as service_role, a server-made profile became Player#%, then Zoe; a second claim of zoe holds ZOE#1', expected;
end $$;
reset role;
rollback;

\echo '=== CHECK 7 (R1434): no table but profiles stores a username, and profiles.display_name is gone ==='
select n.nspname as schema, c.relname as relation, a.attname as column_name
  from pg_attribute a
  join pg_class c on c.oid = a.attrelid
  join pg_namespace n on n.oid = c.relnamespace
 where n.nspname in ('public', 'app', 'auth') and c.relkind in ('r', 'p', 'v', 'm', 'f')
   and a.attnum > 0 and not a.attisdropped
   and (a.attname like '%username%' or a.attname like '%display_name%')
 order by 1, 2, 3;
do $$
declare
  seen text;
begin
  -- R1436: every read joins the name in from profiles, so a rename shows everywhere at once.
  select string_agg(format('%s.%s.%s', n.nspname, c.relname, a.attname), ', '
                    order by n.nspname, c.relname, a.attname)
    into seen
    from pg_attribute a
    join pg_class c on c.oid = a.attrelid
    join pg_namespace n on n.oid = c.relnamespace
   where n.nspname in ('public', 'app', 'auth') and c.relkind in ('r', 'p', 'v', 'm', 'f')
     and a.attnum > 0 and not a.attisdropped
     and (a.attname like '%username%' or a.attname like '%display_name%');
  if seen is distinct from
     'public.profiles.username_base, public.profiles.username_changed_at, public.profiles.username_key, '
     'public.profiles.username_prompted, public.profiles.username_tag' then
    raise exception 'FAIL (CHECK 7): the columns that name a username or a display name are %, expected profiles'' five username columns alone',
      seen;
  end if;
  raise notice 'OK (CHECK 7): profiles'' five username columns are the only ones, and display_name is gone';
end $$;

\echo '=== CHECK 8 (R1434): assign_default_username is a DEFINER trigger on profiles inserts that no client role may call or attach ==='
select t.tgname, t.tgenabled, p.prosecdef, p.proconfig
  from pg_trigger t join pg_proc p on p.oid = t.tgfoid
 where t.tgrelid = 'public.profiles'::regclass and t.tgname = 'profiles_assign_default_username';
do $$
declare
  r       record;
  grantee text;
begin
  select t.tgtype, t.tgenabled, p.prosecdef, p.proconfig, pn.nspname as pronsp, p.proname
    into r
    from pg_trigger t
    join pg_proc p on p.oid = t.tgfoid
    join pg_namespace pn on pn.oid = p.pronamespace
   where t.tgrelid = 'public.profiles'::regclass and t.tgname = 'profiles_assign_default_username';
  if not found then
    raise exception 'FAIL (CHECK 8): public.profiles has no profiles_assign_default_username trigger';
  end if;
  if r.pronsp <> 'app' or r.proname <> 'assign_default_username' then
    raise exception 'FAIL (CHECK 8): the trigger fires %.%, not app.assign_default_username', r.pronsp, r.proname;
  end if;
  -- tgtype bits: 1 = FOR EACH ROW, 2 = BEFORE, 4 = INSERT, 8 = DELETE, 16 = UPDATE.
  if r.tgtype & 1 = 0 or r.tgtype & 2 = 0 or r.tgtype & 4 = 0 or r.tgtype & (8 | 16) <> 0 then
    raise exception 'FAIL (CHECK 8): the trigger is not BEFORE INSERT FOR EACH ROW alone (tgtype %)', r.tgtype;
  end if;
  if r.tgenabled not in ('O', 'A') then
    raise exception 'FAIL (CHECK 8): the trigger is disabled (tgenabled = %)', r.tgenabled;
  end if;
  if not r.prosecdef or not coalesce(r.proconfig, '{}'::text[]) @> array['search_path=""'] then
    raise exception 'FAIL (CHECK 8): app.assign_default_username is DEFINER % with proconfig %', r.prosecdef, r.proconfig;
  end if;

  -- A trigger fires without EXECUTE, so the grant guards only a direct call and CREATE TRIGGER,
  -- both of which need it: no client role may run the owner's read of every username itself.
  foreach grantee in array array['anon', 'authenticated', 'public'] loop
    if has_function_privilege(grantee, 'app.assign_default_username()', 'EXECUTE') then
      raise exception 'FAIL (CHECK 8): % may execute app.assign_default_username()', grantee;
    end if;
  end loop;

  raise notice 'OK (CHECK 8): a BEFORE INSERT row trigger, DEFINER with search_path pinned; anon, authenticated and PUBLIC hold no EXECUTE';
end $$;

-- What the missing EXECUTE refuses, as a client: the call, and attaching the function to a table of
-- its own (a temporary one; a client may create no other), which would run it as the owner.
begin;
set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
create temporary table check8_probe (username_base text, username_key text, username_tag int) on commit drop;
do $$
declare
  stmt text;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (CHECK 8): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  foreach stmt in array array[
    'select app.assign_default_username()',
    'create trigger check8_probe_named before insert on check8_probe for each row '
      || 'execute function app.assign_default_username()'] loop
    begin
      execute stmt;
      raise exception 'FAIL (CHECK 8): authenticated ran "%"', stmt;
    exception
      when insufficient_privilege then
        null;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (CHECK 8): "%" raised "%" (%), not insufficient_privilege', stmt, sqlerrm, sqlstate;
    end;
  end loop;
  raise notice 'OK (CHECK 8): authenticated may neither call app.assign_default_username nor attach it to its own table';
end $$;
reset role;
rollback;

\echo '=== CHECK 9 (R1434): re-applying 0028 renames nobody (rolled back) ==='
begin;
do $$
begin
  perform set_config('r1434.before',
    (select string_agg(format('%s %s %s %s %s %s %s', id, username_base, username_key, username_tag,
                              username_changed_at, username_prompted, updated_at), '; ' order by id)
       from public.profiles), true);
  perform set_config('r1434.trigger_oid',
    (select oid::text from pg_trigger
      where tgrelid = 'public.profiles'::regclass and tgname = 'profiles_assign_default_username'), true);
end $$;
\i /tmp/0028_usernames.sql
do $$
declare
  was text := current_setting('r1434.before', true);
  now_ text;
begin
  select string_agg(format('%s %s %s %s %s %s %s', id, username_base, username_key, username_tag,
                           username_changed_at, username_prompted, updated_at), '; ' order by id)
    into now_ from public.profiles;
  if was is null or now_ is distinct from was then
    raise exception 'FAIL (CHECK 9): re-applying 0028 changed the profiles (% -> %)', was, now_;
  end if;
  -- Vacuity guard: the file really ran, since it drops and recreates its trigger.
  if (select oid::text from pg_trigger
       where tgrelid = 'public.profiles'::regclass and tgname = 'profiles_assign_default_username')
     is not distinct from current_setting('r1434.trigger_oid', true) then
    raise exception 'FAIL (CHECK 9): the trigger is the one from before, so 0028 did not run again';
  end if;
  raise notice 'OK (CHECK 9): a second run of 0028 left every username, and every updated_at, as it was';
end $$;
rollback;

\echo '=== ALL USERNAME CHECKS RAN ==='
