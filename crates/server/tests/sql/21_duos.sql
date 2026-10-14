-- Duos co-op (migration 0037, fullsend SPEC S6, B33, B34, B40; issues #936, #946). Runs after
-- 20 (0036's checks); profiles 1 and 2 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-19: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a guard,
-- and every block runs inside a transaction that is rolled back.

\echo '### S6, B40: the three co-op tables, their keys and their one check ###'
begin;
do $$
declare
  p1  constant uuid := '11111111-1111-1111-1111-111111111111';
  p2  constant uuid := '22222222-2222-2222-2222-222222222222';
  mid constant uuid := 'd0050000-0000-4000-8000-000000000001';
  t   text;
begin
  foreach t in array array['duos_rooms', 'duos_matches', 'duos_match_actions'] loop
    if not (select relrowsecurity from pg_class where oid = ('public.' || t)::regclass) then
      raise exception 'FAIL (S6): row-level security is off on public.%', t;
    end if;
    if not exists (select 1 from pg_policies
                    where schemaname = 'public' and tablename = t and cmd = 'SELECT') then
      raise exception 'FAIL (S6): public.% has no select policy for the people its rows name', t;
    end if;
    if exists (select 1 from pg_policies
                where schemaname = 'public' and tablename = t and cmd <> 'SELECT') then
      raise exception 'FAIL (S6): public.% has a policy that writes; only the server writes', t;
    end if;
  end loop;

  begin
    insert into public.duos_rooms (code, host, difficulty, host_choice, expires_at)
    values ('ABCDEF', p1, 'insane', '{"kind":"random","leanNewest":false}', now() + interval '10 minutes');
    raise exception 'FAIL (S6): a room took the difficulty "insane"';
  exception
    when check_violation then null;
  end;

  insert into public.duos_matches (id, seed, a1, a2, difficulty, options)
  values (mid, 'seed-duos-1', p1, p2, 'easy', '{}');
  insert into public.duos_match_actions (match_id, seq, nonce, action)
  values (mid, 1, 'n-1', '{"seat":"a1","nonce":"n-1","body":{"kind":"concede"}}');
  if (select count(*) from public.duos_match_actions where match_id = mid) <> 1 then
    raise exception 'FAIL (B40): the first log row did not land; the refusals below would prove nothing';
  end if;
  begin
    insert into public.duos_match_actions (match_id, seq, nonce, action) values (mid, 1, 'n-2', '{}');
    raise exception 'FAIL (B40): a second row took seq 1 of one match';
  exception
    when unique_violation then null;
  end;
  begin
    insert into public.duos_match_actions (match_id, seq, nonce, action) values (mid, 2, 'n-1', '{}');
    raise exception 'FAIL (B40): a second row took nonce n-1 of one match';
  exception
    when unique_violation then null;
  end;
  raise notice 'OK (S6, B40): RLS on with select-only policies, the difficulty checked, seq and nonce unique per match';
end $$;
rollback;

\echo '### S6: a room''s people read it and nobody else does; no client reads a co-op match or its log, or writes anything ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('d0050000-0000-4000-8000-000000000003', 'duos-outsider@example.test', now());
insert into public.duos_rooms (code, host, guest, difficulty, host_choice, guest_choice, match_id, expires_at)
values ('DUOSAB', '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222', 'medium',
        '{"kind":"random","leanNewest":false}', '{"kind":"random","leanNewest":true}',
        'd0050000-0000-4000-8000-000000000002', now() + interval '10 minutes');
insert into public.duos_matches (id, seed, a1, a2, difficulty, options)
values ('d0050000-0000-4000-8000-000000000002', 'seed-secret', '11111111-1111-1111-1111-111111111111',
        '22222222-2222-2222-2222-222222222222', 'medium', '{"decks":{}}');
insert into public.duos_match_actions (match_id, seq, nonce, action)
values ('d0050000-0000-4000-8000-000000000002', 1, 'n-1', '{"seat":"a1","nonce":"n-1"}');
do $$
declare
  p1       constant uuid := '11111111-1111-1111-1111-111111111111';
  p2       constant uuid := '22222222-2222-2222-2222-222222222222';
  outsider constant uuid := 'd0050000-0000-4000-8000-000000000003';
  mid      constant uuid := 'd0050000-0000-4000-8000-000000000002';
  -- [what, statement]. Every statement is one the owner can run (the vacuity guard below).
  hidden constant text[][] := array[
    ['duos_matches SELECT', $q$select count(*) from public.duos_matches$q$],
    ['duos_matches.seed', $q$select seed from public.duos_matches$q$],
    ['duos_matches.result', $q$select result from public.duos_matches$q$],
    ['duos_rooms.host_choice', $q$select host_choice from public.duos_rooms$q$],
    ['duos_match_actions SELECT', $q$select count(*) from public.duos_match_actions$q$],
    ['duos_match_actions.action', $q$select action from public.duos_match_actions$q$],
    ['duos_rooms INSERT',
     $q$insert into public.duos_rooms (code, host, difficulty, host_choice, expires_at)
        values ('ZZZZZZ', '11111111-1111-1111-1111-111111111111', 'easy', '{}', now())$q$],
    ['duos_matches UPDATE', $q$update public.duos_matches set result = '{}'$q$],
    ['duos_match_actions DELETE', $q$delete from public.duos_match_actions$q$]];
  who      uuid;
  n        bigint;
  i        int;
begin
  for i in 1 .. array_length(hidden, 1) loop
    begin
      execute hidden[i][2];
      raise exception 'rollback-probe';
    exception
      when raise_exception then
        if sqlerrm <> 'rollback-probe' then raise; end if;
    end;
  end loop;

  if (select count(*) from public.duos_matches where id = mid) <> 1
     or (select count(*) from public.duos_match_actions where match_id = mid) <> 1 then
    raise exception 'FAIL (S6): the match and its log row did not land; the refusals below would prove nothing';
  end if;

  set local role authenticated;
  foreach who in array array[p1, p2, outsider] loop
    perform set_config('request.jwt.claim.sub', who::text, true);
    select count(*) into n from public.duos_rooms where code = 'DUOSAB';
    if n <> (case when who = outsider then 0 else 1 end) then
      raise exception 'FAIL (S6): profile % reads % of the room, expected %', who, n,
        case when who = outsider then 0 else 1 end;
    end if;
  end loop;

  -- TRUST BOUNDARY: the seed with the decks is the whole hidden game and the log replays it, so
  -- not even the match's own people read either (as public.matches and public.match_actions).
  perform set_config('request.jwt.claim.sub', p1::text, true);
  for i in 1 .. array_length(hidden, 1) loop
    begin
      execute hidden[i][2];
      raise exception 'FAIL (S6): the room''s host ran % as a client', hidden[i][1];
    exception
      when insufficient_privilege then null;
    end;
  end loop;
  reset role;

  set local role anon;
  begin
    perform count(*) from public.duos_matches;
    raise exception 'FAIL (S6): anon read public.duos_matches';
  exception
    when insufficient_privilege then null;
  end;
  reset role;
  raise notice 'OK (S6): a room''s host and guest read it and an outsider does not, no client reads a co-op match, its log or a room''s frozen choices, and no client writes';
end $$;
rollback;

\echo '### 0012''s rule: a co-op room and a co-op match go with either of their people ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('d0050000-0000-4000-8000-000000000004', 'duos-leaver@example.test', now());
insert into public.duos_rooms (code, host, difficulty, host_choice, expires_at)
values ('DUOSCD', 'd0050000-0000-4000-8000-000000000004', 'easy', '{"kind":"random","leanNewest":false}',
        now() + interval '10 minutes');
insert into public.duos_matches (id, seed, a1, a2, difficulty, options, ended_at, result)
values ('d0050000-0000-4000-8000-000000000005', 'seed-leaver', '11111111-1111-1111-1111-111111111111',
        'd0050000-0000-4000-8000-000000000004', 'hard', '{}', now(), '{"outcome":"win","winner":"a"}');
insert into public.duos_match_actions (match_id, seq, nonce, action)
values ('d0050000-0000-4000-8000-000000000005', 1, 'n-1', '{}');
do $$
begin
  if not exists (select 1 from public.profiles where id = 'd0050000-0000-4000-8000-000000000004') then
    raise exception 'FAIL (0037): the leaver has no profile; the cascade below would prove nothing';
  end if;
  delete from auth.users where id = 'd0050000-0000-4000-8000-000000000004';
  if exists (select 1 from public.duos_rooms where code = 'DUOSCD') then
    raise exception 'FAIL (0037): the deleted account''s room outlived it';
  end if;
  if exists (select 1 from public.duos_matches where id = 'd0050000-0000-4000-8000-000000000005') then
    raise exception 'FAIL (0037): the deleted account''s co-op match outlived it';
  end if;
  if exists (select 1 from public.duos_match_actions where match_id = 'd0050000-0000-4000-8000-000000000005') then
    raise exception 'FAIL (0037): the deleted account''s co-op log outlived it';
  end if;
  if not exists (select 1 from public.profiles where id = '11111111-1111-1111-1111-111111111111') then
    raise exception 'FAIL (0037): deleting one person took their teammate''s profile';
  end if;
  raise notice 'OK (0037): deleting an account deletes its rooms and co-op matches with their logs, and nothing of its teammate';
end $$;
rollback;
