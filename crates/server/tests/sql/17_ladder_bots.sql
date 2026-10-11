-- The ladder bots (migration 0032, SPEC §9.12, R1448, R1449, R1452; issue #638). Runs last, after
-- 16_play_telemetry.sql; profile 1 is active by then (03 activated it).
\set ON_ERROR_STOP on

-- Same rules as 01-16: every check raises on failure (`raise exception 'FAIL (R…): …'`), every
-- expected refusal is matched on its sqlstate and on the object it names, each check that could pass
-- vacuously carries a guard, and every block runs inside a transaction that is rolled back.

\echo '### R1452: no client may read profiles.kind or public.bots ###'
begin;
-- Something to refuse, as the owner: a bot profile with its row.
insert into public.profiles (id, kind, status) values ('b1444000-0000-4000-8000-000000000001', 'bot', 'active');
insert into public.bots (profile_id, strength, speed, persona, active_from_hour, active_hours)
values ('b1444000-0000-4000-8000-000000000001', 2, 1.0, 'balanced', 8, 12);

do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, statement, the object a refusal names]. Every statement is one the owner can run.
  probes constant text[][] := array[
    ['profiles.kind SELECT', $q$select kind from public.profiles$q$, 'profiles'],
    ['bots SELECT', $q$select count(*) from public.bots$q$, 'bots'],
    ['bots INSERT',
     $q$insert into public.bots (profile_id, strength, speed, persona, active_from_hour, active_hours)
        values ('11111111-1111-1111-1111-111111111111', 0, 1.0, 'silent', 0, 24)$q$,
     'bots'],
    ['bots UPDATE', $q$update public.bots set retired_at = now()$q$, 'bots'],
    ['bots DELETE', $q$delete from public.bots$q$, 'bots']];
  r        text;
  i        int;
  refused  int;
  readable bigint;
begin
  -- Vacuity guard: each statement runs for the owner, in a subtransaction undone at once, so a
  -- refusal below is the privilege system and not a statement nobody could run.
  for i in 1 .. array_length(probes, 1) loop
    begin
      execute probes[i][2];
      raise exception 'rollback-probe';
    exception
      when raise_exception then
        if sqlerrm <> 'rollback-probe' then raise; end if;
    end;
  end loop;

  foreach r in array array['anon', 'authenticated'] loop
    execute format('set local role %I', r);
    perform set_config('request.jwt.claim.sub', case r when 'authenticated' then p1::text else '' end, true);
    if current_user <> r then
      raise exception 'FAIL (R1452): running as %, not % — SET LOCAL ROLE did not take', current_user, r;
    end if;
    if r = 'authenticated' then
      -- The session still reads its own profile's granted columns, so a refusal below is about
      -- `kind` and `bots`.
      select count(*) into readable from public.profiles where id = p1 and status = 'active';
      if readable <> 1 then
        raise exception 'FAIL (R1452): profile 1 cannot read its own profile row; the refusals below would prove nothing';
      end if;
    end if;

    refused := 0;
    for i in 1 .. array_length(probes, 1) loop
      begin
        execute probes[i][2];
        raise exception 'FAIL (R1452): % ran % — whether a profile is a bot is the server''s alone', r, probes[i][1];
      exception
        when insufficient_privilege then
          if sqlerrm not like '%' || probes[i][3] || '%' then
            raise exception 'FAIL (R1452): % was refused % by "%", which does not name %',
              r, probes[i][1], sqlerrm, probes[i][3];
          end if;
          refused := refused + 1;
        when others then
          if sqlerrm like 'FAIL%' then raise; end if;
          raise exception 'FAIL (R1452): % % raised "%" (%), not insufficient_privilege',
            r, probes[i][1], sqlerrm, sqlstate;
      end;
    end loop;
    reset role;
    raise notice 'OK (R1452): % was refused all % statements', r, refused;
  end loop;

  -- And the policy side: RLS on, no policy, no privilege for anon, authenticated or PUBLIC.
  if not (select relrowsecurity from pg_class where oid = 'public.bots'::regclass) then
    raise exception 'FAIL (R1452): row-level security is off on public.bots';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'bots') then
    raise exception 'FAIL (R1452): public.bots has a policy; it must have none';
  end if;
  foreach r in array array['anon', 'authenticated', 'public'] loop
    if exists (select 1 from information_schema.role_table_grants
                where table_schema = 'public' and table_name = 'bots' and grantee = r) then
      raise exception 'FAIL (R1452): % holds a privilege on public.bots', r;
    end if;
  end loop;
  if exists (select 1 from information_schema.column_privileges
              where table_schema = 'public' and table_name = 'profiles' and column_name = 'kind'
                and grantee in ('anon', 'authenticated', 'PUBLIC')) then
    raise exception 'FAIL (R1452): a client role holds a privilege on profiles.kind';
  end if;
  raise notice 'OK (R1452): public.bots has RLS and no policy or client grant, and profiles.kind no client grant';
