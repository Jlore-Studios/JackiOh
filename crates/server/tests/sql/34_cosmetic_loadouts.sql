-- Per-card picks on the account and the loadouts a match freezes (migration 0061, SPEC §14.2, §9.1;
-- R1834000, R1834002, R1834005; issue #1834). Runs after 33_game_record_batches.sql; profiles 1 and 2
-- (11111111-…, 22222222-…) are active by then (03 activated them), and core-001 is in public.cards
-- (01's CHECK 10).
\set ON_ERROR_STOP on

-- Same rules as 01-33: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate (and, for a privilege, on the table it names), each
-- check that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R1834000: public.card_cosmetic_picks keys one row per account and card, follows its account, and names no registry row ###'
do $$
declare
  v_text text;
begin
  -- RLS on (CHECK 1 holds every table to it; this names the table so its absence is caught here too).
  if not exists (select 1 from pg_class
                  where oid = 'public.card_cosmetic_picks'::regclass and relrowsecurity) then
    raise exception 'FAIL (R1834000): public.card_cosmetic_picks has no row level security';
  end if;

  -- The columns, as the store reads and writes them: 0061's seven, then 0072's full_art_id
  -- (R2655000, 40_full_art_tiers.sql).
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'card_cosmetic_picks';
  if v_text is distinct from 'profile_id uuid NO, card_id text NO, foil text YES, finish text YES, '
                             'border text YES, full_art boolean YES, updated_at timestamp with time zone NO, '
                             'full_art_id text YES' then
    raise exception 'FAIL (R1834000): public.card_cosmetic_picks has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- One row per account and card.
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.card_cosmetic_picks'::regclass and c.contype = 'p';
  if v_text is distinct from 'profile_id, card_id' then
    raise exception 'FAIL (R1834000): the primary key is (%), expected (profile_id, card_id)', coalesce(v_text, 'none');
  end if;

  -- Its one foreign key is its account's, cascading; no id is held to public.cosmetics or public.cards,
  -- so a newer client's id is kept as written (the server judges ownership where it is worn).
  select string_agg(format('%s %s', c.confrelid::regclass, c.confdeltype), ', ') into v_text
    from pg_constraint c
   where c.conrelid = 'public.card_cosmetic_picks'::regclass and c.contype = 'f';
  if v_text is distinct from 'profiles c' then
    raise exception 'FAIL (R1834000): the foreign keys are [%], expected the profile''s alone, on delete cascade',
      coalesce(v_text, '(none)');
  end if;

  raise notice 'OK (R1834000): card_cosmetic_picks has RLS on, its columns, one row per (profile_id, card_id) and one foreign key, its account''s, cascading';
end $$;

\echo '### R1834000, §9.1: authenticated reads its own card picks through one select policy and writes none; anon reaches nothing ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  select string_agg(format('%s %s %s %s', policyname, cmd, array_to_string(roles, ','), qual), '; ')
    into v_text
    from pg_policies
   where schemaname = 'public' and tablename = 'card_cosmetic_picks';
  if v_text is distinct from 'card_cosmetic_picks_select_own SELECT authenticated (profile_id = app.current_profile_id())' then
    raise exception 'FAIL (R1834000): the policies are [%], expected one select-own policy to authenticated',
      coalesce(v_text, '(none)');
  end if;

  foreach who in array array['authenticated', 'anon'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.card_cosmetic_picks', op) <> (who = 'authenticated' and op = 'SELECT') then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R1834000): the client privileges on card_cosmetic_picks are wrong for [%]',
      array_to_string(wrong, ', ');
  end if;
  -- What the server's role may do is Supabase's grant, which this suite's stub leaves out
  -- (tests/db/bootstrap.sql makes it, and tests/store/contract.rs writes the table as the server).
  raise notice 'OK (R1834000): one select-own policy to authenticated, SELECT its only client privilege, anon none';
end $$;

\echo '### R1834000: the checks refuse a card or a pick that is no slug of at most 40 characters, and a row that picks nothing ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, statement]. Each breaks one of 0061's checks and nothing else.
  refused constant text[][] := array[
    ['a card id that is no slug',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, foil) values (%L, 'Core-001', 'foil-rainbow')$q$, p1)],
    ['a card id of 41 characters',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, foil) values (%L, %L, 'foil-rainbow')$q$,
            p1, 'c' || repeat('a', 40))],
    ['a foil that is no slug',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, foil) values (%L, 'core-001', 'Foil Rainbow')$q$, p1)],
    ['a finish that starts with a digit',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, finish) values (%L, 'core-001', '9-plain')$q$, p1)],
    ['a border of 41 characters',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, border) values (%L, 'core-001', %L)$q$,
            p1, 'b' || repeat('x', 40))],
    ['an empty foil',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id, foil) values (%L, 'core-001', '')$q$, p1)],
    ['a row that picks nothing',
     format($q$insert into public.card_cosmetic_picks (profile_id, card_id) values (%L, 'core-001')$q$, p1)]];
  i int;
