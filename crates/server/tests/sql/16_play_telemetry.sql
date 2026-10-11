-- Play telemetry (migration 0029, SPEC §9.11, R1442; issue #637). Runs last, after
-- 15_usernames.sql; profiles 1 and 2 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-15: every check raises on failure (`raise exception 'FAIL (R…): …'`), every
-- expected refusal is matched on its sqlstate and on the object it names, each check that could pass
-- vacuously carries a guard, and every block runs inside a transaction that is rolled back.
--
-- A seat's rows are summarised as `<table> <seat> <count>` (emotes, signals, timings), so each
-- assertion compares one string and a failure prints what was there.

\echo '### R1442: no client may read or write the play telemetry, or run its functions ###'
begin;
-- Something to refuse, as the owner: a finished match profile 1 played, with a row of profile 1's
-- seat in each of the three tables.
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, ceiling_at, started_at, ended_at)
values ('f1442000-0000-4000-8000-000000000001', 'over', 'seed-1442a',
        '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222', '[]', '[]',
        'core-1', now(), now(), now());
insert into public.action_timings (match_id, seq, seat, action_kind, legal_count, turn, think_ms,
                                   clock_left_ms, first_in_turn, rank_bucket, pilot)
values ('f1442000-0000-4000-8000-000000000001', 1, 'p1', 'endTurn', 3, 1, 1200, 60000, true, null, 'human');
insert into public.emote_events (match_id, ordinal, seat, emote_id, turn, trigger_event,
                                 ms_since_trigger, reply_to_opponent_ms, pilot)
values ('f1442000-0000-4000-8000-000000000001', 1, 'p1', 'greetings', 1, 'turnStarted', 800, null, 'human');
insert into public.match_signals (match_id, seat, conceded_turn, concede_eval_deficit, pilot)
values ('f1442000-0000-4000-8000-000000000001', 'p1', 7, 12.5, 'human');

do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, statement, the object a refusal names]. Every statement is one the owner can run.
  probes constant text[][] := array[
    ['action_timings SELECT', $q$select count(*) from public.action_timings$q$, 'action_timings'],
    ['action_timings INSERT',
     $q$insert into public.action_timings (match_id, seq, seat, action_kind, legal_count, turn, think_ms,
                                           first_in_turn, pilot)
        values ('f1442000-0000-4000-8000-000000000001', 2, 'p1', 'endTurn', 1, 1, 0, false, 'human')$q$,
     'action_timings'],
    ['action_timings UPDATE', $q$update public.action_timings set think_ms = 0$q$, 'action_timings'],
    ['action_timings DELETE', $q$delete from public.action_timings$q$, 'action_timings'],
    ['emote_events SELECT', $q$select count(*) from public.emote_events$q$, 'emote_events'],
    ['emote_events INSERT',
     $q$insert into public.emote_events (match_id, ordinal, seat, emote_id, turn, pilot)
        values ('f1442000-0000-4000-8000-000000000001', 2, 'p1', 'wow', 1, 'human')$q$,
     'emote_events'],
    ['emote_events UPDATE', $q$update public.emote_events set emote_id = 'wow'$q$, 'emote_events'],
    ['emote_events DELETE', $q$delete from public.emote_events$q$, 'emote_events'],
    ['match_signals SELECT', $q$select count(*) from public.match_signals$q$, 'match_signals'],
    ['match_signals INSERT',
     $q$insert into public.match_signals (match_id, seat, pilot)
        values ('f1442000-0000-4000-8000-000000000001', 'p2', 'human')$q$,
     'match_signals'],
    ['match_signals UPDATE', $q$update public.match_signals set conceded_turn = null$q$, 'match_signals'],
    ['match_signals DELETE', $q$delete from public.match_signals$q$, 'match_signals'],
    ['app.forget_play_telemetry', $q$select app.forget_play_telemetry()$q$, 'forget_play_telemetry'],
    ['app.purge_expired_rows (three cutoffs)',
     $q$select * from app.purge_expired_rows(now(), now(), now())$q$, 'purge_expired_rows']];
  r        text;
  i        int;
  refused  int;
  readable bigint;
