-- Custom cards on the account (migration 0037, SPEC §12.9, R915008; issue #918). Runs after
-- 20_lean_set.sql; profiles 1, 2 and 3 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-20: every check raises on failure (`raise exception 'FAIL (R…): …'`), every
-- expected refusal is matched on its sqlstate and on the object it names, each check that could pass
-- vacuously carries a guard, and every block runs inside a transaction that is rolled back.

\echo '### R915008: app.create_custom_card writes a card at version 1, under the account''s cap ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  t0 constant timestamptz := '2026-10-01 00:00:00+00';
  frame constant jsonb := '{"name": "Spark", "type": "Spell", "cost": 1, "rarity": "Common", "text": ""}';
  r     text;
  v_row record;
  n     int;
begin
  -- The app.settings row 0037 seeds mirrors crates/server/src/config.rs CUSTOM_CARDS_PER_ACCOUNT.
  if (app.setting('max_custom_cards'))::text::int <> 50 then
    raise exception 'FAIL (R915008): app.settings holds % custom cards, expected 50 as config.rs',
      app.setting('max_custom_cards');
  end if;
  if exists (select 1 from public.custom_cards where profile_id in (p1, p2)) then
    raise exception 'FAIL (R915008): profiles 1 and 2 already hold custom cards, so the checks below are off';
  end if;

  r := app.create_custom_card(p1, 'custom-aa', frame, 'return Card {}', '{"steps": []}', t0, 50);
  if r <> 'created' then
    raise exception 'FAIL (R915008): the first create answered %, expected created', r;
  end if;
  select * into v_row from public.custom_cards where id = 'custom-aa';
  if v_row.profile_id <> p1 or v_row.version <> 1 or v_row.frame <> frame
     or v_row.source <> 'return Card {}' or v_row.builder <> '{"steps": []}'::jsonb
     or v_row.created_at <> t0 or v_row.updated_at <> t0 then
    raise exception 'FAIL (R915008): the created card reads back as %', v_row;
  end if;

  -- The caller's cap: two held and a cap of 2, a third is refused and nothing is written.
  perform app.create_custom_card(p1, 'custom-ab', frame, '', null, t0, 50);
  r := app.create_custom_card(p1, 'custom-ac', frame, '', null, t0, 2);
  if r <> 'limit' then
    raise exception 'FAIL (R915008): a third card under a cap of 2 answered %, expected limit', r;
  end if;
  if exists (select 1 from public.custom_cards where id = 'custom-ac') then
    raise exception 'FAIL (R915008): a create refused at the cap still wrote its card';
  end if;
  -- Another profile's cards do not count against this one's.
  r := app.create_custom_card(p2, 'custom-ba', frame, '', null, t0, 1);
  if r <> 'created' then
    raise exception 'FAIL (R915008): profile 2''s first card under a cap of 1 answered %, expected created', r;
  end if;

  -- The database's own cap, when the caller asks for more than app.settings allows.
  update app.settings set value = to_jsonb(2) where key = 'max_custom_cards';
  r := app.create_custom_card(p1, 'custom-ad', frame, '', null, t0, 50);
  update app.settings set value = to_jsonb(50) where key = 'max_custom_cards';
  if r <> 'limit' then
    raise exception 'FAIL (R915008): with app.settings at 2 cards and the caller at 50 a third answered %, expected limit', r;
  end if;

  -- An id already held is refused by the primary key, whoever holds it.
  begin
    perform app.create_custom_card(p2, 'custom-aa', frame, '', null, t0, 50);
    raise exception 'FAIL (R915008): a create took the id custom-aa, which profile 1 holds';
  exception
    when unique_violation then
      if sqlerrm not like '%custom_cards_pkey%' then
        raise exception 'FAIL (R915008): the doubled id was refused by "%", not custom_cards_pkey', sqlerrm;
      end if;
  end;
  select count(*) into n from public.custom_cards where id = 'custom-aa' and profile_id = p1;
  if n <> 1 then
    raise exception 'FAIL (R915008): profile 1''s custom-aa is gone or doubled after the refused create (% rows)', n;
  end if;

  raise notice 'OK (R915008): created at version 1, limit at both caps (the caller''s and app.settings''), per profile, and a held id refused';
end $$;
rollback;

\echo '### R915008: app.update_custom_card replaces a card whole and counts the save, or answers stale or missing ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  p2 constant uuid := '22222222-2222-2222-2222-222222222222';
  t0 constant timestamptz := '2026-10-01 00:00:00+00';
  t1 constant timestamptz := '2026-10-02 00:00:00+00';
  frame  constant jsonb := '{"name": "Spark", "type": "Spell", "cost": 1, "rarity": "Common", "text": ""}';
  edited constant jsonb := '{"name": "Big Spark", "type": "Spell", "cost": 2, "rarity": "Rare", "text": ""}';
  r     text;
  v_row record;
begin
  perform app.create_custom_card(p1, 'custom-aa', frame, 'old', '{"a": 1}', t0, 50);

  r := app.update_custom_card(p1, 'custom-aa', 1, edited, 'new', null, t1);
  if r <> 'updated' then
    raise exception 'FAIL (R915008): a save of version 1 answered %, expected updated', r;
  end if;
  select * into v_row from public.custom_cards where id = 'custom-aa';
  if v_row.version <> 2 or v_row.frame <> edited or v_row.source <> 'new' or v_row.builder is not null
     or v_row.created_at <> t0 or v_row.updated_at <> t1 then
    raise exception 'FAIL (R915008): the saved card reads back as % (version 2, replaced whole, made at t0, saved at t1)', v_row;
  end if;

  -- A second save of version 1 is stale: version 2 came first. Nothing is written.
  r := app.update_custom_card(p1, 'custom-aa', 1, frame, 'stale', null, t1);
  if r <> 'stale' then
    raise exception 'FAIL (R915008): a save of version 1 over version 2 answered %, expected stale', r;
  end if;
  if (select source from public.custom_cards where id = 'custom-aa') <> 'new' then
    raise exception 'FAIL (R915008): a stale save still changed the card';
  end if;

  -- Another profile's card, and no card at all, are both missing; nothing is written.
  r := app.update_custom_card(p2, 'custom-aa', 2, frame, 'theirs', null, t1);
  if r <> 'missing' then
    raise exception 'FAIL (R915008): profile 2 saving profile 1''s card answered %, expected missing', r;
  end if;
  r := app.update_custom_card(p1, 'custom-zz', 1, frame, '', null, t1);
  if r <> 'missing' then
    raise exception 'FAIL (R915008): a save of a card nobody made answered %, expected missing', r;
  end if;
  select * into v_row from public.custom_cards where id = 'custom-aa';
  if v_row.version <> 2 or v_row.source <> 'new' then
    raise exception 'FAIL (R915008): profile 2''s refused save changed profile 1''s card: %', v_row;
  end if;

  raise notice 'OK (R915008): a save replaces the card whole and counts it, keeps created_at, and a stale or missing one writes nothing';
end $$;
rollback;

\echo '### R915008: only an active account saves custom cards ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('b9150080-0000-4000-8000-000000000001', 'pending915008@example.test', now());
do $$
declare
  pending constant uuid := 'b9150080-0000-4000-8000-000000000001';
  frame   constant jsonb := '{"name": "Spark", "type": "Spell", "cost": 1, "rarity": "Common", "text": ""}';
  probe   text;
begin
  if (select status from public.profiles where id = pending) is distinct from 'pending' then
    raise exception 'FAIL (R915008): the new account is not pending, so its refusal proves nothing';
  end if;
  foreach probe in array array['create', 'update'] loop
    begin
      if probe = 'create' then
        perform app.create_custom_card(pending, 'custom-pp', frame, '', null, now(), 50);
      else
        perform app.update_custom_card(pending, 'custom-pp', 1, frame, '', null, now());
      end if;
      raise exception 'FAIL (R915008): a pending account''s % was not refused', probe;
    exception
      when raise_exception then
        if sqlerrm like 'FAIL%' then raise; end if;
        if sqlerrm not like 'custom card: profile % is not active%' then
          raise exception 'FAIL (R915008): a pending account''s % raised "%", not the active check', probe, sqlerrm;
        end if;
    end;
  end loop;
  raise notice 'OK (R915008): a pending account''s create and save are both refused';
end $$;
rollback;

\echo '### R915008: the table holds a card to the engine''s limits ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  frame constant jsonb := '{"name": "Spark", "type": "Spell", "cost": 1, "rarity": "Common", "text": ""}';
  refused_by text;
  probes constant text[][] := array[
    ['an empty tail',            $q$insert into public.custom_cards (id, profile_id, frame) values ('custom-', '11111111-1111-1111-1111-111111111111', '{}')$q$, 'custom_cards_id_check'],
    ['an upper-case id',         $q$insert into public.custom_cards (id, profile_id, frame) values ('custom-ABC', '11111111-1111-1111-1111-111111111111', '{}')$q$, 'custom_cards_id_check'],
    ['a catalog id',             $q$insert into public.custom_cards (id, profile_id, frame) values ('core-001', '11111111-1111-1111-1111-111111111111', '{}')$q$, 'custom_cards_id_check'],
    ['a 65-character id',        $q$insert into public.custom_cards (id, profile_id, frame) values ('custom-' || repeat('a', 58), '11111111-1111-1111-1111-111111111111', '{}')$q$, 'custom_cards_id_check'],
    ['version 0',                $q$insert into public.custom_cards (id, profile_id, version, frame) values ('custom-v0', '11111111-1111-1111-1111-111111111111', 0, '{}')$q$, 'custom_cards_version_check'],
    ['a frame that is a list',   $q$insert into public.custom_cards (id, profile_id, frame) values ('custom-fl', '11111111-1111-1111-1111-111111111111', '[]')$q$, 'custom_cards_frame_check'],
    ['a source of 8193 bytes',   $q$insert into public.custom_cards (id, profile_id, frame, source) values ('custom-sb', '11111111-1111-1111-1111-111111111111', '{}', repeat('x', 8193))$q$, 'custom_cards_source_check'],
    ['8194 bytes in 4097 chars', $q$insert into public.custom_cards (id, profile_id, frame, source) values ('custom-sc', '11111111-1111-1111-1111-111111111111', '{}', repeat('é', 4097))$q$, 'custom_cards_source_check'],
    ['a builder past its bound', $q$insert into public.custom_cards (id, profile_id, frame, builder) values ('custom-bb', '11111111-1111-1111-1111-111111111111', '{}', jsonb_build_object('s', repeat('x', 32768)))$q$, 'custom_cards_builder_check']];
  i int;
begin
  -- Controls: each limit admits what is at it, so a refusal below is the limit's and not the row's.
  insert into public.custom_cards (id, profile_id, frame, source, builder)
  values ('custom-' || repeat('a', 57), p1, frame, repeat('x', 8192), jsonb_build_object('s', repeat('x', 16384))),
         ('custom-0-9', p1, frame, repeat('é', 4096), null);
  if (select count(*) from public.custom_cards where profile_id = p1) <> 2 then
    raise exception 'FAIL (R915008): the cards at every limit were not both written';
  end if;

  for i in 1 .. array_length(probes, 1) loop
    begin
      execute probes[i][2];
      raise exception 'FAIL (R915008): the table took %', probes[i][1];
    exception
      when check_violation then
        get stacked diagnostics refused_by = constraint_name;
        if refused_by <> probes[i][3] then
          raise exception 'FAIL (R915008): % was refused by %, not %', probes[i][1], refused_by, probes[i][3];
        end if;
    end;
  end loop;
  raise notice 'OK (R915008): the id''s form, a version from 1, a frame object, 8192 bytes of source and the builder''s bound hold, and each limit admits what is at it';
end $$;
rollback;

\echo '### R915008: a player reads only its own custom cards and writes none of them ###'
begin;

-- One card for EACH profile: profile 2's is the row to hide from profile 1 (and profile 1's from
-- profile 2). Raw inserts, as superuser, because the point is what RLS shows.
insert into public.custom_cards (id, profile_id, frame, source) values
  ('custom-mine', '11111111-1111-1111-1111-111111111111', '{"name": "Mine"}', 'return Card {}'),
  ('custom-theirs', '22222222-2222-2222-2222-222222222222', '{"name": "Theirs"}', '');