begin
  if not exists (select 1 from public.profiles where id = p1) then
    raise exception 'FAIL (R1834000): profile 1 is missing, so every insert below would fail on its foreign key';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      execute refused[i][2];
      raise exception 'FAIL (R1834000): the schema took %', refused[i][1];
    exception
      when check_violation then null;
    end;
  end loop;

  -- The vacuity guard: what the store writes is taken, at the bounds: ids of 40 characters, a card
  -- the catalog lacks and a cosmetic the registry lacks (a newer client's), and a row whose only pick
  -- is its full-art switch, off.
  insert into public.card_cosmetic_picks (profile_id, card_id, foil, finish, border)
  values (p1, 'c' || repeat('a', 39), 'foil-from-a-newer-client', 'f' || repeat('9', 39), 'border-wood');
  insert into public.card_cosmetic_picks (profile_id, card_id, full_art) values (p1, 'core-001', false);
  if (select count(*) from public.card_cosmetic_picks where profile_id = p1) <> 2 then
    raise exception 'FAIL (R1834000): the two rows the checks allow were not both written';
  end if;
  if (select updated_at from public.card_cosmetic_picks where profile_id = p1 and card_id = 'core-001') is null then
    raise exception 'FAIL (R1834000): updated_at has no default';
  end if;
  -- One row per account and card.
  begin
    insert into public.card_cosmetic_picks (profile_id, card_id, foil) values (p1, 'core-001', 'foil-rainbow');
    raise exception 'FAIL (R1834000): a second row for one account and card was taken';
  exception
    when unique_violation then null;
  end;
  raise notice 'OK (R1834000): % rows the checks refuse are refused, a second row for a card too, and the rows at the bounds are taken',
    array_length(refused, 1);
end $$;
rollback;

\echo '### R1834000: an account reads only its own card picks and writes none of them ###'
begin;

-- Rows for EACH profile, as the owner: profile 2's are the rows to hide from profile 1, and
-- profile 1's from profile 2. Raw inserts, as superuser, because the point is what RLS shows.
insert into public.card_cosmetic_picks (profile_id, card_id, foil, full_art) values
  ('11111111-1111-1111-1111-111111111111', 'core-001', 'foil-rainbow', null),
  ('11111111-1111-1111-1111-111111111111', 'core-002', null, false),
  ('22222222-2222-2222-2222-222222222222', 'core-001', 'foil-holo-sweep', true);

-- Preflight: every write the client is about to be refused is one the owner can make, inside a
-- subtransaction rolled back at once.
do $$
begin
  begin
    insert into public.card_cosmetic_picks (profile_id, card_id, border)
    values ('11111111-1111-1111-1111-111111111111', 'core-003', 'border-wood');
    update public.card_cosmetic_picks set border = 'border-bone';
    delete from public.card_cosmetic_picks;
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception
        'FAIL (R1834000): the owner could not run the writes the client is about to be refused ("%", %) — the refusals below would prove nothing about privileges',
        sqlerrm, sqlstate;
    end if;
  end;
  perform set_config('r1834000.total', (select count(*) from public.card_cosmetic_picks)::text, true);
end $$;

set local role authenticated;
do $$ begin perform set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true); end $$;

