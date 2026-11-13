-- Daily games: a 1v1 match played at days per move, its invites, its notice settings and the
-- browsers' push subscriptions (migration 0089, SPEC §9.13, §9.1; issue #3727). Runs after
-- 52_v0_5_13_retcon.sql; profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03 activated
-- them).
\set ON_ERROR_STOP on

-- Same rules as 01-52: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate (and on the constraint, the table or the message that
-- names it), each check that could pass vacuously carries a guard, and every block that writes runs
-- inside a transaction that is rolled back. Nothing here selects into the log: the do blocks raise
-- or say OK.

\echo '### §9.13: 0089''s four tables have RLS on, their columns, and foreign keys that cascade ###'
do $$
declare
  t      text;
  v_text text;
  r      record;
begin
  foreach t in array array['daily_games', 'daily_invites', 'daily_notify', 'push_subscriptions'] loop
    if not exists (select 1 from pg_class where oid = ('public.' || t)::regclass and relrowsecurity) then
      raise exception 'FAIL (§9.13): public.% has no row level security', t;
    end if;
  end loop;

  -- The columns, as the store reads and writes them: every time a bigint of epoch ms.
  for r in select * from (values
      ('daily_games', 'match_id uuid NO, p1_profile_id uuid NO, p2_profile_id uuid NO, mode text NO, '
                      'per_move_days integer NO, clock jsonb NO, board jsonb NO, next_deadline_at bigint YES, '
                      'seq bigint NO, created_at bigint NO, ended_at bigint YES'),
      ('daily_invites', 'id uuid NO, kind text NO, from_profile uuid NO, to_profile uuid YES, mode text NO, '
                        'per_move_days integer NO, choice jsonb NO, state text NO, match_id uuid YES, '
                        'created_at bigint NO, expires_at bigint NO'),
      ('daily_notify', 'profile_id uuid NO, email_on boolean NO, unsubscribe_token uuid NO, '
                       'last_email_at bigint YES, last_push_at bigint YES'),
      ('push_subscriptions', 'id uuid NO, profile_id uuid NO, endpoint text NO, p256dh text NO, auth text NO, '
                             'created_at bigint NO')) as x(tbl, cols) loop
    select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
      into v_text
      from information_schema.columns
     where table_schema = 'public' and table_name = r.tbl;
    if v_text is distinct from r.cols then
      raise exception 'FAIL (§9.13): public.% has the columns [%], expected [%]', r.tbl, coalesce(v_text, '(none)'), r.cols;
    end if;
  end loop;

  -- Every row goes with its account (0012's rule), a daily game with its match too; an invite's
  -- match is no foreign key (the claim names the match its transaction creates next).
  for r in select * from (values
      ('daily_games', 'matches(match_id) c, profiles(p1_profile_id) c, profiles(p2_profile_id) c'),
      ('daily_invites', 'profiles(from_profile) c, profiles(to_profile) c'),
      ('daily_notify', 'profiles(profile_id) c'),
      ('push_subscriptions', 'profiles(profile_id) c')) as x(tbl, fks) loop
    select string_agg(format('%s(%s) %s', c.confrelid::regclass, a.attname, c.confdeltype), ', '
                      order by c.confrelid::regclass::text, a.attname)
      into v_text
      from pg_constraint c
      join pg_attribute a on a.attrelid = c.conrelid and a.attnum = c.conkey[1]
     where c.conrelid = ('public.' || r.tbl)::regclass and c.contype = 'f';
    if v_text is distinct from r.fks then
      raise exception 'FAIL (§9.13): public.%''s foreign keys are [%], expected [%]', r.tbl, coalesce(v_text, '(none)'), r.fks;
    end if;
  end loop;

  -- Current state, no ledger: no trigger keeps a row from changing.
  if exists (select 1 from pg_trigger
              where tgrelid = any (array['public.daily_games'::regclass, 'public.daily_invites'::regclass,
                                         'public.daily_notify'::regclass, 'public.push_subscriptions'::regclass])
                and not tgisinternal) then
    raise exception 'FAIL (§9.13): a daily table has a trigger, but the server updates every row';
  end if;

  raise notice 'OK (§9.13): the four tables have RLS on, their columns, cascading foreign keys and no trigger';
end $$;

\echo '### §9.13, §9.1: a player reads their own games and invites (never an invite''s choice), nothing of the notices or the push keys, and writes none ###'
do $$
declare
  v_text text;
  t      text;
  op     text;
  who    text;
  col    text;
  wrong  text[] := array[]::text[];
begin
  select string_agg(format('%s %s %s %s', policyname, cmd, array_to_string(roles, ','), qual), '; ' order by policyname)
    into v_text
    from pg_policies
   where schemaname = 'public' and tablename in ('daily_games', 'daily_invites', 'daily_notify', 'push_subscriptions');
  if v_text is distinct from
       'daily_games_select_own SELECT authenticated ((p1_profile_id = app.current_profile_id()) OR (p2_profile_id = app.current_profile_id())); '
       'daily_invites_select_own SELECT authenticated ((from_profile = app.current_profile_id()) OR (to_profile = app.current_profile_id()))' then
    raise exception 'FAIL (§9.13): the policies are [%], expected one select-own policy on each of daily_games and daily_invites',
      coalesce(v_text, '(none)');
  end if;

  foreach t in array array['daily_games', 'daily_invites', 'daily_notify', 'push_subscriptions'] loop
    foreach who in array array['authenticated', 'anon', 'public'] loop
      foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
        if has_table_privilege(who, 'public.' || t, op) <> (t = 'daily_games' and who = 'authenticated' and op = 'SELECT') then
          wrong := wrong || format('%s %s %s', who, op, t);
        end if;
      end loop;
    end loop;
  end loop;
  -- daily_invites: a column grant, every column but the sender's frozen choice.
  for col in select column_name from information_schema.columns
              where table_schema = 'public' and table_name = 'daily_invites' loop
    if has_column_privilege('authenticated', 'public.daily_invites', col, 'SELECT') <> (col <> 'choice') then
      wrong := wrong || format('authenticated SELECT daily_invites.%s', col);
    end if;
    foreach op in array array['INSERT', 'UPDATE', 'REFERENCES'] loop
      if has_column_privilege('authenticated', 'public.daily_invites', col, op) then
        wrong := wrong || format('authenticated %s daily_invites.%s', op, col);
      end if;
    end loop;
    if has_column_privilege('anon', 'public.daily_invites', col, 'SELECT') then
      wrong := wrong || format('anon SELECT daily_invites.%s', col);
    end if;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (§9.13): the client privileges are wrong for [%]', array_to_string(wrong, ', ');
  end if;
  raise notice 'OK (§9.13): one select-own policy each on daily_games and daily_invites, SELECT the only client privilege, daily_invites'' choice and the other two tables out of reach';
end $$;

\echo '### §9.13: the checks refuse what no daily game or invite may be, and one open invite per key ###'
begin;
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, started_at, ceiling_at)
values ('e3727000-0000-4000-8000-0000000000a1', 'live', 'seed-3727',
        '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour');
