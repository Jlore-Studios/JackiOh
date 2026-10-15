-- Play together's social tables (migration 0038, SPEC S7, B43-B53; issue #936). Runs after
-- 21_custom_cards.sql; profiles 1 and 2 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-19: every check raises on failure (`raise exception 'FAIL (S7 …): …'`), every
-- expected refusal is matched on its sqlstate and on the object it names, each check that could
-- pass vacuously carries a guard, and every block runs inside a transaction that is rolled back.

\echo '### S7: the eight tables have RLS, a select policy each but presence, and no client write ###'
begin;
do $$
declare
  readable constant text[] := array['friend_requests', 'friendships', 'blocks', 'challenges',
                                    'chat_messages', 'chat_reports', 'replay_shares'];
  t text;
  r text;
  n int;
begin
  foreach t in array readable || array['presence'] loop
    if to_regclass('public.' || t) is null then
      raise exception 'FAIL (S7): public.% does not exist', t;
    end if;
    if not (select relrowsecurity from pg_class where oid = ('public.' || t)::regclass) then
      raise exception 'FAIL (S7): row-level security is off on public.%', t;
    end if;
    -- Every privilege a client role holds on the table, read off the ACL itself.
    foreach r in array array['anon', 'authenticated', 'PUBLIC'] loop
      select count(*) into n
        from pg_class c
        cross join lateral aclexplode(coalesce(c.relacl, acldefault('r', c.relowner))) a
        left join pg_roles g on g.oid = a.grantee
       where c.oid = ('public.' || t)::regclass
         and coalesce(g.rolname, 'PUBLIC') = r
         and a.privilege_type <> 'SELECT';
      if n <> 0 then
        raise exception 'FAIL (S7): % holds % write privilege(s) on public.%', r, n, t;
      end if;
    end loop;
  end loop;

  foreach t in array readable loop
    select count(*) into n from pg_policies
     where schemaname = 'public' and tablename = t and cmd = 'SELECT' and roles = array['authenticated']::name[];
    if n <> 1 then
      raise exception 'FAIL (S7): public.% has % select policies for authenticated, not 1', t, n;
    end if;
    if not has_table_privilege('authenticated', 'public.' || t, 'select') then
      raise exception 'FAIL (S7): authenticated cannot select public.%; its policy would show nothing', t;
    end if;
  end loop;

  select count(*) into n from pg_policies where schemaname = 'public' and tablename = 'presence';
  if n <> 0 then
    raise exception 'FAIL (S7): public.presence has % policies; it is server-only', n;
  end if;
  foreach r in array array['anon', 'authenticated'] loop
    if has_table_privilege(r, 'public.presence', 'select') then
      raise exception 'FAIL (S7): % can read public.presence; who is online is the server''s to say', r;
    end if;
  end loop;

  select count(*) into n from information_schema.columns
   where table_schema = 'public' and table_name = 'profiles' and column_name = 'allow_spectators'
     and data_type = 'boolean' and is_nullable = 'NO' and column_default = 'true';
  if n <> 1 then
    raise exception 'FAIL (S7, B50): profiles.allow_spectators is not a non-null boolean defaulting to true';
  end if;
  raise notice 'OK (S7): RLS on all eight, one select policy on seven, presence server-only, no client write';
end $$;
rollback;

\echo '### S7: the keys and checks hold (B43, B45, B46, B48, B53) ###'
begin;
-- A third account, for pairs that do not involve profile 1.
insert into auth.users (id, email, email_confirmed_at)
values ('53700000-0000-4000-8000-000000000003', 'social3@example.test', now());
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  p3 constant uuid := '53700000-0000-4000-8000-000000000003';
  m  uuid;
  n  int;
