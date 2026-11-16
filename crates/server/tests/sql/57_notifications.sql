-- The in-game notification feed (migration 0093, SPEC §9.4, §9.13; R4039000; issue #4039). Runs after
-- 56_daily_email_days.sql; profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03 activated
-- them). A feed row is the server's alone: RLS on, no policy, no client privilege, a kind among the
-- eight, and a profile's deletion takes the rows naming it either way.
\set ON_ERROR_STOP on

-- Same rules as 01-56: every check raises on failure (`raise exception 'FAIL (…): …'`), every expected
-- refusal is matched on its sqlstate, each check that could pass vacuously carries a guard, and every
-- block that writes runs inside a transaction that is rolled back. Each heading names the ruling this
-- file proves, as `### R4039000: … ###`.

\echo '### R4039000: public.notifications has its columns and is the server''s alone ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  if not exists (select 1 from pg_class where oid = 'public.notifications'::regclass and relrowsecurity) then
    raise exception 'FAIL (R4039000): public.notifications has no row level security';
  end if;
  select string_agg(policyname, ', ') into v_text from pg_policies
   where schemaname = 'public' and tablename = 'notifications';
  if v_text is not null then
    raise exception 'FAIL (R4039000): public.notifications has the policies [%], expected none', v_text;
  end if;
  foreach who in array array['authenticated', 'anon', 'public'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.notifications', op) then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R4039000): the clients hold [%] on public.notifications', array_to_string(wrong, ', ');
  end if;

  -- The columns, as the store reads and writes them (src/notifications/store.rs).
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text from information_schema.columns
   where table_schema = 'public' and table_name = 'notifications';
  if v_text is distinct from 'id bigint NO, profile_id uuid NO, kind text NO, actor_id uuid YES, subject text YES, '
                             'payload jsonb NO, created_at bigint NO, read_at bigint YES' then
    raise exception 'FAIL (R4039000): public.notifications has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- Both profile references cascade.
  if (select confdeltype from pg_constraint where conname = 'notifications_profile_id_fkey') is distinct from 'c'
     or (select confdeltype from pg_constraint where conname = 'notifications_actor_id_fkey') is distinct from 'c' then
    raise exception 'FAIL (R4039000): public.notifications'' profile references do not cascade';
  end if;

  raise notice 'OK (R4039000): public.notifications has its columns, RLS on, cascading references, and no policy or client privilege';
end $$;

\echo '### R4039000: a kind outside the eight is refused, a client reads and writes none, and a deletion takes the rows naming it ###'
begin;
-- The leaver, whose profile the signup trigger makes (CHECK 8), is profile 1's actor once and told once.
insert into auth.users (id, email, email_confirmed_at)
values ('e4039000-0000-4000-8000-000000000001', 'r4039-leaver@example.test', now());
insert into public.notifications (profile_id, kind, actor_id, subject, payload, created_at)
values ('11111111-1111-1111-1111-111111111111', 'dailyMove', 'e4039000-0000-4000-8000-000000000001',
        'e4039000-0000-4000-8000-0000000000a1', '{}', 1),
       ('e4039000-0000-4000-8000-000000000001', 'friendAccepted', '11111111-1111-1111-1111-111111111111',
        'e4039000-0000-4000-8000-0000000000a2', '{}', 2),
       ('11111111-1111-1111-1111-111111111111', 'friendRequest', '22222222-2222-2222-2222-222222222222',
        'e4039000-0000-4000-8000-0000000000a3', '{}', 3);

do $$
begin
  begin
    insert into public.notifications (profile_id, kind, created_at)
    values ('11111111-1111-1111-1111-111111111111', 'email', 3);
    raise exception 'FAIL (R4039000): the schema took a notification of kind "email"';
  exception when check_violation then
    null;
  end;
  -- Preflight: the writes the client is about to be refused are ones the owner can make, inside a
  -- subtransaction rolled back at once.
  begin
    update public.notifications set read_at = 4;
    delete from public.notifications;
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception
        'FAIL (R4039000): the owner could not make the writes the client is about to be refused ("%", %)',
        sqlerrm, sqlstate;
    end if;
  end;
  raise notice 'OK (R4039000): a kind outside the eight is refused, and the owner may write the rows';
end $$;

set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
do $$
declare
  probe  text;
  probes constant text[][] := array[
    ['SELECT', $q$select count(*) from public.notifications$q$],
    ['INSERT',
     $q$insert into public.notifications (profile_id, kind, created_at)
        values ('11111111-1111-1111-1111-111111111111', 'dailyMove', 5)$q$],
    ['UPDATE', $q$update public.notifications set read_at = 6$q$],
    ['DELETE', $q$delete from public.notifications$q$]];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R4039000): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R4039000): an authenticated % of public.notifications succeeded', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%notifications%' then
          raise exception 'FAIL (R4039000): % was refused by "%" (%), which names no notifications table', probe, sqlerrm, sqlstate;
        end if;
        raise notice 'OK (R4039000): an authenticated % of public.notifications is refused (%)', probe, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R4039000): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;