do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  total  bigint := coalesce(nullif(current_setting('r1834000.total', true), ''), '-1')::bigint;
  seen   bigint;
  probe  text;
  probes constant text[][] := array[
    ['card_cosmetic_picks INSERT',
     $q$insert into public.card_cosmetic_picks (profile_id, card_id, border)
        values ('11111111-1111-1111-1111-111111111111', 'core-003', 'border-wood')$q$],
    ['card_cosmetic_picks UPSERT',
     $q$insert into public.card_cosmetic_picks (profile_id, card_id, foil)
        values ('11111111-1111-1111-1111-111111111111', 'core-001', 'foil-cracked-ice')
        on conflict (profile_id, card_id) do update set foil = excluded.foil$q$],
    ['card_cosmetic_picks UPDATE', $q$update public.card_cosmetic_picks set foil = 'foil-cracked-ice'$q$],
    ['card_cosmetic_picks DELETE', $q$delete from public.card_cosmetic_picks$q$]];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R1834000): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  if total <> 3 then
    raise exception 'FAIL (R1834000): % card_cosmetic_picks rows to measure against, expected 3 (two of profile 1, one of profile 2)', total;
  end if;

  select count(*) into seen from public.card_cosmetic_picks;
  if seen <> 2 or exists (select 1 from public.card_cosmetic_picks where profile_id <> caller) then
    raise exception 'FAIL (R1834000): profile 1 saw % of % card_cosmetic_picks rows — another account''s picks leaked',
      seen, total;
  end if;
  if (select foil from public.card_cosmetic_picks where card_id = 'core-001') is distinct from 'foil-rainbow' then
    raise exception 'FAIL (R1834000): profile 1''s own core-001 row reads back as %',
      (select row_to_json(p) from public.card_cosmetic_picks p where card_id = 'core-001');
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R1834000): % succeeded as a client — card picks are written by the server alone', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%card_cosmetic_picks%' then
          raise exception 'FAIL (R1834000): % was refused by "%" (%), which does not name card_cosmetic_picks',
            probe, sqlerrm, sqlstate;
        end if;
        raise notice 'OK (R1834000): % refused — % (%)', probe, sqlerrm, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R1834000): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;

  raise notice 'OK (R1834000): profile 1 sees its own two card picks and not profile 2''s, and % writes were refused',
    array_length(probes, 1);
end $$;

do $$ begin perform set_config('request.jwt.claim.sub', '22222222-2222-2222-2222-222222222222', true); end $$;
do $$
declare
  seen bigint;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R1834000): running as %, not authenticated', current_user;
  end if;
  select count(*) into seen from public.card_cosmetic_picks;
  if seen <> 1 or exists (
    select 1 from public.card_cosmetic_picks where profile_id <> '22222222-2222-2222-2222-222222222222'
  ) then
    raise exception 'FAIL (R1834000): profile 2 saw % card_cosmetic_picks rows, expected only its own one', seen;
  end if;
  raise notice 'OK (R1834000): profile 2 sees only its own card pick';
end $$;

reset role;
set local role anon;
do $$
begin
  if current_user <> 'anon' then
    raise exception 'FAIL (R1834000): running as %, not anon', current_user;
  end if;
  begin
    perform count(*) from public.card_cosmetic_picks;
    raise exception 'FAIL (R1834000): anon read card_cosmetic_picks';
  exception
    when insufficient_privilege then null;
  end;
  raise notice 'OK (R1834000): anon reads no card picks';
end $$;

reset role;
rollback;

\echo '### 0012''s rule: a deleted account takes its card picks with it, and nobody else''s ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e1834000-0000-4000-8000-000000000001', 'r1834000-leaver@example.test', now());
do $$
declare
  gone constant uuid := 'e1834000-0000-4000-8000-000000000001';
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R1834000): the leaver has no profile; the cascade below would prove nothing';
  end if;
  insert into public.card_cosmetic_picks (profile_id, card_id, foil) values (gone, 'core-001', 'foil-rainbow'),
                                                                             (gone, 'core-002', 'foil-rainbow'),
                                                                             (p1, 'core-001', 'foil-rainbow');

  delete from auth.users where id = gone;

  if exists (select 1 from public.card_cosmetic_picks where profile_id = gone) then
    raise exception 'FAIL (R1834000): a deleted account''s card picks outlived it';
  end if;
  if not exists (select 1 from public.card_cosmetic_picks where profile_id = p1) then
    raise exception 'FAIL (R1834000): deleting one account took another''s card picks';
  end if;
  raise notice 'OK (R1834000): deleting an account deletes its card picks and no other account''s';
