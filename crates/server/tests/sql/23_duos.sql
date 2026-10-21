-- Duos co-op (migrations 0039 and 0051, fullsend SPEC S6, B33, B34, B40; issues #936, #946). Runs after
-- 22 (0038's checks); profiles 1 and 2 are active by then (03 activated them).
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
    -- Only a room is read by its people; a match and its log are server-only (TRUST BOUNDARY):
    -- no policy, so RLS denies every row even should a grant ever be added.
    if t = 'duos_rooms' and not exists (select 1 from pg_policies
                    where schemaname = 'public' and tablename = t and cmd = 'SELECT') then
      raise exception 'FAIL (S6): public.% has no select policy for the people its rows name', t;
    end if;
    if t <> 'duos_rooms' and exists (select 1 from pg_policies
                where schemaname = 'public' and tablename = t) then
      raise exception 'FAIL (S6): public.% has a policy; a co-op match and its log are server-only', t;
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
  raise notice 'OK (S6, B40): RLS on, a select policy on rooms only, the difficulty checked, seq and nonce unique per match';
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

\echo '### R1497000 (0051): a deleted account''s room goes, and its seat on a finished co-op match empties while the match and its log stay ###'
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
    raise exception 'FAIL (R1497000): the leaver has no profile; the delete below would prove nothing';
  end if;
  delete from auth.users where id = 'd0050000-0000-4000-8000-000000000004';
  if exists (select 1 from public.duos_rooms where code = 'DUOSCD') then
    raise exception 'FAIL (R1497000): the deleted account''s room outlived it';
  end if;
  if not exists (select 1 from public.duos_matches
                  where id = 'd0050000-0000-4000-8000-000000000005'
                    and a1 = '11111111-1111-1111-1111-111111111111' and a2 is null
                    and result = '{"outcome":"win","winner":"a"}'::jsonb) then
    raise exception 'FAIL (R1497000): the finished co-op match did not stay for the teammate with the deleted seat empty';
  end if;
  if (select count(*) from public.duos_match_actions
       where match_id = 'd0050000-0000-4000-8000-000000000005') <> 1 then
    raise exception 'FAIL (R1497000): the finished co-op match''s log did not stay with it';
  end if;
  if not exists (select 1 from public.profiles where id = '11111111-1111-1111-1111-111111111111') then
    raise exception 'FAIL (R1497000): deleting one person took their teammate''s profile';
  end if;
  raise notice 'OK (R1497000): deleting an account deletes its rooms and empties its seat on a finished co-op match, which keeps its result and its log, and takes nothing of its teammate';
end $$;
rollback;

\echo '### R1497000 (0051): a live co-op match refuses the delete of either of its people ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('d0050000-0000-4000-8000-000000000006', 'duos-mid-game@example.test', now());
do $$
declare
  mate constant uuid := '11111111-1111-1111-1111-111111111111';
  gone constant uuid := 'd0050000-0000-4000-8000-000000000006';
  mid  constant uuid := 'd0050000-0000-4000-8000-000000000007';
  seat text;
  refused_by text;
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R1497000): the player has no profile; the refusals below would prove nothing';
  end if;
  foreach seat in array array['a1', 'a2'] loop
    delete from public.duos_matches where id = mid;
    insert into public.duos_matches (id, seed, a1, a2, difficulty, options)
    values (mid, 'seed-live', case when seat = 'a1' then gone else mate end,
            case when seat = 'a1' then mate else gone end, 'easy', '{}');
    insert into public.duos_match_actions (match_id, seq, nonce, action) values (mid, 1, 'n-1', '{}');
    begin
      delete from auth.users where id = gone;
      raise exception 'FAIL (R1497000): a person in seat % of a live co-op match was deleted', seat;
    exception
      when check_violation then
        get stacked diagnostics refused_by = constraint_name;
        if refused_by is distinct from 'duos_matches_seats_filled_unless_over_check' then
          raise exception 'FAIL (R1497000): the live co-op match (seat %) was refused by "%"', seat, refused_by;
        end if;
    end;
    if not exists (select 1 from public.profiles where id = gone)
       or not exists (select 1 from public.duos_matches
                       where id = mid and ended_at is null and a1 is not null and a2 is not null)
       or (select count(*) from public.duos_match_actions where match_id = mid) <> 1 then
      raise exception 'FAIL (R1497000): the refused delete (seat %) changed the profile, the match or its log', seat;
    end if;
  end loop;
  -- Once the match has ended, the same delete goes through and empties the seat.
  update public.duos_matches set ended_at = now(), result = '{"outcome":"draw"}' where id = mid;
  delete from auth.users where id = gone;
  if not exists (select 1 from public.duos_matches where id = mid and a1 = mate and a2 is null) then
    raise exception 'FAIL (R1497000): once the co-op match ended, the delete did not empty the seat';
  end if;
  raise notice 'OK (R1497000): a live co-op match refuses the delete of either seat''s person, and a match that has ended lets it go';
end $$;
rollback;
