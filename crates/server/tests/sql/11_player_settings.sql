-- Player settings on the account (migration 0018, SPEC §9.1, R633, R634), as the server
-- (postgres/service_role) drives it. Runs after 10_last_boards.sql; profiles 1, 2 and 3 are
-- active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-10 (see 03's header for why each one exists): a check FAILS LOUDLY with
-- `raise exception 'FAIL (R633): ...'`, an expected refusal is matched on its exact message prefix
-- or constraint name, and a check that could be trivially true carries a control that succeeds.
--
-- Each SPEC §11 row this file proves is named in a `### Rnnn: … ###` heading, which is how the
-- §11 index (packages/engine/test/rulings.test.ts) credits an SQL file. What a CLIENT may do with
-- the table is 02_rls_as_client.sql's R633 block. Everything here is rolled back.

\echo '### R633: app.merge_player_settings keeps each group the strictly newest write left, and caps the row ###'
\echo '### R634: a group replaces the stored one only when its time is strictly later ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  t0 constant timestamptz := '2026-01-01 00:00:00+00';
  t1 constant timestamptz := '2026-01-01 00:01:00+00';
  t2 constant timestamptz := '2026-01-01 00:02:00+00';
  r     text;
  v_row record;
begin
  -- The two app.settings rows 0018 seeds mirror apps/server/src/config.ts.
  if (app.setting('player_settings_groups_max'))::text::int <> 8
     or (app.setting('player_settings_bytes_max'))::text::int <> 4096 then
    raise exception 'FAIL (R633): app.settings holds % groups / % bytes, expected 8 / 4096 as config.ts',
      app.setting('player_settings_groups_max'), app.setting('player_settings_bytes_max');
  end if;
  if exists (select 1 from public.player_settings where profile_id in (p1, p2)) then
    raise exception 'FAIL (R633): profiles 1 and 2 already hold settings rows, so the checks below are off';
  end if;

  -- The first write makes the row.
  r := app.merge_player_settings(p1,
    '{"audio": {"at": 1000, "values": {"master": 0.5, "muted": false}},
      "gameplay": {"at": 1000, "values": {"dragToPlay": true}}}', t0, 8, 4096);
  if r <> 'merged' then
    raise exception 'FAIL (R633): the first write answered %, expected merged', r;
  end if;
  select * into v_row from public.player_settings where profile_id = p1;
  if v_row.groups -> 'audio' -> 'values' ->> 'master' <> '0.5'
     or v_row.groups -> 'gameplay' ->> 'at' <> '1000'
     or v_row.created_at <> t0 or v_row.updated_at <> t0 then
    raise exception 'FAIL (R633): the first write reads back as %', v_row;
  end if;

  -- R634: a strictly later group replaces the stored one whole; groups the write does not name stay.
  perform app.merge_player_settings(p1, '{"audio": {"at": 2000, "values": {"master": 0.9}}}', t1, 8, 4096);
  select * into v_row from public.player_settings where profile_id = p1;
  if v_row.groups -> 'audio' ->> 'at' <> '2000'
     or v_row.groups -> 'audio' -> 'values' ->> 'master' <> '0.9'
     or v_row.groups -> 'audio' -> 'values' ? 'muted'
     or v_row.groups -> 'gameplay' ->> 'at' <> '1000'
     or v_row.created_at <> t0 or v_row.updated_at <> t1 then
    raise exception 'FAIL (R634): a later audio group read back as % (it replaces the group whole, and leaves gameplay alone)', v_row;
  end if;

  -- An older group changes nothing, and neither does a tie (the same write twice is the same write once).
  perform app.merge_player_settings(p1, '{"audio": {"at": 1500, "values": {"master": 0.1}}}', t2, 8, 4096);
  perform app.merge_player_settings(p1, '{"audio": {"at": 2000, "values": {"master": 0.2}}}', t2, 8, 4096);
  select * into v_row from public.player_settings where profile_id = p1;
  if v_row.groups -> 'audio' -> 'values' ->> 'master' <> '0.9' or v_row.groups -> 'audio' ->> 'at' <> '2000' then
    raise exception 'FAIL (R634): an older and a tied audio group changed the stored one: %', v_row.groups -> 'audio';
  end if;

  -- A write that changes one group and is older for another takes only the newer one.
  perform app.merge_player_settings(p1,
    '{"audio": {"at": 100, "values": {"master": 0.3}}, "fx": {"at": 100, "values": {"speed": 2}}}', t2, 8, 4096);
  select * into v_row from public.player_settings where profile_id = p1;
  if v_row.groups -> 'audio' ->> 'at' <> '2000' or v_row.groups -> 'fx' -> 'values' ->> 'speed' <> '2' then
    raise exception 'FAIL (R634): a write older for audio and new for fx read back as %', v_row.groups;
  end if;

  raise notice 'OK (R634): a later group replaces whole, older and tied groups change nothing, and a mixed write takes only what is newer';
end $$;
rollback;

begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  t0 constant timestamptz := '2026-01-01 00:00:00+00';
  r     text;
  v_row record;
begin
  perform app.merge_player_settings(p1, '{"a": {"at": 1, "values": {}}, "b": {"at": 1, "values": {}}}', t0, 8, 4096);

  -- Two groups held and a cap of 2: a third is refused, and the row is exactly as it was.
  r := app.merge_player_settings(p1, '{"c": {"at": 9, "values": {"x": true}}}', t0, 2, 4096);
  if r <> 'limit' then
    raise exception 'FAIL (R633): a third group under a cap of 2 answered %, expected limit', r;
  end if;
  select * into v_row from public.player_settings where profile_id = p1;
  if v_row.groups ? 'c' or (select count(*) from jsonb_object_keys(v_row.groups)) <> 2 then
    raise exception 'FAIL (R633): a write refused at the group cap still changed the row: %', v_row.groups;
  end if;
  -- A group already held may still be replaced at the cap.
  r := app.merge_player_settings(p1, '{"a": {"at": 9, "values": {"x": true}}}', t0, 2, 4096);
  if r <> 'merged' then
    raise exception 'FAIL (R633): replacing a held group at the cap answered %, expected merged', r;
  end if;

  -- The byte cap, the caller's: the merged row's text.
  r := app.merge_player_settings(p1, jsonb_build_object('a', jsonb_build_object('at', 10,
        'values', jsonb_build_object('s', repeat('x', 300)))), t0, 8, 200);
  if r <> 'limit' then
    raise exception 'FAIL (R633): 300 bytes under a cap of 200 answered %, expected limit', r;
  end if;
  if (select groups -> 'a' ->> 'at' from public.player_settings where profile_id = p1) <> '9' then
    raise exception 'FAIL (R633): a write refused at the byte cap still changed the row';
  end if;

  -- The database's own caps, when the caller asks for more than app.settings allows.
  update app.settings set value = to_jsonb(2) where key = 'player_settings_groups_max';
  r := app.merge_player_settings(p1, '{"c": {"at": 9, "values": {}}}', t0, 8, 4096);
  update app.settings set value = to_jsonb(8) where key = 'player_settings_groups_max';
  if r <> 'limit' then
    raise exception 'FAIL (R633): with app.settings at 2 groups and the caller at 8 a third answered %, expected limit', r;
  end if;
  update app.settings set value = to_jsonb(100) where key = 'player_settings_bytes_max';
  r := app.merge_player_settings(p1, jsonb_build_object('a', jsonb_build_object('at', 11,
        'values', jsonb_build_object('s', repeat('x', 300)))), t0, 8, 4096);
  update app.settings set value = to_jsonb(4096) where key = 'player_settings_bytes_max';
  if r <> 'limit' then
    raise exception 'FAIL (R633): with app.settings at 100 bytes and the caller at 4096 300 bytes answered %, expected limit', r;
  end if;

  raise notice 'OK (R633, R634): created, a strictly later group replaces whole, older and tied groups change nothing, groups apart, limit at both caps (the caller''s and app.settings'')';
end $$;
rollback;

\echo '-- each profile''s row is its own'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  t0 constant timestamptz := '2026-01-01 00:00:00+00';
begin
  perform app.merge_player_settings(p1, '{"audio": {"at": 1, "values": {"master": 0.1}}}', t0, 8, 4096);
  perform app.merge_player_settings(p2, '{"audio": {"at": 1, "values": {"master": 0.2}}}', t0, 8, 4096);
  perform app.merge_player_settings(p2, '{"fx": {"at": 3, "values": {"speed": 2}}}', t0, 8, 4096);
  if (select groups -> 'audio' -> 'values' ->> 'master' from public.player_settings where profile_id = p1) <> '0.1'
     or (select groups ? 'fx' from public.player_settings where profile_id = p1) then
    raise exception 'FAIL (R633): profile 2''s writes changed profile 1''s row';
  end if;
  raise notice 'OK (R633): two profiles'' rows are apart';
end $$;
rollback;

\echo '-- the shape app.merge_player_settings and the table refuse (the server refuses it first)'
begin;
do $$
declare
  -- [label, call, expected message prefix]
  probes constant text[][] := array[
    ['no groups at all',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', null, now(), 8, 4096)$q$,
     'settings: p_groups must be an object'],
    ['a list of groups',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '[]', now(), 8, 4096)$q$,
     'settings: p_groups must be an object'],
    ['a group that is not an object',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '{"a": 1}', now(), 8, 4096)$q$,
     'settings: group a must be'],
    ['a group with no values',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '{"a": {"at": 1}}', now(), 8, 4096)$q$,
     'settings: group a must be'],
    ['a group with no time',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '{"a": {"values": {}}}', now(), 8, 4096)$q$,
     'settings: group a must be'],
    ['a group whose time is a string',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '{"a": {"at": "1", "values": {}}}', now(), 8, 4096)$q$,
     'settings: group a must be'],
    ['a negative time',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '{"a": {"at": -1, "values": {}}}', now(), 8, 4096)$q$,
     'settings: group a must be'],
    ['a fractional time',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '{"a": {"at": 1.5, "values": {}}}', now(), 8, 4096)$q$,
     'settings: group a must be'],
    ['values that are a list',
     $q$select app.merge_player_settings('11111111-1111-1111-1111-111111111111', '{"a": {"at": 1, "values": []}}', now(), 8, 4096)$q$,
     'settings: group a must be'],
    ['a profile that does not exist',
     $q$select app.merge_player_settings('99999999-9999-4999-8999-999999999999', '{}', now(), 8, 4096)$q$,
     'settings: profile 99999999-9999-4999-8999-999999999999 not found']];
  refused_by text;
  i int;