end $$;
rollback;

\echo '### R1834002, R1834005: the frozen slots and a bot''s cosmetics are a jsonb object or null, the loadouts an array of two or null ###'
begin;
-- A row of each table to write the columns of, as the owner.
insert into public.tickets (id, profile_id, rating, frozen_deck, catalog_version, status)
values ('e1834002-0000-4000-8000-000000000001', '11111111-1111-1111-1111-111111111111', 1000, '[]', 'core-1', 'queued');
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, started_at, ceiling_at)
values ('e1834002-0000-4000-8000-000000000002', 'live', 'seed-1834',
        '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour');
insert into public.matches (id, room_code, status, seed, p1_profile_id, p1_deck, catalog_version, ceiling_at)
values ('e1834002-0000-4000-8000-000000000003', 'SLTXYZ', 'open', '', '11111111-1111-1111-1111-111111111111',
        '[]', 'core-1', now() + interval '1 hour');
insert into public.profiles (id, kind, status) values ('e1834005-0000-4000-8000-000000000001', 'bot', 'active');
insert into public.bots (profile_id, strength, speed, persona, active_from_hour, active_hours)
values ('e1834005-0000-4000-8000-000000000001', 2, 1.0, 'balanced', 8, 12);

do $$
declare
  ticket constant uuid := 'e1834002-0000-4000-8000-000000000001';
  live   constant uuid := 'e1834002-0000-4000-8000-000000000002';
  room   constant uuid := 'e1834002-0000-4000-8000-000000000003';
  bot    constant uuid := 'e1834005-0000-4000-8000-000000000001';
  -- [table, column, row id column, row id]
  shaped constant text[][] := array[
    ['tickets', 'deck_cosmetics', 'id', ticket::text],
    ['matches', 'room_deck_cosmetics', 'id', room::text],
    ['matches', 'loadouts', 'id', live::text],
    ['bots', 'cosmetics', 'profile_id', bot::text]];
  -- [what, table, column, row id column, row id, value] the checks refuse.
  refused constant text[][] := array[
    ['a ticket''s slots as an array', 'tickets', 'deck_cosmetics', 'id', ticket::text, '[]'],
    ['a ticket''s slots as a bare id', 'tickets', 'deck_cosmetics', 'id', ticket::text, '"table-felt"'],
    ['a room''s slots as an array', 'matches', 'room_deck_cosmetics', 'id', room::text, '["table-felt"]'],
    ['a json null for a room''s slots', 'matches', 'room_deck_cosmetics', 'id', room::text, 'null'],
    ['the loadouts as an object', 'matches', 'loadouts', 'id', live::text, '{"p1": {}, "p2": {}}'],
    ['one seat''s loadout', 'matches', 'loadouts', 'id', live::text, '[{}]'],
    ['three seats'' loadouts', 'matches', 'loadouts', 'id', live::text, '[{}, {}, {}]'],
    ['no seat''s loadout', 'matches', 'loadouts', 'id', live::text, '[]'],
    ['a bot''s cosmetics as an array', 'bots', 'cosmetics', 'profile_id', bot::text, '[]'],
    ['a bot''s cosmetics as a number', 'bots', 'cosmetics', 'profile_id', bot::text, '7']];
  -- [table, column, row id column, row id, value] the checks take: the shapes the store writes.
  taken constant text[][] := array[
    ['tickets', 'deck_cosmetics', 'id', ticket::text, '{"table": "table-felt", "cardBack": "card-back-lacquer"}'],
    ['tickets', 'deck_cosmetics', 'id', ticket::text, '{}'],
    ['matches', 'room_deck_cosmetics', 'id', room::text, '{"field": "field-rain"}'],
    ['matches', 'loadouts', 'id', live::text,
     '[{"seat": {"table": "table-felt"}, "cards": []}, {"seat": {"table": "table-tavern"}, "cards": [{"cardId": "core-001", "fullArt": true}]}]'],
    ['bots', 'cosmetics', 'profile_id', bot::text,
     '{"wardrobe": {"cards": [], "account": []}, "account": {"picks": {}, "fullArt": true}, "cards": [], "drawnAt": 0}']];
  col record;
  i   int;
  n   bigint;