begin
  select count(*) into n from public.profiles where id in (p1, p2, p3);
  if n <> 3 then
    raise exception 'FAIL (S7): % of the three profiles exist; the checks below would prove nothing', n;
  end if;

  -- B43: one request per direction and pair, never to oneself.
  insert into public.friend_requests (from_profile, to_profile) values (p1, p2);
  insert into public.friend_requests (from_profile, to_profile) values (p2, p1);
  begin
    insert into public.friend_requests (from_profile, to_profile) values (p1, p2);
    raise exception 'FAIL (S7, B43): a second request from p1 to p2 was stored';
  exception when unique_violation then
    if sqlerrm not like '%friend_requests%' then raise; end if;
  end;
  begin
    insert into public.friend_requests (from_profile, to_profile) values (p1, p1);
    raise exception 'FAIL (S7, B43): a request to oneself was stored';
  exception when check_violation then
    if sqlerrm not like '%friend_requests%' then raise; end if;
  end;

  -- B45: one row per pair, in order.
  insert into public.friendships (a, b) values (least(p1, p2), greatest(p1, p2));
  begin
    insert into public.friendships (a, b) values (greatest(p1, p2), least(p1, p2));
    raise exception 'FAIL (S7, B45): a friendship was stored out of order';
  exception when check_violation then
    if sqlerrm not like '%friendships%' then raise; end if;
  end;

  -- B46: never a block of oneself.
  begin
    insert into public.blocks (blocker, blocked) values (p3, p3);
    raise exception 'FAIL (S7, B46): a block of oneself was stored';
  exception when check_violation then
    if sqlerrm not like '%blocks%' then raise; end if;
  end;

  -- B48: one open challenge per challenger and friend; a closed one makes room for the next.
  insert into public.challenges (from_profile, to_profile, room_code, mode, expires_at)
  values (p1, p2, 'ABCDEF', 'bo1', now() + interval '2 minutes');
  begin
    insert into public.challenges (from_profile, to_profile, room_code, mode, expires_at)
    values (p1, p2, 'GHJKLM', 'bo1', now() + interval '2 minutes');
    raise exception 'FAIL (S7, B48): a second open challenge from p1 to p2 was stored';
  exception when unique_violation then
    if sqlerrm not like '%challenges_open_pair_key%' then raise; end if;
  end;
  update public.challenges set state = 'expired' where from_profile = p1 and to_profile = p2;
  insert into public.challenges (from_profile, to_profile, room_code, mode, expires_at)
  values (p1, p2, 'GHJKLM', 'bo1', now() + interval '2 minutes');
  begin
    update public.challenges set state = 'lost' where room_code = 'GHJKLM';
    raise exception 'FAIL (S7, B49): a challenge took the state "lost"';
  exception when check_violation then
    if sqlerrm not like '%challenges%' then raise; end if;
  end;

  -- B53: a body of 1 to 500 characters, one report per message and reporter.
  insert into public.chat_messages (from_profile, to_profile, body)
  values (p1, p2, repeat('x', 500)) returning id into m;
  foreach n in array array[0, 501] loop
    begin
      insert into public.chat_messages (from_profile, to_profile, body) values (p1, p2, repeat('x', n));
      raise exception 'FAIL (S7, B53): a body of % characters was stored', n;
    exception when check_violation then
      if sqlerrm not like '%chat_messages%' then raise; end if;
    end;
  end loop;
  insert into public.chat_reports (message_id, reporter, reason) values (m, p2, 'spam');
  begin
    insert into public.chat_reports (message_id, reporter, reason) values (m, p2, 'again');
    raise exception 'FAIL (S7, B53): a second report by the same reporter was stored';
  exception when unique_violation then
    if sqlerrm not like '%chat_reports%' then raise; end if;
  end;

  -- B53: a purged message takes its reports with it.
  delete from public.chat_messages where id = m;
  select count(*) into n from public.chat_reports where message_id = m;
  if n <> 0 then
    raise exception 'FAIL (S7, B53): % report(s) outlived their message', n;
  end if;
  raise notice 'OK (S7): one request per direction, ordered friendships, no self-block, one open challenge per pair, bodies of 1-500, one report each';