do $$
declare
  m  constant uuid := 'e3727000-0000-4000-8000-0000000000a1';
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  -- [what, mode, per_move_days, p2, clock, board, the check it breaks]. Each breaks that check alone.
  games constant text[][] := array[
    ['a Best-of-3 game', 'bo3', '3', p2::text, '{}', '{}', 'daily_games_mode_check'],
    ['two days per move', 'bo1', '2', p2::text, '{}', '{}', 'daily_games_per_move_days_check'],
    ['no time per move', 'bo1', '0', p2::text, '{}', '{}', 'daily_games_per_move_days_check'],
    ['one account in both seats', 'bo1', '3', p1::text, '{}', '{}', 'daily_games_seats_check'],
    ['a clock that is no object', 'bo1', '3', p2::text, '[]', '{}', 'daily_games_clock_check'],
    ['a board that is no object', 'random', '14', p2::text, '{}', '1', 'daily_games_board_check']];
  -- [what, kind, to_profile, mode, per_move_days, choice, state, the check it breaks].
  invites constant text[][] := array[
    ['a challenge to nobody', 'challenge', null, 'bo1', '3', '{}', 'open', 'daily_invites_to_check'],
    ['a seek to somebody', 'seek', p2::text, 'bo1', '3', '{}', 'open', 'daily_invites_to_check'],
    ['a challenge to oneself', 'challenge', p1::text, 'bo1', '3', '{}', 'open', 'daily_invites_self_check'],
    ['an invite of another kind', 'rematch', null, 'bo1', '3', '{}', 'open', 'daily_invites_kind_check'],
    ['a Best-of-3 seek', 'seek', null, 'bo3', '3', '{}', 'open', 'daily_invites_mode_check'],
    ['a seek of five days', 'seek', null, 'bo1', '5', '{}', 'open', 'daily_invites_per_move_days_check'],
    ['a choice that is no object', 'seek', null, 'bo1', '3', '"deck"', 'open', 'daily_invites_choice_check'],
    ['a state of its own', 'seek', null, 'bo1', '3', '{}', 'won', 'daily_invites_state_check']];
  i     int;
  v_con text;