begin
  -- Vacuity guard: the same call with a well-formed body succeeds, so each refusal below is about
  -- the one thing its probe changed.
  if app.merge_player_settings('11111111-1111-1111-1111-111111111111',
       '{"a": {"at": 1, "values": {"k": true}}}', now(), 8, 4096) <> 'merged' then
    raise exception 'FAIL (R633): the control write was not merged, so the refusals below prove nothing';
  end if;
  delete from public.player_settings where profile_id = '11111111-1111-1111-1111-111111111111';

  for i in 1 .. array_length(probes, 1) loop
    begin
      execute probes[i][2];
      raise exception 'FAIL (R633): % was merged — app.merge_player_settings must refuse it', probes[i][1];
    exception when others then
      if sqlerrm like 'FAIL%' then raise; end if;
      if sqlerrm not like probes[i][3] || '%' then
        raise exception 'FAIL (R633): % raised "%" (%), expected a message starting "%"',
          probes[i][1], sqlerrm, sqlstate, probes[i][3];
      end if;
    end;
  end loop;
  if exists (select 1 from public.player_settings where profile_id = '11111111-1111-1111-1111-111111111111') then
    raise exception 'FAIL (R633): a refused write left a row behind';
  end if;

  -- The table holds the same line without the function: the groups are an object.
  begin
    insert into public.player_settings (profile_id, groups) values ('11111111-1111-1111-1111-111111111111', '[]');
    raise exception 'FAIL (R633): a raw row held a list for its groups';
  exception
    when check_violation then
      get stacked diagnostics refused_by = constraint_name;
      if refused_by is distinct from 'player_settings_groups_object_check' then
        raise exception 'FAIL (R633): the list was refused by "%", not player_settings_groups_object_check', refused_by;
      end if;
  end;

  raise notice 'OK (R633): a missing or non-object list, a group without an object, a time that is missing, a string, negative or fractional, values that are a list and an unknown profile refused; the table refuses a list';
end $$;
rollback;

\echo '-- a profile that is not active keeps no settings on the account (rolled back)'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('66666666-6666-6666-6666-666666666666', 'pending-settings@example.test', now());
do $$
begin
  if (select status from public.profiles where id = '66666666-6666-6666-6666-666666666666') <> 'pending' then
    raise exception 'FAIL (R633): profile 6 is not pending, so the gate below is not exercised';
  end if;
  perform app.merge_player_settings('66666666-6666-6666-6666-666666666666',
    '{"audio": {"at": 1, "values": {}}}', now(), 8, 4096);
  raise exception 'FAIL (R633): a pending profile wrote settings — §9.4 gives it nothing but the code screen';
exception when others then
  if sqlerrm like 'FAIL%' then raise; end if;
  if sqlerrm not like 'settings: profile % is not active (status=pending)' then
    raise exception 'FAIL (R633): the pending profile was refused with "%" (%), not the active-profile gate',
      sqlerrm, sqlstate;
  end if;
  raise notice 'OK (R633): %', sqlerrm;
end $$;
rollback;

\echo '### ALL PLAYER SETTINGS CHECKS RAN ###'