begin
  -- Vacuity guard: each statement runs for the owner, in a subtransaction undone at once, so a
  -- refusal below is the privilege system and not a statement nobody could run. The trigger function
  -- is the one exception: the owner gets past EXECUTE to Postgres' own "only as a trigger".
  for i in 1 .. array_length(probes, 1) loop
    begin
      execute probes[i][2];
      raise exception 'rollback-probe';
    exception
      when raise_exception then
        if sqlerrm <> 'rollback-probe' then raise; end if;
      when feature_not_supported then
        if probes[i][3] <> 'forget_play_telemetry' then
          raise exception 'FAIL (R1442): the owner could not run % ("%")', probes[i][1], sqlerrm;
        end if;
    end;
  end loop;

  foreach r in array array['anon', 'authenticated'] loop
    execute format('set local role %I', r);
    -- authenticated as profile 1, who played the match the rows are of; anon with no claim.
    perform set_config('request.jwt.claim.sub', case r when 'authenticated' then p1::text else '' end, true);
    if current_user <> r then
      raise exception 'FAIL (R1442): running as %, not % — SET LOCAL ROLE did not take', current_user, r;
    end if;
    if r = 'authenticated' then
      if auth.uid() is distinct from p1 then
        raise exception 'FAIL (R1442): authenticated runs as %, not profile 1', auth.uid();
      end if;
      -- The session still reaches what a client may read, so a refusal below is about these tables.
      select count(*) into readable from public.cards;
      if readable < 1 then
        raise exception 'FAIL (R1442): this session reads no public.cards either; the refusals below would prove nothing';
      end if;
    end if;

    refused := 0;
    for i in 1 .. array_length(probes, 1) loop
      begin
        execute probes[i][2];
        raise exception 'FAIL (R1442): % ran % — the play telemetry is the server''s alone', r, probes[i][1];
      exception
        when insufficient_privilege then
          -- anon has no USAGE on schema app (0001), so its calls are refused at the schema.
          if sqlerrm not like '%' || probes[i][3] || '%'
             and not (r = 'anon' and probes[i][1] like 'app.%' and sqlerrm like '%schema app%') then
            raise exception 'FAIL (R1442): % was refused % by "%", which does not name %',
              r, probes[i][1], sqlerrm, probes[i][3];
          end if;
          refused := refused + 1;
        when others then
          if sqlerrm like 'FAIL%' then raise; end if;
          raise exception 'FAIL (R1442): % % raised "%" (%), not insufficient_privilege',
            r, probes[i][1], sqlerrm, sqlstate;
      end;
    end loop;
    reset role;
    raise notice 'OK (R1442): % was refused all % statements', r, refused;
  end loop;
end $$;

-- And the grants and the policy side: RLS on, no policy, no privilege on the tables and no EXECUTE on
-- the functions for anon, authenticated or PUBLIC; the purge is service_role's.
do $$
declare
  tables constant text[] := array['action_timings', 'emote_events', 'match_signals'];
  fns    constant text[] := array['app.forget_play_telemetry()',
                                  'app.purge_expired_rows(timestamptz, timestamptz, timestamptz)',
                                  'app.purge_expired_rows(timestamptz, timestamptz)'];
  t       text;
  grantee text;
begin
  foreach t in array tables loop
    if not (select relrowsecurity from pg_class where oid = ('public.' || t)::regclass) then
      raise exception 'FAIL (R1442): row-level security is off on public.%', t;
    end if;
    if exists (select 1 from pg_policies where schemaname = 'public' and tablename = t) then
      raise exception 'FAIL (R1442): public.% has a policy; it is meant to have none (server only)', t;
    end if;
    foreach grantee in array array['anon', 'authenticated', 'public'] loop
      if has_table_privilege(grantee, 'public.' || t,
                             'SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER') then
        raise exception 'FAIL (R1442): % holds a privilege on public.%', grantee, t;
      end if;
    end loop;
  end loop;

  foreach t in array fns loop
    foreach grantee in array array['anon', 'authenticated', 'public'] loop
      if has_function_privilege(grantee, t, 'EXECUTE') then
        raise exception 'FAIL (R1442): % may execute %', grantee, t;
      end if;
    end loop;
  end loop;
  if not has_function_privilege('service_role', 'app.purge_expired_rows(timestamptz, timestamptz, timestamptz)', 'EXECUTE') then
    raise exception 'FAIL (R1442): service_role may not execute the three-cutoff purge it calls';
  end if;

  raise notice 'OK (R1442): RLS on with no policy on all three tables; no client role holds a privilege on them or EXECUTE on either function';