end $$;
rollback;

\echo '### S7: a client reads only the rows naming its own profile, and writes none ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('53700000-0000-4000-8000-000000000003', 'social3@example.test', now());
-- As the owner: rows of profile 1's, and rows between profiles 2 and 3 that profile 1 must not see.
insert into public.friend_requests (from_profile, to_profile) values
  ('11111111-1111-1111-1111-111111111111', '53700000-0000-4000-8000-000000000003'),
  ('22222222-2222-2222-2222-222222222222', '53700000-0000-4000-8000-000000000003');
insert into public.friendships (a, b) values
  (least('11111111-1111-1111-1111-111111111111'::uuid, '22222222-2222-2222-2222-222222222222'::uuid),
   greatest('11111111-1111-1111-1111-111111111111'::uuid, '22222222-2222-2222-2222-222222222222'::uuid)),
  (least('22222222-2222-2222-2222-222222222222'::uuid, '53700000-0000-4000-8000-000000000003'::uuid),
   greatest('22222222-2222-2222-2222-222222222222'::uuid, '53700000-0000-4000-8000-000000000003'::uuid));
insert into public.blocks (blocker, blocked) values
  ('11111111-1111-1111-1111-111111111111', '53700000-0000-4000-8000-000000000003'),
  ('53700000-0000-4000-8000-000000000003', '11111111-1111-1111-1111-111111111111');
insert into public.presence (profile_id, last_seen) values
  ('11111111-1111-1111-1111-111111111111', now()), ('22222222-2222-2222-2222-222222222222', now());
insert into public.challenges (from_profile, to_profile, room_code, mode, expires_at) values
  ('22222222-2222-2222-2222-222222222222', '11111111-1111-1111-1111-111111111111', 'PQRSTU', 'random',
   now() + interval '2 minutes'),
  ('22222222-2222-2222-2222-222222222222', '53700000-0000-4000-8000-000000000003', 'VWXYZA', 'random',
   now() + interval '2 minutes');
insert into public.chat_messages (id, from_profile, to_profile, body) values
  ('53700000-0000-4000-8000-0000000000c1', '22222222-2222-2222-2222-222222222222',
   '11111111-1111-1111-1111-111111111111', 'gg'),
  ('53700000-0000-4000-8000-0000000000c2', '22222222-2222-2222-2222-222222222222',
   '53700000-0000-4000-8000-000000000003', 'wp');
insert into public.chat_reports (message_id, reporter, reason) values
  ('53700000-0000-4000-8000-0000000000c1', '11111111-1111-1111-1111-111111111111', 'spam'),
  ('53700000-0000-4000-8000-0000000000c2', '53700000-0000-4000-8000-000000000003', 'spam');
insert into public.replay_shares (match_id, sharer, friend) values
  ('53700000-0000-4000-8000-0000000000d1', '22222222-2222-2222-2222-222222222222',
   '11111111-1111-1111-1111-111111111111'),
  ('53700000-0000-4000-8000-0000000000d2', '22222222-2222-2222-2222-222222222222',
   '53700000-0000-4000-8000-000000000003');
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [table, rows profile 1 must see, rows in the table]
  expected constant text[][] := array[
    ['friend_requests', '1', '2'], ['friendships', '1', '2'], ['blocks', '1', '2'],
    ['challenges', '1', '2'], ['chat_messages', '1', '2'], ['chat_reports', '1', '2'],
    ['replay_shares', '1', '2']];
  i     int;
  total bigint;
  seen  bigint;
