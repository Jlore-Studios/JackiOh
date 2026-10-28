-- A spend's idempotency key and the answer it committed (migration 0067, SPEC §14.11; R2124000;
-- issue #2124). Runs after 35_game_totals.sql; profiles 1 and 2 (11111111-…, 22222222-…) exist by
-- then. The client's refusal on idempotency_keys is also 02_rls_as_client.sql's CHECK 3, with the
-- other server-only tables.
\set ON_ERROR_STOP on

-- Same rules as 01-35: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate (and, for a privilege, on the table it names), each
-- check that could pass vacuously carries a guard, and every block that writes runs inside a
-- transaction that is rolled back. Each SPEC §11 row this file proves is named in a
-- `### Rnnn: … ###` heading.

\echo '### R2124000: public.idempotency_keys keys one answer per account and key, follows its account, and is the server''s alone ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  if not exists (select 1 from pg_class
                  where oid = 'public.idempotency_keys'::regclass and relrowsecurity) then
    raise exception 'FAIL (R2124000): public.idempotency_keys has no row level security';
  end if;

  -- The columns, as the store reads and writes them.
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'idempotency_keys';
  if v_text is distinct from 'profile_id uuid NO, key uuid NO, route text NO, body_hash text NO, '
                             'status smallint NO, body jsonb NO, created_at timestamp with time zone NO' then
    raise exception 'FAIL (R2124000): public.idempotency_keys has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- One answer per account and key: a key is its account's, never global.
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.idempotency_keys'::regclass and c.contype = 'p';
  if v_text is distinct from 'profile_id, key' then
    raise exception 'FAIL (R2124000): the primary key is (%), expected (profile_id, key)', coalesce(v_text, 'none');
  end if;

  -- Its one foreign key is its account's, cascading (0012's rule).
  select string_agg(format('%s %s', c.confrelid::regclass, c.confdeltype), ', ') into v_text
    from pg_constraint c
   where c.conrelid = 'public.idempotency_keys'::regclass and c.contype = 'f';
  if v_text is distinct from 'profiles c' then
    raise exception 'FAIL (R2124000): the foreign keys are [%], expected the profile''s alone, on delete cascade',
      coalesce(v_text, '(none)');
  end if;

  -- The sweep's index (`created_at < cutoff` over every account).
  if not exists (select 1 from pg_indexes
                  where schemaname = 'public' and tablename = 'idempotency_keys'
                    and indexname = 'idempotency_keys_created_at') then
    raise exception 'FAIL (R2124000): idempotency_keys_created_at, the sweep''s index, is missing';
  end if;

  -- No policy and no client privilege: a client that read a row could replay another's answer,
  -- and one that wrote a row could forge one.
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'idempotency_keys') then
    raise exception 'FAIL (R2124000): idempotency_keys has a policy, so a client could reach it';
  end if;
  foreach who in array array['authenticated', 'anon'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.idempotency_keys', op) then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R2124000): a client holds [%] on idempotency_keys', array_to_string(wrong, ', ');
  end if;
  raise notice 'OK (R2124000): idempotency_keys has RLS on, its columns, one row per (profile_id, key), one foreign key, its account''s, cascading, the sweep''s index, and no policy or client privilege';
end $$;

\echo '### R2124000: the checks refuse a status that is no success and a body hash that is no SHA-256 in hex; a key is held once per account ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  k  constant uuid := 'a2124000-0000-4000-8000-000000000001';
  -- [what, status, body_hash]. Each breaks one of 0067's checks and nothing else.
  refused constant text[][] := array[
    ['a refusal''s 404', '404', repeat('ab', 32)],
    ['a 199', '199', repeat('ab', 32)],
    ['a 300', '300', repeat('ab', 32)],
    ['a hash in upper case', '200', repeat('AB', 32)],
    ['a hash of 62 digits', '200', repeat('ab', 31)],
    ['a hash of 66 digits', '200', repeat('ab', 33)],
    ['a hash that is no hex', '200', repeat('zz', 32)]];
  i int;
begin
  if not exists (select 1 from public.profiles where id = p1) or not exists (select 1 from public.profiles where id = p2) then
    raise exception 'FAIL (R2124000): profile 1 or 2 is missing, so every insert below would fail on its foreign key';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
      values (p1, k, '/api/packs/buy', refused[i][3], refused[i][2]::smallint, '{}', now());
      raise exception 'FAIL (R2124000): the schema took %', refused[i][1];
    exception
      when check_violation then null;
    end;
  end loop;

  -- The vacuity guard: what the store writes is taken, at the bounds of the status.
  insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
  values (p1, k, '/api/packs/buy', repeat('0123456789abcdef', 4), 200,
          '{"pack": "Core", "jlore": 50, "unopened": 1}', now()),
         (p1, 'a2124000-0000-4000-8000-000000000002', '/api/codes/redeem', repeat('f', 64), 299, '{}', now());
  if (select count(*) from public.idempotency_keys where profile_id = p1) <> 2 then
    raise exception 'FAIL (R2124000): the two rows the checks allow were not both written';
  end if;

  -- One answer per account and key.
  begin
    insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
    values (p1, k, '/api/packs/open', repeat('ab', 32), 200, '{}', now());
    raise exception 'FAIL (R2124000): a second row for one account and key was taken';
  exception
    when unique_violation then null;
  end;
  -- The same key is another account's to send.
  insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
  values (p2, k, '/api/packs/buy', repeat('ab', 32), 200, '{}', now());
  if (select count(*) from public.idempotency_keys where key = k) <> 2 then
    raise exception 'FAIL (R2124000): the same key under two accounts is not two rows';
  end if;
  raise notice 'OK (R2124000): % rows the checks refuse are refused, a second row for one account and key too, the same key under another account is taken, and the rows at the bounds are',
    array_length(refused, 1);
end $$;
rollback;

\echo '### R2124000: a client reads and writes no idempotency key, its own included ###'
begin;
insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
values ('11111111-1111-1111-1111-111111111111', 'a2124000-0000-4000-8000-000000000003', '/api/packs/buy',
        repeat('ab', 32), 200, '{}', now());

set local role authenticated;
do $$ begin perform set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true); end $$;
do $$
declare
  probe  text;
  probes constant text[][] := array[
    ['idempotency_keys SELECT', $q$select count(*) from public.idempotency_keys$q$],
    ['idempotency_keys INSERT',
     $q$insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
        values ('11111111-1111-1111-1111-111111111111', 'a2124000-0000-4000-8000-000000000004', '/api/packs/buy',
                repeat('ab', 32), 200, '{}', now())$q$],
    ['idempotency_keys UPDATE', $q$update public.idempotency_keys set body = '{"forged": true}'$q$],
    ['idempotency_keys DELETE', $q$delete from public.idempotency_keys$q$]];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R2124000): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R2124000): % succeeded as a client — idempotency keys are the server''s alone', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%idempotency_keys%' then
          raise exception 'FAIL (R2124000): % was refused by "%" (%), which does not name idempotency_keys',
            probe, sqlerrm, sqlstate;
        end if;
        raise notice 'OK (R2124000): % refused — % (%)', probe, sqlerrm, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R2124000): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;