end $$;
reset role;

set local role anon;
do $$
declare
  n bigint;
begin
  if current_user <> 'anon' then
    raise exception 'FAIL (R4039000): running as %, not anon — SET LOCAL did not take', current_user;
  end if;
  begin
    select count(*) into n from public.notifications;
    raise exception 'FAIL (R4039000): anon read public.notifications (% rows)', n;
  exception when insufficient_privilege then
    raise notice 'OK (R4039000): an anon read of public.notifications is refused (%)', sqlstate;
  end;
end $$;
reset role;

-- The leaver is the actor of one of profile 1's rows and the recipient of another: deleting the account
-- (0012) takes both, and leaves profile 1's row from profile 2.
do $$
declare
  gone constant uuid := 'e4039000-0000-4000-8000-000000000001';
begin
  if not exists (select 1 from public.profiles where id = gone)
     or (select count(*) from public.notifications) <> 3 then
    raise exception 'FAIL (R4039000): the leaver or its rows are missing; the cascade below would prove nothing';
  end if;
  delete from auth.users where id = gone;
  if exists (select 1 from public.notifications where profile_id = gone or actor_id = gone) then
    raise exception 'FAIL (R4039000): a row naming a deleted profile outlived it';
  end if;
  if (select string_agg(subject, ', ') from public.notifications) is distinct from 'e4039000-0000-4000-8000-0000000000a3' then
    raise exception 'FAIL (R4039000): deleting one account took another''s rows [%]',
      (select string_agg(subject, ', ') from public.notifications);
  end if;
  raise notice 'OK (R4039000): a deleted account takes the rows that tell it and the rows it is the actor of, and no other';
end $$;
rollback;

\echo '### R4039000: 0093 re-applies over a row of its own and changes none ###'
begin;
insert into public.notifications (profile_id, kind, subject, payload, created_at)
values ('11111111-1111-1111-1111-111111111111', 'lobbyInvite', 'LOBBQQ', '{"format":"duel"}', 7);
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0093_notifications.sql
set local client_min_messages = notice;
do $$
begin
  if (select count(*) from public.notifications) <> 1 then
    raise exception 'FAIL (R4039000): % rows after the re-apply, expected the one written',
      (select count(*) from public.notifications);
  end if;
  if (select payload from public.notifications where subject = 'LOBBQQ') is distinct from '{"format":"duel"}'::jsonb then
    raise exception 'FAIL (R4039000): re-applying 0093 changed a row';
  end if;
  if (select count(*) from pg_constraint
       where conrelid = 'public.notifications'::regclass and conname = 'notifications_kind_check') <> 1 then
    raise exception 'FAIL (R4039000): re-applying 0093 left notifications_kind_check other than once';
  end if;
  if not exists (select 1 from pg_class where oid = 'public.notifications'::regclass and relrowsecurity) then
    raise exception 'FAIL (R4039000): re-applying 0093 turned row level security off';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'notifications') then
    raise exception 'FAIL (R4039000): re-applying 0093 left a policy on public.notifications';
  end if;
  raise notice 'OK (R4039000): 0093 re-applies over its own row and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