begin
  for i in 1 .. array_length(expected, 1) loop
    execute format('select count(*) from public.%I', expected[i][1]) into total;
    if total <> expected[i][3]::bigint then
      raise exception 'FAIL (S7): public.% holds % rows, not %; the check would prove nothing',
        expected[i][1], total, expected[i][3];
    end if;
  end loop;

  execute 'set local role authenticated';
  perform set_config('request.jwt.claim.sub', p1::text, true);
  if current_user <> 'authenticated' then
    raise exception 'FAIL (S7): running as %, not authenticated — SET LOCAL ROLE did not take', current_user;
  end if;
  for i in 1 .. array_length(expected, 1) loop
    execute format('select count(*) from public.%I', expected[i][1]) into seen;
    if seen <> expected[i][2]::bigint then
      raise exception 'FAIL (S7): profile 1 sees % row(s) of public.%, not %', seen, expected[i][1], expected[i][2];
    end if;
  end loop;
  begin
    perform count(*) from public.presence;
    raise exception 'FAIL (S7, B47): profile 1 read public.presence';
  exception when insufficient_privilege then
    if sqlerrm not like '%presence%' then raise; end if;
  end;
  begin
    insert into public.chat_messages (from_profile, to_profile, body)
    values (p1, '22222222-2222-2222-2222-222222222222', 'forged');
    raise exception 'FAIL (S7, B53): profile 1 wrote a chat message without the server';
  exception when insufficient_privilege then
    if sqlerrm not like '%chat_messages%' then raise; end if;
  end;
  begin
    update public.challenges set state = 'accepted';
    raise exception 'FAIL (S7, B49): profile 1 changed a challenge without the server';
  exception when insufficient_privilege then
    if sqlerrm not like '%challenges%' then raise; end if;
  end;
  reset role;
  raise notice 'OK (S7): profile 1 sees its own row of each table, none of presence, and writes nothing';
end $$;
rollback;

\echo '### S7: every social row goes with either profile it names ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('53700000-0000-4000-8000-000000000003', 'social3@example.test', now());
do $$
declare
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  p3 constant uuid := '53700000-0000-4000-8000-000000000003';
  m  uuid;
  n  int;
begin
  insert into public.friend_requests (from_profile, to_profile) values (p2, p3);
  insert into public.friendships (a, b) values (least(p2, p3), greatest(p2, p3));
  insert into public.blocks (blocker, blocked) values (p3, p2);
  insert into public.presence (profile_id, last_seen) values (p3, now());
  insert into public.challenges (from_profile, to_profile, room_code, mode, expires_at)
  values (p3, p2, 'BCDEFG', 'bo1', now() + interval '2 minutes');
  insert into public.chat_messages (from_profile, to_profile, body) values (p3, p2, 'hi') returning id into m;
  insert into public.chat_reports (message_id, reporter, reason) values (m, p2, 'spam');
  insert into public.replay_shares (match_id, sharer, friend)
  values ('53700000-0000-4000-8000-0000000000d3', p3, p2);

  -- The account goes as 0012 deletes one: its auth user, whose profile goes with it.
  delete from auth.users where id = p3;
  select count(*) into n from public.profiles where id = p3;
  if n <> 0 then
    raise exception 'FAIL (S7): the deleted account''s profile is still there; the check below would prove nothing';
  end if;

  select (select count(*) from public.friend_requests where p3 in (from_profile, to_profile))
       + (select count(*) from public.friendships where p3 in (a, b))
       + (select count(*) from public.blocks where p3 in (blocker, blocked))
       + (select count(*) from public.presence where profile_id = p3)
       + (select count(*) from public.challenges where p3 in (from_profile, to_profile))
       + (select count(*) from public.chat_messages where p3 in (from_profile, to_profile))
       + (select count(*) from public.chat_reports where message_id = m)
       + (select count(*) from public.replay_shares where p3 in (sharer, friend))
    into n;
  if n <> 0 then
    raise exception 'FAIL (S7): % social row(s) outlived the deleted profile', n;
  end if;
  raise notice 'OK (S7): a deleted profile takes its requests, friendships, blocks, presence, challenges, messages, reports and shares';
end $$;
rollback;
