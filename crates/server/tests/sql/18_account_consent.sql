-- Consent on the account (migration 0034, SPEC §9.4, R757000, R757001; issue #757). Runs after
-- 17_ladder_bots.sql; profile 1 is active by then (03 activated it).
\set ON_ERROR_STOP on

-- Same rules as 01-17: every check raises on failure (`raise exception 'FAIL (R…): …'`), every
-- expected refusal is matched on its sqlstate and on the object it names, each check that could pass
-- vacuously carries a guard, and every block runs inside a transaction that is rolled back.

\echo '### R757001: 0034 gives every person''s account that predates it a baseline record ###'
begin;
-- An account that "predates" 0034: signed up in 2020, with no record. 0034 is applied again over
-- it, as a database that held it when 0034 first ran saw it, beside a bot's profile.
insert into auth.users (id, email, email_confirmed_at)
values ('b7570000-0000-4000-8000-000000000001', 'old757@example.test', now());
update public.profiles set created_at = '2020-01-02 03:04:05+00'
 where id = 'b7570000-0000-4000-8000-000000000001';
delete from public.terms_acceptances where profile_id = 'b7570000-0000-4000-8000-000000000001';
insert into public.profiles (id, kind, status) values ('b7570000-0000-4000-8000-000000000002', 'bot', 'active');
\i /tmp/0034_account_consent.sql
-- Twice: a second application writes nothing more.
\i /tmp/0034_account_consent.sql
do $$
declare
  old constant uuid := 'b7570000-0000-4000-8000-000000000001';
  bot constant uuid := 'b7570000-0000-4000-8000-000000000002';
  n   int;
  r   record;
begin
  select count(*) into n from public.profiles where id = old and kind = 'human';
  if n <> 1 then
    raise exception 'FAIL (R757001): signup made % human profiles, not 1; the checks below would prove nothing', n;
  end if;
  select count(*) into n from public.terms_acceptances where profile_id = old;
  if n <> 1 then
    raise exception 'FAIL (R757001): the account that predates 0034 has % records, not 1', n;
  end if;
  select terms_version, source, accepted_at into r from public.terms_acceptances where profile_id = old;
  if r.terms_version <> 'baseline' or r.source <> 'baseline' then
    raise exception 'FAIL (R757001): the baseline record is (%, %), not (baseline, baseline)', r.terms_version, r.source;
  end if;
  if r.accepted_at <> '2020-01-02 03:04:05+00'::timestamptz then
    raise exception 'FAIL (R757001): the baseline record is dated %, not when the account was created', r.accepted_at;
  end if;
  select count(*) into n from public.terms_acceptances where profile_id = bot;
  if n <> 0 then
    raise exception 'FAIL (R757001): a ladder bot''s profile got % terms records; only people agree to terms', n;
  end if;
  select count(*) into n from public.age_checks where profile_id = old;
  if n <> 0 then
    raise exception 'FAIL (R757000): 0034 recorded an age check nobody answered';
  end if;
  raise notice 'OK (R757001): one baseline record per person''s account, dated at sign-up, none for a bot, and none twice';
end $$;
rollback;

\echo '### R757000: no client may read or write public.age_checks ###'
\echo '### R757001: no client may read or write public.terms_acceptances ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, statement, the object a refusal names]. Every statement is one the owner can run.
  probes constant text[][] := array[
    ['age_checks SELECT', $q$select count(*) from public.age_checks$q$, 'age_checks'],
    ['age_checks INSERT',
     $q$insert into public.age_checks (profile_id, min_age) values ('11111111-1111-1111-1111-111111111111', 13)$q$,
     'age_checks'],
    ['age_checks UPDATE', $q$update public.age_checks set min_age = 13$q$, 'age_checks'],
    ['age_checks DELETE', $q$delete from public.age_checks$q$, 'age_checks'],
    ['terms_acceptances SELECT', $q$select count(*) from public.terms_acceptances$q$, 'terms_acceptances'],
    ['terms_acceptances INSERT',
     $q$insert into public.terms_acceptances (profile_id, terms_version, source)
        values ('11111111-1111-1111-1111-111111111111', 'v-client', 'signup')$q$,
     'terms_acceptances'],
    ['terms_acceptances UPDATE', $q$update public.terms_acceptances set source = 'signup'$q$, 'terms_acceptances'],
    ['terms_acceptances DELETE', $q$delete from public.terms_acceptances$q$, 'terms_acceptances']];
  r        text;
  t        text;
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
      raise exception 'FAIL (R757000): running as %, not % — SET LOCAL ROLE did not take', current_user, r;
    end if;
    if r = 'authenticated' then
      -- The session still reads its own profile, so a refusal below is about the two tables.
      select count(*) into readable from public.profiles where id = p1 and status = 'active';
      if readable <> 1 then
        raise exception 'FAIL (R757000): profile 1 cannot read its own profile row; the refusals below would prove nothing';
      end if;
    end if;

    refused := 0;
    for i in 1 .. array_length(probes, 1) loop
      begin
        execute probes[i][2];
        raise exception 'FAIL (R757001): % ran % — what an account agreed to is the server''s alone to record', r, probes[i][1];
      exception
        when insufficient_privilege then
          if sqlerrm not like '%' || probes[i][3] || '%' then
            raise exception 'FAIL (R757001): % was refused % by "%", which does not name %',
              r, probes[i][1], sqlerrm, probes[i][3];
          end if;
          refused := refused + 1;
        when others then
          if sqlerrm like 'FAIL%' then raise; end if;
          raise exception 'FAIL (R757001): % % raised "%" (%), not insufficient_privilege',
            r, probes[i][1], sqlerrm, sqlstate;
      end;
    end loop;
    reset role;
    raise notice 'OK (R757000, R757001): % was refused all % statements', r, refused;
  end loop;

  -- And the policy side: RLS on, no policy, no privilege for anon, authenticated or PUBLIC.
  foreach t in array array['age_checks', 'terms_acceptances'] loop
    if not (select relrowsecurity from pg_class where oid = ('public.' || t)::regclass) then
      raise exception 'FAIL (R757001): row-level security is off on public.%', t;
    end if;
    if exists (select 1 from pg_policies where schemaname = 'public' and tablename = t) then
      raise exception 'FAIL (R757001): public.% has a policy; it must have none', t;
    end if;
    foreach r in array array['anon', 'authenticated', 'public'] loop
      if exists (select 1 from information_schema.role_table_grants
                  where table_schema = 'public' and table_name = t and grantee = r) then
        raise exception 'FAIL (R757001): % holds a privilege on public.%', r, t;
      end if;
    end loop;
  end loop;
  raise notice 'OK (R757000, R757001): both tables have RLS and no policy or client grant';
end $$;
rollback;

\echo '### R757000: an age check is one per account, against a minimum that could be one ###'
begin;
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  nobody  constant uuid := 'b7570000-0000-4000-8000-0000000000ff';
  v_min   smallint;
begin
  insert into public.age_checks (profile_id, min_age) values (p1, 13);
  select min_age into v_min from public.age_checks where profile_id = p1;
  if v_min is distinct from 13::smallint then
    raise exception 'FAIL (R757000): the age check was not stored; the refusals below would prove nothing';
  end if;

  begin
    insert into public.age_checks (profile_id, min_age) values (p1, 16);
    raise exception 'FAIL (R757000): a second age check was stored for one account';
  exception
    when unique_violation then
      if sqlerrm not like '%age_checks_pkey%' then
        raise exception 'FAIL (R757000): the refusal "%" does not name age_checks_pkey', sqlerrm;
      end if;
  end;

  foreach v_min in array array[0, 121]::smallint[] loop
    begin
      update public.age_checks set min_age = v_min where profile_id = p1;
      raise exception 'FAIL (R757000): a minimum age of % was stored', v_min;
    exception
      when check_violation then
        if sqlerrm not like '%age_checks_min_age_check%' then
          raise exception 'FAIL (R757000): the refusal "%" does not name age_checks_min_age_check', sqlerrm;
        end if;
    end;
  end loop;

  begin
    insert into public.age_checks (profile_id, min_age) values (nobody, 13);
    raise exception 'FAIL (R757000): an age check for no account was stored';
  exception
    when foreign_key_violation then
      if sqlerrm not like '%age_checks%' then
        raise exception 'FAIL (R757000): the refusal "%" does not name age_checks', sqlerrm;
      end if;
  end;
  raise notice 'OK (R757000): one age check per account, its minimum 1-120, its account real';
end $$;
rollback;

\echo '### R757001: each version is agreed to once per account, by one of three sources ###'
begin;
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  nobody  constant uuid := 'b7570000-0000-4000-8000-0000000000ff';
  n       int;
  v_text  text;
begin
  insert into public.terms_acceptances (profile_id, terms_version, source) values (p1, '2026-10-13/2026-10-13', 'signup');
  insert into public.terms_acceptances (profile_id, terms_version, source) values (p1, '2027-01-01/2026-10-13', 'reaccept');
  select count(*) into n from public.terms_acceptances where profile_id = p1 and terms_version <> 'baseline';
  if n <> 2 then
    raise exception 'FAIL (R757001): % of 2 versions were stored; the refusals below would prove nothing', n;
  end if;

  begin
    insert into public.terms_acceptances (profile_id, terms_version, source) values (p1, '2026-10-13/2026-10-13', 'reaccept');
    raise exception 'FAIL (R757001): one version was agreed to twice by one account';
  exception
    when unique_violation then
      if sqlerrm not like '%terms_acceptances_pkey%' then
        raise exception 'FAIL (R757001): the refusal "%" does not name terms_acceptances_pkey', sqlerrm;
      end if;
  end;

  foreach v_text in array array['', repeat('v', 65)] loop
    begin
      insert into public.terms_acceptances (profile_id, terms_version, source) values (p1, v_text, 'signup');
      raise exception 'FAIL (R757001): a version % characters long was stored', char_length(v_text);
    exception
      when check_violation then
        if sqlerrm not like '%terms_acceptances_version_check%' then
          raise exception 'FAIL (R757001): the refusal "%" does not name terms_acceptances_version_check', sqlerrm;
        end if;
    end;
  end loop;

  begin
    insert into public.terms_acceptances (profile_id, terms_version, source) values (p1, 'v-implied', 'implied');
    raise exception 'FAIL (R757001): a source other than signup, reaccept or baseline was stored';
  exception
    when check_violation then
      if sqlerrm not like '%terms_acceptances_source_check%' then
        raise exception 'FAIL (R757001): the refusal "%" does not name terms_acceptances_source_check', sqlerrm;
      end if;
  end;

  begin
    insert into public.terms_acceptances (profile_id, terms_version, source) values (nobody, 'v1', 'signup');
    raise exception 'FAIL (R757001): a record for no account was stored';
  exception
    when foreign_key_violation then
      if sqlerrm not like '%terms_acceptances%' then
        raise exception 'FAIL (R757001): the refusal "%" does not name terms_acceptances', sqlerrm;
      end if;
  end;
  raise notice 'OK (R757001): one record per account and version, versions 1-64 characters, three sources, a real account';
end $$;
rollback;