begin
  for i in 1 .. array_length(games, 1) loop
    begin
      insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
      values (m, p1, games[i][4]::uuid, games[i][2], games[i][3]::int, games[i][5]::jsonb, games[i][6]::jsonb, 1);
      raise exception 'FAIL (§9.13): the schema took %', games[i][1];
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from games[i][7] then
          raise exception 'FAIL (§9.13): % was refused by %, not %', games[i][1], v_con, games[i][7];
        end if;
    end;
  end loop;
  -- The seq a move's write is conditional on is a log seq, never below the 0 before the first action.
  begin
    insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, seq, created_at)
    values (m, p1, p2, 'bo1', 3, '{}', '{}', -1, 1);
    raise exception 'FAIL (§9.13): the schema took a seq below 0';
  exception
    when check_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'daily_games_seq_check' then
        raise exception 'FAIL (§9.13): a seq below 0 was refused by %, not daily_games_seq_check', v_con;
      end if;
  end;
  for i in 1 .. array_length(invites, 1) loop
    begin
      insert into public.daily_invites (kind, from_profile, to_profile, mode, per_move_days, choice, state, created_at, expires_at)
      values (invites[i][2], p1, invites[i][3]::uuid, invites[i][4], invites[i][5]::int, invites[i][6]::jsonb,
              invites[i][7], 1, 2);
      raise exception 'FAIL (§9.13): the schema took %', invites[i][1];
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from invites[i][8] then
          raise exception 'FAIL (§9.13): % was refused by %, not %', invites[i][1], v_con, invites[i][8];
        end if;
    end;
  end loop;

  -- The vacuity guard: what the store writes is taken, at each time per move and both modes.
  insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
  values (m, p1, p2, 'random', 14, '{"perMoveDays": 14, "holds": []}', '{"turn": 1}', 1);
  if (select seq from public.daily_games where match_id = m) is distinct from 0 then
    raise exception 'FAIL (§9.13): a new daily game''s seq is not 0';
  end if;
  begin
    insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
    values (m, p1, p2, 'bo1', 1, '{}', '{}', 2);
    raise exception 'FAIL (§9.13): a second daily game for one match was taken';
  exception
    when unique_violation then
      get stacked diagnostics v_con = constraint_name;
      if v_con is distinct from 'daily_games_pkey' then
        raise exception 'FAIL (§9.13): the second game was refused by %, not daily_games_pkey', v_con;
      end if;
  end;
  begin
    insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
    values (gen_random_uuid(), p1, p2, 'bo1', 1, '{}', '{}', 2);
    raise exception 'FAIL (§9.13): a daily game with no match was taken';
  exception
    when foreign_key_violation then null;
  end;

  -- One open challenge per sender and friend, one open seek per sender, mode and time; a closed
  -- one frees its key, and another kind, mode or time is another key.
  insert into public.daily_invites (kind, from_profile, to_profile, mode, per_move_days, choice, created_at, expires_at)
  values ('challenge', p1, p2, 'bo1', 3, '{}', 1, 2), ('challenge', p2, p1, 'bo1', 3, '{}', 1, 2),
         ('seek', p1, null, 'bo1', 3, '{}', 1, 2), ('seek', p1, null, 'bo1', 7, '{}', 1, 2),
         ('seek', p1, null, 'random', 3, '{}', 1, 2), ('seek', p2, null, 'bo1', 3, '{}', 1, 2);
  for i in 1 .. 2 loop
    begin
      if i = 1 then
        insert into public.daily_invites (kind, from_profile, to_profile, mode, per_move_days, choice, created_at, expires_at)
        values ('challenge', p1, p2, 'random', 14, '{}', 3, 4);
      else
        insert into public.daily_invites (kind, from_profile, to_profile, mode, per_move_days, choice, created_at, expires_at)
        values ('seek', p1, null, 'bo1', 3, '{}', 3, 4);
      end if;
      raise exception 'FAIL (§9.13): a second open invite of one key was taken (case %)', i;
    exception
      when unique_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from (array['daily_invites_open_challenge_key', 'daily_invites_open_seek_key'])[i] then
          raise exception 'FAIL (§9.13): the second open invite (case %) was refused by %', i, v_con;
        end if;
    end;
  end loop;
  update public.daily_invites set state = 'declined' where kind = 'challenge' and from_profile = p1;
  insert into public.daily_invites (kind, from_profile, to_profile, mode, per_move_days, choice, created_at, expires_at)
  values ('challenge', p1, p2, 'bo1', 3, '{}', 3, 4);
  if (select count(*) from public.daily_invites where from_profile = p1 and kind = 'challenge') <> 2 then
    raise exception 'FAIL (§9.13): a closed challenge did not free its key';
  end if;
  raise notice 'OK (§9.13): % games and % invites the checks refuse are refused, each by its own check; one game per match, beside its match; one open invite per key',
    array_length(games, 1), array_length(invites, 1);