end $$;
rollback;

\echo '### R1442: the purge deletes the telemetry of every match that ended before its third cutoff, and nothing newer ###'
begin;
do $$
declare
  p1     constant uuid := '11111111-1111-1111-1111-111111111111';
  p2     constant uuid := '22222222-2222-2222-2222-222222222222';
  old_m  constant uuid := 'f1442000-0000-4000-8000-000000000011';
  new_m  constant uuid := 'f1442000-0000-4000-8000-000000000012';
  live_m constant uuid := 'f1442000-0000-4000-8000-000000000013';
  full_set constant text := 'emotes p1 1, emotes p2 1, signals p1 1, signals p2 1, timings p1 2, timings p2 2';
  cutoff constant timestamptz := now() - interval '365 days';
  v_row  record;
  v_old  record;
  m      uuid;
  seen   text;
begin
  -- Exactness guard: the purge's count is compared with the rows written here, so there may be no
  -- others.
  if exists (select 1 from public.action_timings) or exists (select 1 from public.emote_events)
     or exists (select 1 from public.match_signals) then
    raise exception 'FAIL (R1442): the telemetry tables already hold rows, so the purge''s count is off';
  end if;

  -- A match that ended 366 days ago, one that ended 364 days ago, and a live one, each with both seats'
  -- rows: two timings, one emote and one signal a seat. The live match carries an old ended_at too, so
  -- only its status keeps its rows.
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (old_m, 'over', 's-1442-old', p1, p2, '[]', '[]', 'core-1', now(), now(), now() - interval '366 days'),
         (new_m, 'over', 's-1442-new', p1, p2, '[]', '[]', 'core-1', now(), now(), now() - interval '364 days'),
         (live_m, 'live', 's-1442-live', p1, p2, '[]', '[]', 'core-1', now() + interval '1 hour', now(),
          now() - interval '366 days');
  insert into public.action_timings (match_id, seq, seat, action_kind, legal_count, turn, think_ms,
                                     first_in_turn, pilot)
  select m_id, s.seq, s.seat, 'endTurn', 1, 1, 500, s.seq <= 2, 'human'
    from unnest(array[old_m, new_m, live_m]) m_id,
         (values (1, 'p1'), (2, 'p2'), (3, 'p1'), (4, 'p2')) s(seq, seat);
  insert into public.emote_events (match_id, ordinal, seat, emote_id, turn, pilot)
  select m_id, s.ordinal, s.seat, 'greetings', 1, 'human'
    from unnest(array[old_m, new_m, live_m]) m_id, (values (1, 'p1'), (2, 'p2')) s(ordinal, seat);
  insert into public.match_signals (match_id, seat, pilot)
  select m_id, s.seat, 'human'
    from unnest(array[old_m, new_m, live_m]) m_id, (values ('p1'), ('p2')) s(seat);

  select * into v_row from app.purge_expired_rows(now() - interval '30 days', now() - interval '90 days', cutoff);

  -- The old match's eight rows, the three tables counted together.
  if v_row.play_telemetry <> 8 then
    raise exception 'FAIL (R1442): the purge reports % telemetry rows, expected the old match''s 8', v_row.play_telemetry;
  end if;
  foreach m in array array[old_m, new_m, live_m] loop
    select string_agg(format('%s %s %s', x.t, x.seat, x.c), ', ' order by x.t, x.seat) into seen
      from (select 'timings' as t, seat, count(*) as c from public.action_timings where match_id = m group by seat
            union all
            select 'emotes', seat, count(*) from public.emote_events where match_id = m group by seat
            union all
            select 'signals', seat, count(*) from public.match_signals where match_id = m group by seat) x;
    if m = old_m and seen is not null then
      raise exception 'FAIL (R1442): the match that ended 366 days ago kept telemetry: %', seen;
    end if;
    if m <> old_m and seen is distinct from full_set then
      raise exception 'FAIL (R1442): the recent or live match % holds [%] after the purge, expected [%]',
        m, coalesce(seen, 'nothing'), full_set;
    end if;
  end loop;
  -- The purge deletes telemetry, never the match it is of.
  if (select count(*) from public.matches where id in (old_m, new_m, live_m)) <> 3 then
    raise exception 'FAIL (R1442): the purge deleted a match row';
  end if;

  -- 0013's two-cutoff purge, kept for the server a deploy replaces, still answers with its two
  -- columns and leaves the telemetry alone, even of a match that ended past both of its cutoffs.
  select * into v_old from app.purge_expired_rows(now() - interval '30 days', now() - interval '90 days');
  if v_old::text !~ '^\(\d+,\d+\)$' then
    raise exception 'FAIL (R1442): the two-cutoff purge answers %, not (code_attempts, match_actions)', v_old;
  end if;
  if (select count(*) from public.action_timings) + (select count(*) from public.emote_events)
     + (select count(*) from public.match_signals) <> 16 then
    raise exception 'FAIL (R1442): the two-cutoff purge deleted telemetry';
  end if;

  raise notice 'OK (R1442): the purge deleted the 8 rows of the match past its cutoff, kept a recent and a live match''s 16, and 0013''s purge still runs';