end $$;
rollback;

\echo '### R1452: the auth user''s foreign key gives way to two triggers ###'
begin;
do $$
declare
  gone   constant uuid := 'b1444000-0000-4000-8000-000000000002';
  orphan constant uuid := 'b1444000-0000-4000-8000-000000000003';
  bot    constant uuid := 'b1444000-0000-4000-8000-000000000004';
  n      int;
  v_tag  int;
begin
  -- Deleting an auth user still deletes its profile, as the key's cascade did.
  insert into auth.users (id, email, email_confirmed_at) values (gone, 'gone1444@example.test', now());
  select count(*) into n from public.profiles where id = gone and kind = 'human';
  if n <> 1 then
    raise exception 'FAIL (R1452): signup made % human profiles, not 1; the delete below would prove nothing', n;
  end if;
  delete from auth.users where id = gone;
  select count(*) into n from public.profiles where id = gone;
  if n <> 0 then
    raise exception 'FAIL (R1452): deleting the auth user left its profile';
  end if;

  -- A person's profile with no auth user is refused, as the key refused it.
  begin
    insert into public.profiles (id, status) values (orphan, 'pending');
    raise exception 'FAIL (R1452): a human profile with no auth user was inserted';
  exception
    when foreign_key_violation then
      if sqlerrm not like '%' || orphan::text || '%' then
        raise exception 'FAIL (R1452): the refusal "%" does not name the profile', sqlerrm;
      end if;
  end;

  -- A bot's profile has none, and is named the lowest free Player#n as any new profile (R1434).
  insert into public.profiles (id, kind, status) values (bot, 'bot', 'active');
  select username_tag into v_tag from public.profiles where id = bot and username_key = 'player';
  if v_tag is null then
    raise exception 'FAIL (R1452): the bot profile was not given a default Player#n';
  end if;
  insert into public.bots (profile_id, strength, speed, persona, active_from_hour, active_hours)
  values (bot, 0, 1.0, 'polite', 0, 24);

  -- Its row goes with its profile.
  delete from public.profiles where id = bot;
  select count(*) into n from public.bots where profile_id = bot;
  if n <> 0 then
    raise exception 'FAIL (R1449): deleting the bot''s profile left its bots row';
  end if;

  -- And `kind` holds only its two values.
  begin
    insert into public.profiles (id, kind, status) values (orphan, 'robot', 'active');
    raise exception 'FAIL (R1452): a profile of kind robot was inserted';
  exception
    when check_violation then
      if sqlerrm not like '%profiles_kind_check%' then
        raise exception 'FAIL (R1452): the refusal "%" does not name profiles_kind_check', sqlerrm;
      end if;
  end;
  raise notice 'OK (R1452): an auth user''s delete takes its profile, a human profile needs one, a bot''s does not';
end $$;
rollback;

\echo '### R1449: a bot owns no collection, whatever grants the launch collection ###'
begin;
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  bot     constant uuid := 'b1446000-0000-4000-8000-000000000001';
  v_cards bigint;
  n       bigint;
begin
  -- Vacuity guard: the launch backfill has cards to grant, and grants them to a person.
  select count(*) into v_cards from public.cards
   where token = false and catalog_version = app.catalog_version();
  if v_cards = 0 then
    raise exception 'FAIL (R1449): the current catalog holds no non-Token card; the backfill below would grant nothing to anyone';
  end if;

  insert into public.profiles (id, kind, status) values (bot, 'bot', 'active');
  perform app.grant_launch_collection_all();
  select count(*) into n from public.collection where profile_id = p1 and quantity >= 1;
  if n <> v_cards then
    raise exception 'FAIL (R1449): profile 1 owns % of the % cards after the backfill; it must still grant people', n, v_cards;
  end if;
  select count(*) into n from public.collection where profile_id = bot;
  if n <> 0 then
    raise exception 'FAIL (R1449): the backfill (0016 runs it on every catalog stamp) granted the bot % cards', n;
  end if;
  select count(*) into n from public.collection_grants where profile_id = bot;
  if n <> 0 then
    raise exception 'FAIL (R1449): the bot has % collection_grants rows', n;
  end if;
  raise notice 'OK (R1449): the launch backfill grants people and passes over the bot';
end $$;
rollback;