end $$;
rollback;

\echo '### §9.13: a player reads the daily games they are seated in and the invites they sent or received, and nobody else''s ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3727000-0000-4000-8000-000000000003', 'r3727-three@example.test', now());
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, started_at, ceiling_at)
values ('e3727000-0000-4000-8000-0000000000b1', 'live', 'seed-3727-b1',
        '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour'),
       ('e3727000-0000-4000-8000-0000000000b2', 'live', 'seed-3727-b2',
        '22222222-2222-2222-2222-222222222222', 'e3727000-0000-4000-8000-000000000003',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour');
insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
values ('e3727000-0000-4000-8000-0000000000b1', '11111111-1111-1111-1111-111111111111',
        '22222222-2222-2222-2222-222222222222', 'bo1', 3, '{}', '{}', 1),
       ('e3727000-0000-4000-8000-0000000000b2', '22222222-2222-2222-2222-222222222222',
        'e3727000-0000-4000-8000-000000000003', 'random', 7, '{}', '{}', 2);
insert into public.daily_invites (id, kind, from_profile, to_profile, mode, per_move_days, choice, created_at, expires_at)
values ('e3727000-0000-4000-8000-0000000000c1', 'challenge', '11111111-1111-1111-1111-111111111111',
        '22222222-2222-2222-2222-222222222222', 'bo1', 3, '{"deck": "one"}', 1, 2),
       ('e3727000-0000-4000-8000-0000000000c2', 'challenge', 'e3727000-0000-4000-8000-000000000003',
        '22222222-2222-2222-2222-222222222222', 'bo1', 3, '{"deck": "three"}', 1, 2),
       ('e3727000-0000-4000-8000-0000000000c3', 'seek', '11111111-1111-1111-1111-111111111111',
        null, 'bo1', 3, '{"deck": "one"}', 1, 2),
       ('e3727000-0000-4000-8000-0000000000c4', 'seek', 'e3727000-0000-4000-8000-000000000003',
        null, 'bo1', 3, '{"deck": "three"}', 1, 2);
insert into public.daily_notify (profile_id)
values ('11111111-1111-1111-1111-111111111111'), ('22222222-2222-2222-2222-222222222222');
insert into public.push_subscriptions (profile_id, endpoint, p256dh, auth, created_at)
values ('11111111-1111-1111-1111-111111111111', 'https://push.example.test/1', 'k1', 'a1', 1),
       ('22222222-2222-2222-2222-222222222222', 'https://push.example.test/2', 'k2', 'a2', 1);

-- Preflight: the writes the client is about to be refused are ones the owner can make, inside a
-- subtransaction rolled back at once.
do $$
begin
  begin
    update public.daily_games set ended_at = 3;
    update public.daily_invites set state = 'cancelled';
    update public.daily_notify set email_on = false;
    delete from public.push_subscriptions;
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception
        'FAIL (§9.13): the owner could not make the writes the client is about to be refused ("%", %) — the refusals below would prove nothing about privileges',
        sqlerrm, sqlstate;
    end if;
  end;
end $$;

set local role authenticated;
do $$ begin perform set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true); end $$;