-- Preflight, as in R633's block (02): every write the client is about to be refused is one the
-- owner can make, inside a subtransaction rolled back at once.
do $$
begin
  begin
    insert into public.custom_cards (id, profile_id, frame)
    values ('custom-probe', '11111111-1111-1111-1111-111111111111', '{}');
    update public.custom_cards set source = 'x';
    delete from public.custom_cards;
    perform app.create_custom_card('11111111-1111-1111-1111-111111111111', 'custom-fn', '{}', '', null, now(), 50);
    perform app.update_custom_card('11111111-1111-1111-1111-111111111111', 'custom-fn', 1, '{}', '', null, now());
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception
        'FAIL (R915008): the owner could not run the writes the client is about to be refused ("%", %) — the refusals below would prove nothing about privileges',
        sqlerrm, sqlstate;
    end if;
  end;
  perform set_config('rls915008.total', (select count(*) from public.custom_cards)::text, true);
end $$;

set local role authenticated;
select set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true);

do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  total  bigint := coalesce(nullif(current_setting('rls915008.total', true), ''), '-1')::bigint;
  seen   bigint;
  probe  text;
  probes constant text[][] := array[
    ['custom_cards INSERT',
     $q$insert into public.custom_cards (id, profile_id, frame)
        values ('custom-probe', '11111111-1111-1111-1111-111111111111', '{}')$q$,
     'custom_cards'],
    ['custom_cards UPDATE', $q$update public.custom_cards set source = 'x'$q$, 'custom_cards'],
    ['custom_cards DELETE', $q$delete from public.custom_cards$q$, 'custom_cards'],
    ['app.create_custom_card',
     $q$select app.create_custom_card('11111111-1111-1111-1111-111111111111', 'custom-fn', '{}', '', null, now(), 50)$q$,
     'create_custom_card'],
    ['app.update_custom_card',
     $q$select app.update_custom_card('11111111-1111-1111-1111-111111111111', 'custom-mine', 1, '{}', '', null, now())$q$,
     'update_custom_card']];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R915008): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  if total <> 2 then
    raise exception 'FAIL (R915008): % custom_cards rows to measure against, expected 2 (one per profile)', total;
  end if;

  select count(*) into seen from public.custom_cards;
  if seen <> 1 or exists (select 1 from public.custom_cards where profile_id <> caller) then
    raise exception 'FAIL (R915008): profile 1 saw % of % custom_cards rows — another profile''s card leaked',
      seen, total;
  end if;
  if (select source from public.custom_cards) <> 'return Card {}' then
    raise exception 'FAIL (R915008): profile 1''s own card reads back as %', (select row_to_json(c) from public.custom_cards c);
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R915008): % succeeded as a client — custom cards are written by the server alone', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%' || probes[i][3] || '%' then
          raise exception 'FAIL (R915008): % was refused by "%" (%), which does not name %',
            probe, sqlerrm, sqlstate, probes[i][3];
        end if;
        raise notice 'OK (R915008): % refused — % (%)', probe, sqlerrm, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R915008): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;

  raise notice 'OK (R915008): profile 1 sees its own card and not profile 2''s, and 3 writes and both functions were refused';
