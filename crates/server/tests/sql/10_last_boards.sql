-- Last boards (migration 0017, SPEC §8.7 C+ #29, R417, R565). Runs after 09_catalog_growth.sql;
-- profiles 1 and 2 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-09: every check raises on failure (`raise exception 'FAIL (R…): …'`), every
-- expected refusal is matched on its constraint name or its SQLSTATE, never on `when others`, each
-- check that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Each SPEC §11 row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R565: a last board is one row per profile and kind, replaced by the server, gone with its profile ###'
begin;
do $$
declare
  p1    constant uuid := '11111111-1111-1111-1111-111111111111';
  gone  constant uuid := 'eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee';
  v_board constant jsonb := '[{"defId": "core-012", "radiant": false}, {"defId": "t-1:core-012+core-025", "radiant": true}]';
  v_later constant jsonb := '[{"defId": "classicplus-029", "radiant": true}]';
  v_row record;
  n     int;
  refused_by text;
begin
  if exists (select 1 from public.last_boards where profile_id = p1) then
    raise exception 'FAIL (R565): profile 1 already holds a last board, so the checks below are off';
  end if;

  -- The write results.ts makes as a game ends: an upsert on (profile_id, kind).
  insert into public.last_boards (profile_id, kind, board, created_at, updated_at)
  values (p1, 'server', v_board, '2026-01-01 00:00:00+00', '2026-01-01 00:00:00+00')
  on conflict (profile_id, kind) do update set board = excluded.board, updated_at = excluded.updated_at;
  insert into public.last_boards (profile_id, kind, board, created_at, updated_at)
  values (p1, 'server', v_later, '2026-01-01 00:05:00+00', '2026-01-01 00:05:00+00')
  on conflict (profile_id, kind) do update set board = excluded.board, updated_at = excluded.updated_at;
  select * into v_row from public.last_boards where profile_id = p1 and kind = 'server';
  if v_row.board <> v_later or v_row.created_at <> '2026-01-01 00:00:00+00' or v_row.updated_at <> '2026-01-01 00:05:00+00' then
    raise exception 'FAIL (R565): the second game''s board did not replace the first: %', v_row;
  end if;
  select count(*) into n from public.last_boards where profile_id = p1;
  if n <> 1 then
    raise exception 'FAIL (R565): % rows for one profile and kind, expected 1', n;
  end if;

  -- A practice board would be a row of its own kind, beside the server's.
  insert into public.last_boards (profile_id, kind, board) values (p1, 'practice', v_board);
  if (select board from public.last_boards where profile_id = p1 and kind = 'server') <> v_later then
    raise exception 'FAIL (R565): a practice board changed the server board';
  end if;

  -- The shape the table refuses: another kind, a board that is not an array of {defId, radiant}.
  begin
    insert into public.last_boards (profile_id, kind, board) values (p1, 'hotseat', '[]');
    raise exception 'FAIL (R565): a last board of kind hotseat was accepted';
  exception when check_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'last_boards_kind_check' then
      raise exception 'FAIL (R565): kind hotseat refused by "%"', refused_by;
    end if;
  end;
  begin
    update public.last_boards set board = '{"defId": "core-012", "radiant": false}' where profile_id = p1 and kind = 'server';
    raise exception 'FAIL (R565): a board that is an object, not an array, was accepted';
  exception when check_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'last_boards_board_check' then
      raise exception 'FAIL (R565): an object board refused by "%"', refused_by;
    end if;
  end;
  begin
    update public.last_boards set board = '[{"defId": "core-012"}]' where profile_id = p1 and kind = 'server';
    raise exception 'FAIL (R565): an entry with no face was accepted';
  exception when check_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'last_boards_board_check' then
      raise exception 'FAIL (R565): an entry with no face refused by "%"', refused_by;
    end if;
  end;
  begin
    update public.last_boards set board = '[{"defId": 12, "radiant": false}]' where profile_id = p1 and kind = 'server';
    raise exception 'FAIL (R565): an entry whose defId is a number was accepted';
  exception when check_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'last_boards_board_check' then
      raise exception 'FAIL (R565): a numeric defId refused by "%"', refused_by;
    end if;
  end;

  -- Deleting the account takes its boards with it (0012's own rows cascade).
  insert into auth.users (id, email, email_confirmed_at) values (gone, 'gone-0016@example.test', now());
  insert into public.last_boards (profile_id, kind, board) values (gone, 'server', v_board);
  delete from auth.users where id = gone;
  if exists (select 1 from public.last_boards where profile_id = gone) then
    raise exception 'FAIL (R565): a deleted profile''s last board is still stored';
  end if;

  raise notice 'OK (R565): replaced in place per (profile, kind), kinds apart, shape refused, cascaded with the profile';
end $$;
rollback;

\echo '### R417: a match keeps the boards it started with, whatever the profiles'' rows do later ###'
begin;
do $$
declare
  p1    constant uuid := '11111111-1111-1111-1111-111111111111';
  p2    constant uuid := '22222222-2222-2222-2222-222222222222';
  mid   constant uuid := 'eeeeeeee-0000-0000-0000-000000000016';
  old   constant uuid := 'eeeeeeee-0000-0000-0000-000000000017';
  v_board constant jsonb := '[{"defId": "core-012", "radiant": true}]';
  v_row record;
  refused_by text;
begin
  -- A match row as matches.create writes it, both boards frozen in.
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              p1_last_board, p2_last_board, catalog_version, ceiling_at, started_at)
  values (mid, 'live', 'seed-0016', p1, p2, '[]', '[]', v_board, '[]', 'core-1', now() + interval '1 hour', now());
  -- A row written without them (an open room, any match from before 0017) reads the empty board.
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (old, 'over', 'seed-0015', p1, p2, '[]', '[]', 'core-1', now(), now(), now());

  insert into public.last_boards (profile_id, kind, board) values (p1, 'server', '[{"defId": "core-025", "radiant": false}]')
  on conflict (profile_id, kind) do update set board = excluded.board;
  select p1_last_board, p2_last_board into v_row from public.matches where id = mid;
  if v_row.p1_last_board <> v_board or v_row.p2_last_board <> '[]'::jsonb then
    raise exception 'FAIL (R417): the match''s frozen boards read %, expected p1 % and an empty p2', v_row, v_board;
  end if;
  select p1_last_board, p2_last_board into v_row from public.matches where id = old;
  if v_row.p1_last_board <> '[]'::jsonb or v_row.p2_last_board <> '[]'::jsonb then
    raise exception 'FAIL (R417): a match written without boards reads %, expected two empty boards', v_row;
  end if;
  -- The live-match rebuild reads them: app.live_matches() returns the whole row.
  if not exists (select 1 from app.live_matches() m where m.id = mid and m.p1_last_board = v_board) then
    raise exception 'FAIL (R417): app.live_matches() does not carry the frozen board a restart folds with';
  end if;

  begin
    update public.matches set p2_last_board = '[{"radiant": true}]' where id = mid;
    raise exception 'FAIL (R417): a frozen board entry with no defId was accepted';
  exception when check_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'matches_last_boards_check' then
      raise exception 'FAIL (R417): a malformed frozen board refused by "%"', refused_by;
    end if;
  end;

  raise notice 'OK (R417): the match row keeps its starting boards, defaults to empty, and live_matches carries them';
end $$;
rollback;

\echo '### R565: no client may read or write a last board ###'
begin;
-- Vacuity guards, as the owner: the table exists and holds a row of profile 1's own to be refused,
-- and the very statements the client attempts below succeed for the owner.
insert into public.last_boards (profile_id, kind, board)
values ('11111111-1111-1111-1111-111111111111', 'server', '[{"defId": "core-012", "radiant": false}]')
on conflict (profile_id, kind) do update set board = excluded.board;
do $$
begin
  begin
    update public.last_boards set board = '[]' where profile_id = '11111111-1111-1111-1111-111111111111';
    delete from public.last_boards where profile_id = '11111111-1111-1111-1111-111111111111';
    insert into public.last_boards (profile_id, kind, board) values ('11111111-1111-1111-1111-111111111111', 'server', '[]');
    raise exception 'rollback-probe';
  exception when raise_exception then
    if sqlerrm <> 'rollback-probe' then raise; end if;
  end;
end $$;

set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);
do $$
declare
  probes constant text[] := array[
    $q$select count(*) from public.last_boards$q$,
    $q$select count(*) from public.last_boards where profile_id = '11111111-1111-1111-1111-111111111111'$q$,
    $q$insert into public.last_boards (profile_id, kind, board) values ('11111111-1111-1111-1111-111111111111', 'practice', '[]')$q$,
    $q$update public.last_boards set board = '[]'$q$,
    $q$delete from public.last_boards$q$,
    $q$select p1_last_board from public.matches$q$];
  probe text;
  readable bigint;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R565): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  -- The session still reaches what a client may read, so a refusal below is about these tables.
  select count(*) into readable from public.cards;
  if readable < 1 then
    raise exception 'FAIL (R565): this session reads no public.cards either; the refusals below would prove nothing';
  end if;
  foreach probe in array probes loop
    begin
      execute probe;
      raise exception 'FAIL (R565): a client ran "%" — a last board is the server''s alone', probe;
    exception
      when insufficient_privilege then null;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R565): "%" raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;
  raise notice 'OK (R565): a client can neither read nor write public.last_boards, nor read a match''s frozen boards';
end $$;
reset role;

-- And the policy side: RLS is on and no policy names the table for any role.
do $$
begin
  if not (select relrowsecurity from pg_class where oid = 'public.last_boards'::regclass) then
    raise exception 'FAIL (R565): row-level security is off on public.last_boards';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'last_boards') then
    raise exception 'FAIL (R565): public.last_boards has a policy; it is meant to have none (server only)';
  end if;
  raise notice 'OK (R565): RLS on with no policy';
end $$;
rollback;