do $$
declare
  seen   text;
  probe  text;
  probes constant text[][] := array[
    ['daily_games INSERT',
     $q$insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
        values ('e3727000-0000-4000-8000-0000000000b1', '11111111-1111-1111-1111-111111111111',
                '22222222-2222-2222-2222-222222222222', 'bo1', 3, '{}', '{}', 1)$q$],
    ['daily_games UPDATE', $q$update public.daily_games set ended_at = 3$q$],
    ['daily_games DELETE', $q$delete from public.daily_games$q$],
    ['daily_invites INSERT',
     $q$insert into public.daily_invites (kind, from_profile, mode, per_move_days, choice, created_at, expires_at)
        values ('seek', '11111111-1111-1111-1111-111111111111', 'bo1', 7, '{}', 1, 2)$q$],
    ['daily_invites UPDATE', $q$update public.daily_invites set state = 'cancelled'$q$],
    ['daily_invites DELETE', $q$delete from public.daily_invites$q$],
    ['daily_invites SELECT choice', $q$select choice from public.daily_invites$q$],
    ['daily_notify SELECT', $q$select unsubscribe_token from public.daily_notify$q$],
    ['daily_notify UPDATE', $q$update public.daily_notify set email_on = false$q$],
    ['push_subscriptions SELECT', $q$select endpoint, p256dh, auth from public.push_subscriptions$q$],
    ['push_subscriptions DELETE', $q$delete from public.push_subscriptions$q$]];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (§9.13): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;

  select string_agg(match_id::text, ', ' order by match_id) into seen from public.daily_games;
  if seen is distinct from 'e3727000-0000-4000-8000-0000000000b1' then
    raise exception 'FAIL (§9.13): profile 1 read the daily games [%], expected its own one', coalesce(seen, '(none)');
  end if;
  select string_agg(format('%s %s', id, kind), ', ' order by id) into seen from public.daily_invites;
  if seen is distinct from 'e3727000-0000-4000-8000-0000000000c1 challenge, e3727000-0000-4000-8000-0000000000c3 seek' then
    raise exception 'FAIL (§9.13): profile 1 read the invites [%], expected the challenge and the seek it sent', coalesce(seen, '(none)');
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (§9.13): % succeeded as a client — the server is the only writer and the only reader of it', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%daily_%' and sqlerrm not like '%push_subscriptions%' then
          raise exception 'FAIL (§9.13): % was refused by "%" (%), which names no daily table', probe, sqlerrm, sqlstate;
        end if;
        raise notice 'OK (§9.13): % refused — % (%)', probe, sqlerrm, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (§9.13): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;
  raise notice 'OK (§9.13): profile 1 reads its own game and the two invites it sent, and % reads and writes were refused',
    array_length(probes, 1);
end $$;

do $$ begin perform set_config('request.jwt.claim.sub', '22222222-2222-2222-2222-222222222222', true); end $$;
do $$
declare
  seen text;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (§9.13): running as %, not authenticated', current_user;
  end if;
  select string_agg(match_id::text, ', ' order by match_id) into seen from public.daily_games;
  if seen is distinct from 'e3727000-0000-4000-8000-0000000000b1, e3727000-0000-4000-8000-0000000000b2' then
    raise exception 'FAIL (§9.13): profile 2 read the daily games [%], expected both it is seated in', coalesce(seen, '(none)');
  end if;
  select string_agg(id::text, ', ' order by id) into seen from public.daily_invites;
  if seen is distinct from 'e3727000-0000-4000-8000-0000000000c1, e3727000-0000-4000-8000-0000000000c2' then
    raise exception 'FAIL (§9.13): profile 2 read the invites [%], expected the two challenges it received', coalesce(seen, '(none)');
  end if;
  raise notice 'OK (§9.13): profile 2 reads both games it is seated in and the two challenges sent to it, and no seek of another';