end $$;

select set_config('request.jwt.claim.sub', '22222222-2222-2222-2222-222222222222', true);
do $$
declare
  seen bigint;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R915008): running as %, not authenticated', current_user;
  end if;
  select count(*) into seen from public.custom_cards;
  if seen <> 1 or exists (
    select 1 from public.custom_cards where profile_id <> '22222222-2222-2222-2222-222222222222'
  ) then
    raise exception 'FAIL (R915008): profile 2 saw % custom_cards rows, expected only its own', seen;
  end if;
  raise notice 'OK (R915008): profile 2 sees only its own card';
end $$;

-- anon reads nothing at all: no grant.
set local role anon;
do $$
begin
  begin
    perform count(*) from public.custom_cards;
    raise exception 'FAIL (R915008): anon read public.custom_cards';
  exception
    when insufficient_privilege then
      raise notice 'OK (R915008): anon is refused — % (%)', sqlerrm, sqlstate;
  end;
end $$;

reset role;
rollback;

\echo '### R915008: deleting an account deletes its custom cards ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('b9150080-0000-4000-8000-000000000002', 'gone915008@example.test', now());
do $$
declare
  gone  constant uuid := 'b9150080-0000-4000-8000-000000000002';
  other constant uuid := '11111111-1111-1111-1111-111111111111';
  n int;
begin
  insert into public.custom_cards (id, profile_id, frame) values
    ('custom-gone-1', gone, '{}'), ('custom-gone-2', gone, '{}'), ('custom-kept', other, '{}');
  select count(*) into n from public.custom_cards where profile_id = gone;
  if n <> 2 then
    raise exception 'FAIL (R915008): % cards seeded for the account to delete, not 2; the check below would prove nothing', n;
  end if;
  delete from auth.users where id = gone;
  select count(*) into n from public.custom_cards where profile_id = gone;
  if n <> 0 then
    raise exception 'FAIL (R915008): % custom cards outlived their account', n;
  end if;
  if not exists (select 1 from public.custom_cards where id = 'custom-kept') then
    raise exception 'FAIL (R915008): deleting one account deleted another''s custom card';
  end if;
  raise notice 'OK (R915008): an account''s custom cards go with it, and no one else''s do';
end $$;
rollback;
