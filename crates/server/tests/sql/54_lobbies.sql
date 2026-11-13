-- Private lobbies (migration 0090, SPEC §9.5, §13.9; R3730000; issue #3730). Runs after 53_daily_games.sql;
-- profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03 activated them). A lobby is the server's
-- alone: RLS on, no policy, no client privilege, and a profile's deletion takes the lobbies it hosts or sits in.
\set ON_ERROR_STOP on

-- Same rules as 01-53: every check raises on failure (`raise exception 'FAIL (…): …'`), every expected refusal is
-- matched on its sqlstate, each check that could pass vacuously carries a guard, and every block that writes runs
-- inside a transaction that is rolled back.

\echo '### R3730000: lobbies are the server''s alone ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  if not exists (select 1 from pg_class where oid = 'public.lobbies'::regclass and relrowsecurity) then
    raise exception 'FAIL (R3730000): public.lobbies has no row level security';
  end if;
  select string_agg(policyname, ', ') into v_text from pg_policies
   where schemaname = 'public' and tablename = 'lobbies';
  if v_text is not null then
    raise exception 'FAIL (R3730000): public.lobbies has the policies [%], expected none', v_text;
  end if;
  foreach who in array array['authenticated', 'anon', 'public'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.lobbies', op) then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R3730000): the clients hold [%] on public.lobbies', array_to_string(wrong, ', ');
  end if;

  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text from information_schema.columns
   where table_schema = 'public' and table_name = 'lobbies';
  if v_text is distinct from 'code text NO, host uuid NO, guest uuid YES, settings jsonb NO, host_pick jsonb YES, '
                             'guest_pick jsonb YES, host_ready boolean NO, guest_ready boolean NO, rev bigint NO, '
                             'catalog_version text NO, created_at timestamp with time zone NO, '
                             'expires_at timestamp with time zone NO, match_id uuid YES, series_id uuid YES, '
                             'started_at timestamp with time zone YES' then
    raise exception 'FAIL (R3730000): public.lobbies has the columns [%]', coalesce(v_text, '(none)');
  end if;

  raise notice 'OK (R3730000): public.lobbies has RLS on, no policy and no client privilege';
end $$;

\echo '### R3730000: an authenticated read of a lobby is refused, and a profile''s deletion takes its lobbies ###'
begin;
insert into public.lobbies (code, host, guest, settings, catalog_version, created_at, expires_at)
values ('LOBBAA', '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222',
        '{"format":"duel","mode":"bo1"}', 'check', now(), now() + interval '15 minutes');

do $$
begin
  begin
    insert into public.lobbies (code, host, guest, settings, catalog_version, created_at, expires_at)
    values ('LOBBAB', '11111111-1111-1111-1111-111111111111', '11111111-1111-1111-1111-111111111111',
            '{"format":"duel","mode":"bo1"}', 'check', now(), now());
    raise exception 'FAIL (R3730000): a lobby whose guest is its host was taken';
  exception when check_violation then
    null;
  end;
  raise notice 'OK (R3730000): a lobby''s guest is never its host';
end $$;

set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
do $$
declare
  n bigint;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R3730000): running as %, not authenticated', current_user;
  end if;
  begin
    select count(*) into n from public.lobbies;
    raise exception 'FAIL (R3730000): authenticated read public.lobbies (% rows)', n;
  exception when insufficient_privilege then
    raise notice 'OK (R3730000): an authenticated read of public.lobbies is refused (%)', sqlstate;
  end;
end $$;
reset role;

do $$
begin
  if (select confdeltype from pg_constraint where conname = 'lobbies_guest_fkey') is distinct from 'c'
     or (select confdeltype from pg_constraint where conname = 'lobbies_host_fkey') is distinct from 'c' then
    raise exception 'FAIL (R3730000): public.lobbies'' profile references do not cascade';
  end if;
  raise notice 'OK (R3730000): public.lobbies'' host and guest cascade with their profile';
end $$;
rollback;