end $$;

-- Signed in as nobody: the policies' app.current_profile_id() is null, so no row is anybody's.
do $$ begin perform set_config('request.jwt.claim.sub', '', true); end $$;
do $$
begin
  if exists (select 1 from public.daily_games) or exists (select 1 from public.daily_invites) then
    raise exception 'FAIL (§9.13): a client with no profile read a daily game or invite';
  end if;
  raise notice 'OK (§9.13): a client with no profile reads no daily game or invite';
end $$;

reset role;
set local role anon;
do $$
declare
  t text;
begin
  if current_user <> 'anon' then
    raise exception 'FAIL (§9.13): running as %, not anon', current_user;
  end if;
  foreach t in array array['daily_games', 'daily_invites', 'daily_notify', 'push_subscriptions'] loop
    begin
      execute format('select count(*) from public.%I', t);
      raise exception 'FAIL (§9.13): anon read public.%', t;
    exception
      when insufficient_privilege then null;
    end;
  end loop;
  raise notice 'OK (§9.13): anon reads none of the four tables';
end $$;

reset role;
rollback;

\echo '### §9.13, R112, §9.5: app.live_matches() and app.reap_stale_matches() leave a daily match out, and only a daily one ###'
begin;
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, started_at, ceiling_at)
values ('e3727000-0000-4000-8000-0000000000d1', 'live', 'seed-3727-d1',
        '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour'),
       ('e3727000-0000-4000-8000-0000000000d2', 'live', 'seed-3727-d2',
        '22222222-2222-2222-2222-222222222222', '11111111-1111-1111-1111-111111111111',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour');
insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
values ('e3727000-0000-4000-8000-0000000000d2', '22222222-2222-2222-2222-222222222222',
        '11111111-1111-1111-1111-111111111111', 'bo1', 1, '{}', '{}', 1);
do $$
declare
  live  constant uuid := 'e3727000-0000-4000-8000-0000000000d1';
  daily constant uuid := 'e3727000-0000-4000-8000-0000000000d2';
  total bigint;
begin
  if not exists (select 1 from app.live_matches() m where m.id = live) then
    raise exception 'FAIL (§9.13): app.live_matches() left out a live match that is no daily game';
  end if;
  if exists (select 1 from app.live_matches() m where m.id = daily) then
    raise exception 'FAIL (§9.13): app.live_matches() lists a daily match, which the reaper would draw at its ceiling';
  end if;
  select count(*) into total from app.live_matches();
  if total <> (select count(*) from public.matches m
                where m.status = 'live' and not exists (select 1 from public.daily_games d where d.match_id = m.id)) then
    raise exception 'FAIL (§9.13): app.live_matches() returned % rows, not every live match but the daily ones', total;
  end if;
  -- The reaper's SQL half leaves a daily match past its row's ceiling alone, and still reaps a live one.
  update public.matches set ceiling_at = now() - interval '1 minute' where id in (live, daily);
  perform app.reap_stale_matches();
  if (select status from public.matches where id = daily) is distinct from 'live' then
    raise exception 'FAIL (§9.13): app.reap_stale_matches() drew a daily match at its row''s ceiling';
  end if;
  if (select status from public.matches where id = live) is distinct from 'over' then
    raise exception 'FAIL (§9.13): app.reap_stale_matches() left a live match past its ceiling, so the check above measured nothing';
  end if;
  -- Without its daily row the same match is a live one again: the filter is the daily row, not the match.
  delete from public.daily_games where match_id = daily;
  if not exists (select 1 from app.live_matches() m where m.id = daily) then
    raise exception 'FAIL (§9.13): app.live_matches() still leaves out a live match whose daily row is gone';
  end if;
  if not (select p.prosecdef from pg_proc p join pg_namespace n on n.oid = p.pronamespace
           where n.nspname = 'app' and p.proname = 'live_matches') then
    raise exception 'FAIL (§9.13): app.live_matches() lost SECURITY DEFINER';
  end if;
  if has_function_privilege('authenticated', 'app.live_matches()', 'EXECUTE')
     or has_function_privilege('anon', 'app.live_matches()', 'EXECUTE')
     or not has_function_privilege('service_role', 'app.live_matches()', 'EXECUTE') then
    raise exception 'FAIL (§9.13): app.live_matches() is not service_role''s alone';
  end if;
  raise notice 'OK (§9.13): app.live_matches() lists the live match and leaves out the daily one, and is service_role''s alone; app.reap_stale_matches() reaps the live one and leaves the daily one';
end $$;
rollback;

\echo '### §9.13, 0012: a deleted account takes its daily rows with it, and nobody else''s ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3727000-0000-4000-8000-000000000004', 'r3727-leaver@example.test', now());
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, ceiling_at, started_at, ended_at)
values ('e3727000-0000-4000-8000-0000000000e1', 'over', 'seed-3727-e1',
        'e3727000-0000-4000-8000-000000000004', '11111111-1111-1111-1111-111111111111',
        '[]', '[]', 'core-1', now(), now(), now()),
       ('e3727000-0000-4000-8000-0000000000e2', 'over', 'seed-3727-e2',
        '22222222-2222-2222-2222-222222222222', '11111111-1111-1111-1111-111111111111',
        '[]', '[]', 'core-1', now(), now(), now());