end $$;
rollback;

\echo '### R1442: deleting an account deletes the telemetry of every seat it held; the opponent''s stays ###'
begin;
do $$
declare
  r record;
begin
  -- The trigger that does it: BEFORE DELETE on profiles, each row, so the seats on public.matches
  -- still name the profile (0012's foreign keys empty them after). INVOKER with search_path pinned.
  select t.tgtype, t.tgenabled, p.prosecdef, p.proconfig, pn.nspname as pronsp, p.proname
    into r
    from pg_trigger t
    join pg_proc p on p.oid = t.tgfoid
    join pg_namespace pn on pn.oid = p.pronamespace
   where t.tgrelid = 'public.profiles'::regclass and t.tgname = 'profiles_forget_play_telemetry';
  if not found then
    raise exception 'FAIL (R1442): public.profiles has no profiles_forget_play_telemetry trigger';
  end if;
  if r.pronsp <> 'app' or r.proname <> 'forget_play_telemetry' then
    raise exception 'FAIL (R1442): the trigger fires %.%, not app.forget_play_telemetry', r.pronsp, r.proname;
  end if;
  -- tgtype bits: 1 = FOR EACH ROW, 2 = BEFORE, 4 = INSERT, 8 = DELETE, 16 = UPDATE.
  if r.tgtype & 1 = 0 or r.tgtype & 2 = 0 or r.tgtype & 8 = 0 or r.tgtype & (4 | 16) <> 0 then
    raise exception 'FAIL (R1442): the trigger is not BEFORE DELETE FOR EACH ROW alone (tgtype %)', r.tgtype;
  end if;
  if r.tgenabled not in ('O', 'A') then
    raise exception 'FAIL (R1442): the trigger is disabled (tgenabled = %)', r.tgenabled;
  end if;
  if r.prosecdef or not coalesce(r.proconfig, '{}'::text[]) @> array['search_path=""'] then
    raise exception 'FAIL (R1442): app.forget_play_telemetry is DEFINER % with proconfig %', r.prosecdef, r.proconfig;
  end if;
  raise notice 'OK (R1442): profiles_forget_play_telemetry is a BEFORE DELETE row trigger, INVOKER with search_path pinned';
end $$;

do $$
declare
  gone  constant uuid := 'f1442000-dddd-4ddd-8ddd-dddddddddddd';
  other constant uuid := '22222222-2222-2222-2222-222222222222';
  p1    constant uuid := '11111111-1111-1111-1111-111111111111';
  as_p1 constant uuid := 'f1442000-0000-4000-8000-000000000021';
  as_p2 constant uuid := 'f1442000-0000-4000-8000-000000000022';
  apart constant uuid := 'f1442000-0000-4000-8000-000000000023';
  full_set constant text := 'emotes p1 1, emotes p2 1, signals p1 1, signals p2 1, timings p1 2, timings p2 2';
  -- [match, what must be left of it once the account is gone].
  expected constant text[][] := array[
    [as_p1::text, 'emotes p2 1, signals p2 1, timings p2 2'],
    [as_p2::text, 'emotes p1 1, signals p1 1, timings p1 2'],
    [apart::text, full_set]];
  m     uuid;
  seen  text;
  v_row record;
  i     int;
begin
  -- A player who sat in p1 of one finished match and in p2 of another, both against profile 2, and a
  -- match between profiles 1 and 2 it never played; every seat of each with telemetry.
  insert into auth.users (id, email, email_confirmed_at) values (gone, 'gone-1442@example.test', now());
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (as_p1, 'over', 's-1442-p1', gone, other, '[]', '[]', 'core-1', now(), now(), now()),
         (as_p2, 'over', 's-1442-p2', other, gone, '[]', '[]', 'core-1', now(), now(), now()),
         (apart, 'over', 's-1442-apart', other, p1, '[]', '[]', 'core-1', now(), now(), now());
  insert into public.action_timings (match_id, seq, seat, action_kind, legal_count, turn, think_ms,
                                     first_in_turn, pilot)
  select m_id, s.seq, s.seat, 'endTurn', 1, 1, 500, s.seq <= 2, 'human'
    from unnest(array[as_p1, as_p2, apart]) m_id,
         (values (1, 'p1'), (2, 'p2'), (3, 'p1'), (4, 'p2')) s(seq, seat);
  insert into public.emote_events (match_id, ordinal, seat, emote_id, turn, pilot)
  select m_id, s.ordinal, s.seat, 'greetings', 1, 'human'
    from unnest(array[as_p1, as_p2, apart]) m_id, (values (1, 'p1'), (2, 'p2')) s(ordinal, seat);
  insert into public.match_signals (match_id, seat, pilot)
  select m_id, s.seat, 'human'
    from unnest(array[as_p1, as_p2, apart]) m_id, (values ('p1'), ('p2')) s(seat);

  -- Vacuity guard: every seat of every match holds its rows before the delete.
  foreach m in array array[as_p1, as_p2, apart] loop
    select string_agg(format('%s %s %s', x.t, x.seat, x.c), ', ' order by x.t, x.seat) into seen
      from (select 'timings' as t, seat, count(*) as c from public.action_timings where match_id = m group by seat
            union all
            select 'emotes', seat, count(*) from public.emote_events where match_id = m group by seat
            union all
            select 'signals', seat, count(*) from public.match_signals where match_id = m group by seat) x;
    if seen is distinct from full_set then
      raise exception 'FAIL (R1442): match % holds [%] before the delete, expected [%]', m, seen, full_set;
    end if;
  end loop;

  -- The delete the Supabase dashboard and the admin API make; it cascades to public.profiles.
  delete from auth.users where id = gone;
  if exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R1442): the profile outlived its auth.users row';
  end if;

  for i in 1 .. array_length(expected, 1) loop
    m := expected[i][1]::uuid;
    select string_agg(format('%s %s %s', x.t, x.seat, x.c), ', ' order by x.t, x.seat) into seen
      from (select 'timings' as t, seat, count(*) as c from public.action_timings where match_id = m group by seat
            union all
            select 'emotes', seat, count(*) from public.emote_events where match_id = m group by seat
            union all
            select 'signals', seat, count(*) from public.match_signals where match_id = m group by seat) x;
    if seen is distinct from expected[i][2] then
      raise exception 'FAIL (R1442): match % holds [%] after the delete, expected [%]',
        m, coalesce(seen, 'nothing'), expected[i][2];
    end if;
  end loop;

  -- The opponent keeps both matches, the deleted player's seat emptied (0012), so its rows still
  -- have the match they are of.
  select p1_profile_id, p2_profile_id into v_row from public.matches where id = as_p1;
  if not found or v_row.p1_profile_id is not null or v_row.p2_profile_id <> other then
    raise exception 'FAIL (R1442): the match the deleted player sat in p1 of reads %', v_row;
  end if;
  select p1_profile_id, p2_profile_id into v_row from public.matches where id = as_p2;
  if not found or v_row.p1_profile_id <> other or v_row.p2_profile_id is not null then
    raise exception 'FAIL (R1442): the match the deleted player sat in p2 of reads %', v_row;
  end if;

  -- And a match deleted outright (R679's void) takes all of its telemetry with it.
  delete from public.matches where id = apart;
  if exists (select 1 from public.action_timings where match_id = apart)
     or exists (select 1 from public.emote_events where match_id = apart)
     or exists (select 1 from public.match_signals where match_id = apart) then
    raise exception 'FAIL (R1442): a deleted match left telemetry behind';
  end if;

  raise notice 'OK (R1442): the deleted player''s seats lost their rows in p1 and in p2; the opponent''s and a bystander match''s stayed; a deleted match took its own';
end $$;
rollback;

\echo '### ALL PLAY TELEMETRY CHECKS RAN ###'