end $$;
reset role;
rollback;

\echo '### R2124000: a deleted account takes its idempotency keys with it, and nobody else''s ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e2124000-0000-4000-8000-000000000001', 'r2124000-leaver@example.test', now());
do $$
declare
  gone constant uuid := 'e2124000-0000-4000-8000-000000000001';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
  k    constant uuid := 'a2124000-0000-4000-8000-000000000005';
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R2124000): the leaver has no profile; the cascade below would prove nothing';
  end if;
  insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
  values (gone, k, '/api/packs/buy', repeat('ab', 32), 200, '{}', now()),
         (p1, k, '/api/packs/buy', repeat('ab', 32), 200, '{}', now());

  delete from auth.users where id = gone;

  if exists (select 1 from public.idempotency_keys where profile_id = gone) then
    raise exception 'FAIL (R2124000): a deleted account''s idempotency keys outlived it';
  end if;
  if not exists (select 1 from public.idempotency_keys where profile_id = p1 and key = k) then
    raise exception 'FAIL (R2124000): deleting one account took another''s idempotency key';
  end if;
  raise notice 'OK (R2124000): deleting an account deletes its idempotency keys and no other account''s';
end $$;
rollback;

\echo '### R2124000: 0067 re-applies over rows of its own and changes none ###'
begin;
insert into public.idempotency_keys (profile_id, key, route, body_hash, status, body, created_at)
values ('11111111-1111-1111-1111-111111111111', 'a2124000-0000-4000-8000-000000000006', '/api/packs/buy',
        repeat('ab', 32), 200, '{}', now());
create temp table r2124000_rows (n bigint not null) on commit drop;
insert into r2124000_rows select count(*) from public.idempotency_keys;
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0067_idempotency_keys.sql
set local client_min_messages = notice;
do $$
begin
  if (select n from r2124000_rows) <> 1 then
    raise exception 'FAIL (R2124000): % keys before the re-apply, expected the one written', (select n from r2124000_rows);
  end if;
  if (select count(*) from public.idempotency_keys) <> (select n from r2124000_rows) then
    raise exception 'FAIL (R2124000): re-applying 0067 changed the keys';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'idempotency_keys') then
    raise exception 'FAIL (R2124000): re-applying 0067 left a policy on idempotency_keys';
  end if;
  raise notice 'OK (R2124000): 0067 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