begin
  -- Each column: jsonb, nullable, no default, and NULL on the rows written without it (every row
  -- from before 0061, which every reader takes as none, or all defaults).
  for i in 1 .. array_length(shaped, 1) loop
    select data_type, is_nullable, column_default into col
      from information_schema.columns
     where table_schema = 'public' and table_name = shaped[i][1] and column_name = shaped[i][2];
    if not found or col.data_type <> 'jsonb' or col.is_nullable <> 'YES' or col.column_default is not null then
      raise exception 'FAIL (0061): public.%.% is not a nullable jsonb with no default: %', shaped[i][1], shaped[i][2], col;
    end if;
    execute format('select count(*) from public.%I where %I = %L and %I is null',
                   shaped[i][1], shaped[i][3], shaped[i][4], shaped[i][2]) into n;
    if n <> 1 then
      raise exception 'FAIL (0061): the % row written without public.%.% does not read it as NULL',
        shaped[i][1], shaped[i][1], shaped[i][2];
    end if;
  end loop;

  for i in 1 .. array_length(refused, 1) loop
    begin
      execute format('update public.%I set %I = %L::jsonb where %I = %L',
                     refused[i][2], refused[i][3], refused[i][6], refused[i][4], refused[i][5]);
      get diagnostics n = row_count;
      raise exception 'FAIL (0061): the schema took % (% rows)', refused[i][1], n;
    exception
      when check_violation then null;
    end;
  end loop;

  for i in 1 .. array_length(taken, 1) loop
    execute format('update public.%I set %I = %L::jsonb where %I = %L',
                   taken[i][1], taken[i][2], taken[i][5], taken[i][3], taken[i][4]);
    get diagnostics n = row_count;
    if n <> 1 then
      raise exception 'FAIL (0061): writing % to public.%.% touched % rows, so its refusals above touched none either',
        taken[i][5], taken[i][1], taken[i][2], n;
    end if;
    execute format('update public.%I set %I = null where %I = %L',
                   taken[i][1], taken[i][2], taken[i][3], taken[i][4]);
  end loop;
  raise notice 'OK (0061): the four columns are nullable jsonb, % shapes are refused and the % the store writes are taken',
    array_length(refused, 1), array_length(taken, 1);
end $$;
rollback;

\echo '### R1834000: 0061 re-applies over rows of its own and changes none ###'
begin;
insert into public.card_cosmetic_picks (profile_id, card_id, foil)
values ('11111111-1111-1111-1111-111111111111', 'core-001', 'foil-rainbow');
create temp table r1834000_rows (n bigint not null) on commit drop;
insert into r1834000_rows select count(*) from public.card_cosmetic_picks;
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0061_cosmetic_loadouts.sql
set local client_min_messages = notice;
do $$
begin
  if (select n from r1834000_rows) <> 1 then
    raise exception 'FAIL (R1834000): % card picks before the re-apply, expected the one written', (select n from r1834000_rows);
  end if;
  if (select count(*) from public.card_cosmetic_picks) <> (select n from r1834000_rows) then
    raise exception 'FAIL (R1834000): re-applying 0061 changed the card picks';
  end if;
  if (select count(*) from pg_policies where schemaname = 'public' and tablename = 'card_cosmetic_picks') <> 1 then
    raise exception 'FAIL (R1834000): re-applying 0061 left other than one policy on card_cosmetic_picks';
  end if;
  raise notice 'OK (R1834000): 0061 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