do $$
declare
  gone constant uuid := 'e3727000-0000-4000-8000-000000000004';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
  p2   constant uuid := '22222222-2222-2222-2222-222222222222';
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (§9.13): the leaver has no profile; the cascade below would prove nothing';
  end if;
  insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at, ended_at)
  values ('e3727000-0000-4000-8000-0000000000e1', gone, p1, 'bo1', 3, '{}', '{}', 1, 2),
         ('e3727000-0000-4000-8000-0000000000e2', p2, p1, 'bo1', 3, '{}', '{}', 1, 2);
  insert into public.daily_invites (kind, from_profile, to_profile, mode, per_move_days, choice, created_at, expires_at)
  values ('challenge', gone, p1, 'bo1', 3, '{}', 1, 2), ('challenge', p2, gone, 'bo1', 3, '{}', 1, 2),
         ('seek', gone, null, 'bo1', 3, '{}', 1, 2), ('challenge', p2, p1, 'bo1', 3, '{}', 1, 2);
  insert into public.daily_notify (profile_id) values (gone), (p1);
  insert into public.push_subscriptions (profile_id, endpoint, p256dh, auth, created_at)
  values (gone, 'https://push.example.test/gone', 'k', 'a', 1), (p1, 'https://push.example.test/p1', 'k', 'a', 1);

  delete from auth.users where id = gone;

  if exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (§9.13): the leaver''s profile outlived its auth.users row';
  end if;
  if exists (select 1 from public.daily_games where match_id = 'e3727000-0000-4000-8000-0000000000e1')
     or exists (select 1 from public.daily_invites where from_profile = gone or to_profile = gone)
     or exists (select 1 from public.daily_notify where profile_id = gone)
     or exists (select 1 from public.push_subscriptions where profile_id = gone) then
    raise exception 'FAIL (§9.13): a deleted account''s daily rows outlived it';
  end if;
  if not exists (select 1 from public.daily_games where match_id = 'e3727000-0000-4000-8000-0000000000e2')
     or (select count(*) from public.daily_invites where from_profile = p2 and to_profile = p1) <> 1
     or not exists (select 1 from public.daily_notify where profile_id = p1)
     or not exists (select 1 from public.push_subscriptions where profile_id = p1) then
    raise exception 'FAIL (§9.13): deleting one account took another''s daily rows';
  end if;
  -- A daily game goes with its match too.
  delete from public.matches where id = 'e3727000-0000-4000-8000-0000000000e2';
  if exists (select 1 from public.daily_games where match_id = 'e3727000-0000-4000-8000-0000000000e2') then
    raise exception 'FAIL (§9.13): a daily game outlived its match';
  end if;
  raise notice 'OK (§9.13): deleting an account deletes its daily games, invites, notices and subscriptions and no other account''s, and a game goes with its match';
end $$;
rollback;

\echo '### R3727008, 0012: a profile seated in a running daily game is not deleted, and is once its match is over ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e3727000-0000-4000-8000-000000000005', 'r3727-player@example.test', now());
do $$
declare
  gone constant uuid := 'e3727000-0000-4000-8000-000000000005';
  mate constant uuid := '11111111-1111-1111-1111-111111111111';
  mid  constant uuid := 'e3727000-0000-4000-8000-0000000000f1';
  seat text;
  refused_by text;
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R3727008): the player has no profile; the refusal below would prove nothing';
  end if;
  foreach seat in array array['p1', 'p2'] loop
    delete from public.matches where id = mid;
    insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                                catalog_version, started_at, ceiling_at)
    values (mid, 'live', 'seed-3727-f1', case when seat = 'p1' then gone else mate end,
            case when seat = 'p1' then mate else gone end, '[]', '[]', 'core-1', now(), now() + interval '1 hour');
    insert into public.daily_games (match_id, p1_profile_id, p2_profile_id, mode, per_move_days, clock, board, created_at)
    values (mid, case when seat = 'p1' then gone else mate end, case when seat = 'p1' then mate else gone end,
            'bo1', 3, '{}', '{}', 1);
    begin
      -- What the store's `profiles_remove` runs.
      delete from public.profiles where id = gone;
      raise exception 'FAIL (R3727008): a player in seat % of a running daily game was deleted', seat;
    exception
      when check_violation then
        get stacked diagnostics refused_by = constraint_name;
        if refused_by is distinct from 'matches_seats_filled_unless_over_check' then
          raise exception 'FAIL (R3727008): the running daily game (seat %) was refused by "%"', seat, refused_by;
        end if;
    end;
    if not exists (select 1 from public.profiles where id = gone)
       or not exists (select 1 from public.matches
                       where id = mid and status = 'live' and p1_profile_id is not null and p2_profile_id is not null)
       or not exists (select 1 from public.daily_games where match_id = mid and ended_at is null) then
      raise exception 'FAIL (R3727008): the refused delete (seat %) changed the profile, the match or its daily game', seat;
    end if;
  end loop;
  -- Once the match is over, the same delete goes through: the daily game goes, the match stays.
  update public.daily_games set ended_at = 2 where match_id = mid;
  update public.matches set status = 'over', ended_at = now() where id = mid;
  delete from public.profiles where id = gone;
  if exists (select 1 from public.profiles where id = gone)
     or exists (select 1 from public.daily_games where match_id = mid)
     or not exists (select 1 from public.matches where id = mid and p2_profile_id is null and p1_profile_id = mate) then
    raise exception 'FAIL (R3727008): a profile on a finished daily game was not deleted as a finished match''s is';
  end if;
  raise notice 'OK (R3727008): a running daily game''s seat refuses its profile''s delete (matches_seats_filled_unless_over_check), from either seat, and a finished one''s does not';
end $$;
rollback;

\echo '### §9.13: 0089 re-applies over rows of its own and changes none ###'
begin;
insert into public.daily_notify (profile_id, email_on) values ('11111111-1111-1111-1111-111111111111', false);
create temp table r3727_rows (token uuid not null) on commit drop;
insert into r3727_rows select unsubscribe_token from public.daily_notify;
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0089_daily_games.sql
set local client_min_messages = notice;
do $$
begin
  if (select count(*) from r3727_rows) <> 1 then
    raise exception 'FAIL (§9.13): % notice rows before the re-apply, expected the one written', (select count(*) from r3727_rows);
  end if;
  if (select count(*) from public.daily_notify) <> 1
     or (select email_on from public.daily_notify) is distinct from false
     or (select unsubscribe_token from public.daily_notify) is distinct from (select token from r3727_rows) then
    raise exception 'FAIL (§9.13): re-applying 0089 changed public.daily_notify';
  end if;
  if (select count(*) from pg_policies where schemaname = 'public'
       and tablename in ('daily_games', 'daily_invites', 'daily_notify', 'push_subscriptions')) <> 2 then
    raise exception 'FAIL (§9.13): re-applying 0089 left other than two policies on the daily tables';
  end if;
  if has_column_privilege('authenticated', 'public.daily_invites', 'choice', 'SELECT') then
    raise exception 'FAIL (§9.13): re-applying 0089 granted a client an invite''s choice';
  end if;
  raise notice 'OK (§9.13): 0089 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
