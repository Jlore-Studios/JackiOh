-- A saved deck's earlier versions (migration 0065, SPEC §9.4, §9.1; R2100000; issue #2100). Runs after
-- 35_game_totals.sql; profiles 1 and 2 (11111111-…, 22222222-…) are active by then (03 activated them).
-- What a save keeps (the window, the cap, a restore) is the server's, proven in
-- tests/store/contract.rs against both stores; this file proves what the schema holds.
\set ON_ERROR_STOP on

-- Same rules as 01-35: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate (and, for a privilege, on the table it names), each
-- check that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R2100000: public.deck_versions has RLS on, its columns, one row per version and one foreign key, its deck''s, cascading ###'
do $$
declare
  v_text text;
begin
  -- RLS on (CHECK 1 holds every table to it; this names the table so its absence is caught here too).
  if not exists (select 1 from pg_class
                  where oid = 'public.deck_versions'::regclass and relrowsecurity) then
    raise exception 'FAIL (R2100000): public.deck_versions has no row level security';
  end if;

  -- The columns, as the store reads and writes them.
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'deck_versions';
  if v_text is distinct from 'id uuid NO, profile_id uuid NO, deck_id uuid NO, name text NO, cards jsonb NO, '
                             'saved_at timestamp with time zone NO' then
    raise exception 'FAIL (R2100000): public.deck_versions has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- One row per version, keyed by the id the server mints.
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.deck_versions'::regclass and c.contype = 'p';
  if v_text is distinct from 'id' then
    raise exception 'FAIL (R2100000): the primary key is (%), expected (id)', coalesce(v_text, 'none');
  end if;

  -- Its one foreign key is its deck's, on (profile_id, deck_id), cascading: a version names only a
  -- deck of its own profile, and goes with it.
  select string_agg(format('%s %s', c.confrelid::regclass, c.confdeltype), ', ') into v_text
    from pg_constraint c
   where c.conrelid = 'public.deck_versions'::regclass and c.contype = 'f';
  if v_text is distinct from 'decks c' then
    raise exception 'FAIL (R2100000): the foreign keys are [%], expected the deck''s alone, on delete cascade',
      coalesce(v_text, '(none)');
  end if;
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.deck_versions'::regclass and c.contype = 'f';
  if v_text is distinct from 'profile_id, deck_id' then
    raise exception 'FAIL (R2100000): the foreign key is on (%), expected (profile_id, deck_id)', coalesce(v_text, 'none');
  end if;

  -- The read path, a deck's versions newest first.
  select pg_get_indexdef(i.indexrelid) into v_text
    from pg_index i
   where i.indrelid = 'public.deck_versions'::regclass
     and i.indexrelid = 'public.deck_versions_deck_saved_idx'::regclass;
  if v_text is null or v_text not like '%(profile_id, deck_id, saved_at DESC, id DESC)' then
    raise exception 'FAIL (R2100000): deck_versions_deck_saved_idx is [%], expected (profile_id, deck_id, saved_at desc, id desc)',
      coalesce(v_text, '(none)');
  end if;

  raise notice 'OK (R2100000): deck_versions has RLS on, its columns, one row per id, one foreign key, its deck''s, cascading, and its newest-first index';
end $$;

\echo '### R2100000, §9.1: authenticated reads its own deck versions through one select policy and writes none; anon reaches nothing ###'
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
   where schemaname = 'public' and tablename = 'deck_versions';
  if v_text is distinct from 'deck_versions_select_own SELECT authenticated (profile_id = app.current_profile_id())' then
    raise exception 'FAIL (R2100000): the policies are [%], expected one select-own policy to authenticated',
      coalesce(v_text, '(none)');
  end if;

  foreach who in array array['authenticated', 'anon'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.deck_versions', op) <> (who = 'authenticated' and op = 'SELECT') then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (R2100000): the client privileges on deck_versions are wrong for [%]',
      array_to_string(wrong, ', ');
  end if;
  -- What the server's role may do is Supabase's grant, which this suite's stub leaves out
  -- (tests/db/bootstrap.sql makes it, and tests/store/contract.rs writes the table as the server).
  raise notice 'OK (R2100000): one select-own policy to authenticated, SELECT its only client privilege, anon none';
end $$;

\echo '### R2100000: the schema refuses cards that are no array and a version of a deck that is not its profile''s ###'
begin;
insert into public.decks (id, profile_id, name, cards, catalog_version) values
  ('e2100000-0000-4000-8000-000000000001', '11111111-1111-1111-1111-111111111111', 'Mine', '["core-001"]', 'core-1');
do $$
declare
  p1   constant uuid := '11111111-1111-1111-1111-111111111111';
  p2   constant uuid := '22222222-2222-2222-2222-222222222222';
  deck constant uuid := 'e2100000-0000-4000-8000-000000000001';
begin
  if not exists (select 1 from public.profiles where id = p2) then
    raise exception 'FAIL (R2100000): profile 2 is missing, so the foreign key below would refuse it for the wrong reason';
  end if;
  begin
    insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
    values ('e2100000-0000-4000-8000-000000000101', p1, deck, 'Mine', '{"core-001": 1}', now());
    raise exception 'FAIL (R2100000): the schema took cards that are an object, not an array';
  exception
    when check_violation then null;
  end;
  -- Profile 2's version of profile 1's deck: the composite key names no deck of profile 2's.
  begin
    insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
    values ('e2100000-0000-4000-8000-000000000102', p2, deck, 'Mine', '["core-001"]', now());
    raise exception 'FAIL (R2100000): the schema took profile 2''s version of profile 1''s deck';
  exception
    when foreign_key_violation then null;
  end;
  begin
    insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
    values ('e2100000-0000-4000-8000-000000000103', p1, 'e2100000-0000-4000-8000-0000000000ff', 'Gone', '[]', now());
    raise exception 'FAIL (R2100000): the schema took a version of a deck that does not exist';
  exception
    when foreign_key_violation then null;
  end;

  -- The vacuity guard: what the store writes is taken.
  insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
  values ('e2100000-0000-4000-8000-000000000104', p1, deck, 'Mine', '["core-001"]', now());
  if (select count(*) from public.deck_versions where deck_id = deck) <> 1 then
    raise exception 'FAIL (R2100000): the version the checks allow was not written';
  end if;
  raise notice 'OK (R2100000): cards that are no array and a version of another profile''s or no deck are refused, and the store''s row is taken';
end $$;
rollback;

\echo '### R2100000: an account reads only its own deck versions and writes none of them ###'
begin;

-- Rows for EACH profile, as the owner: profile 2's are the rows to hide from profile 1, and
-- profile 1's from profile 2. Raw inserts, as superuser, because the point is what RLS shows.
insert into public.decks (id, profile_id, name, cards, catalog_version) values
  ('e2100000-0000-4000-8000-000000000011', '11111111-1111-1111-1111-111111111111', 'Mine', '["core-001"]', 'core-1'),
  ('e2100000-0000-4000-8000-000000000012', '22222222-2222-2222-2222-222222222222', 'Theirs', '["core-002"]', 'core-1');
insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at) values
  ('e2100000-0000-4000-8000-000000000111', '11111111-1111-1111-1111-111111111111',
   'e2100000-0000-4000-8000-000000000011', 'Mine', '["core-001"]', now() - interval '1 hour'),
  ('e2100000-0000-4000-8000-000000000112', '11111111-1111-1111-1111-111111111111',
   'e2100000-0000-4000-8000-000000000011', 'Mine v2', '["core-001", "core-002"]', now()),
  ('e2100000-0000-4000-8000-000000000113', '22222222-2222-2222-2222-222222222222',
   'e2100000-0000-4000-8000-000000000012', 'Theirs', '["core-002"]', now());

-- Preflight: every write the client is about to be refused is one the owner can make, inside a
-- subtransaction rolled back at once.
do $$
begin
  begin
    insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
    values ('e2100000-0000-4000-8000-000000000114', '11111111-1111-1111-1111-111111111111',
            'e2100000-0000-4000-8000-000000000011', 'Mine v3', '[]', now());
    update public.deck_versions set name = 'Renamed';
    delete from public.deck_versions;
    raise exception 'owner-control-rollback';
  exception when others then
    if sqlerrm <> 'owner-control-rollback' then
      raise exception
        'FAIL (R2100000): the owner could not run the writes the client is about to be refused ("%", %) — the refusals below would prove nothing about privileges',
        sqlerrm, sqlstate;
    end if;
  end;
  perform set_config('r2100000.total', (select count(*) from public.deck_versions
                                         where id::text like 'e2100000-%')::text, true);
end $$;

set local role authenticated;
do $$ begin perform set_config('request.jwt.claim.sub', '11111111-1111-1111-1111-111111111111', true); end $$;

do $$
declare
  caller constant uuid := '11111111-1111-1111-1111-111111111111';
  total  bigint := coalesce(nullif(current_setting('r2100000.total', true), ''), '-1')::bigint;
  seen   bigint;
  probe  text;
  probes constant text[][] := array[
    ['deck_versions INSERT',
     $q$insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
        values ('e2100000-0000-4000-8000-000000000115', '11111111-1111-1111-1111-111111111111',
                'e2100000-0000-4000-8000-000000000011', 'Mine v3', '[]', now())$q$],
    ['deck_versions UPDATE', $q$update public.deck_versions set name = 'Renamed'$q$],
    ['deck_versions DELETE', $q$delete from public.deck_versions$q$]];
  i int;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R2100000): running as %, not authenticated — SET LOCAL did not take', current_user;
  end if;
  if total <> 3 then
    raise exception 'FAIL (R2100000): % deck_versions rows to measure against, expected 3 (two of profile 1, one of profile 2)', total;
  end if;

  select count(*) into seen from public.deck_versions where id::text like 'e2100000-%';
  if seen <> 2 or exists (select 1 from public.deck_versions where profile_id <> caller) then
    raise exception 'FAIL (R2100000): profile 1 saw % of % deck_versions rows — another account''s versions leaked',
      seen, total;
  end if;
  if (select name from public.deck_versions where id = 'e2100000-0000-4000-8000-000000000112') is distinct from 'Mine v2' then
    raise exception 'FAIL (R2100000): profile 1''s own newest version reads back as %',
      (select row_to_json(v) from public.deck_versions v where id = 'e2100000-0000-4000-8000-000000000112');
  end if;

  for i in 1 .. array_length(probes, 1) loop
    probe := probes[i][1];
    begin
      execute probes[i][2];
      raise exception 'FAIL (R2100000): % succeeded as a client — deck versions are written by the server alone', probe;
    exception
      when insufficient_privilege then
        if sqlerrm not like '%deck_versions%' then
          raise exception 'FAIL (R2100000): % was refused by "%" (%), which does not name deck_versions',
            probe, sqlerrm, sqlstate;
        end if;
        raise notice 'OK (R2100000): % refused — % (%)', probe, sqlerrm, sqlstate;
      when others then
        if sqlerrm like 'FAIL%' then raise; end if;
        raise exception 'FAIL (R2100000): % raised "%" (%), not insufficient_privilege', probe, sqlerrm, sqlstate;
    end;
  end loop;

  raise notice 'OK (R2100000): profile 1 sees its own two deck versions and not profile 2''s, and % writes were refused',
    array_length(probes, 1);
end $$;

do $$ begin perform set_config('request.jwt.claim.sub', '22222222-2222-2222-2222-222222222222', true); end $$;
do $$
declare
  seen bigint;
begin
  if current_user <> 'authenticated' then
    raise exception 'FAIL (R2100000): running as %, not authenticated', current_user;
  end if;
  select count(*) into seen from public.deck_versions where id::text like 'e2100000-%';
  if seen <> 1 or exists (
    select 1 from public.deck_versions where profile_id <> '22222222-2222-2222-2222-222222222222'
  ) then
    raise exception 'FAIL (R2100000): profile 2 saw % of the deck_versions rows, expected only its own one', seen;
  end if;
  raise notice 'OK (R2100000): profile 2 sees only its own deck version';
end $$;

reset role;
set local role anon;
do $$
begin
  if current_user <> 'anon' then
    raise exception 'FAIL (R2100000): running as %, not anon', current_user;
  end if;
  begin
    perform count(*) from public.deck_versions;
    raise exception 'FAIL (R2100000): anon read deck_versions';
  exception
    when insufficient_privilege then null;
  end;
  raise notice 'OK (R2100000): anon reads no deck versions';
end $$;

reset role;
rollback;

\echo '### R2100000: deleting a deck takes its versions, and deleting an account takes the rest, and nobody else''s ###'
begin;
insert into auth.users (id, email, email_confirmed_at)
values ('e2100000-0000-4000-8000-000000000201', 'r2100000-leaver@example.test', now());
do $$
declare
  gone    constant uuid := 'e2100000-0000-4000-8000-000000000201';
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  dropped constant uuid := 'e2100000-0000-4000-8000-000000000021';
  kept    constant uuid := 'e2100000-0000-4000-8000-000000000022';
  mine    constant uuid := 'e2100000-0000-4000-8000-000000000023';
begin
  if not exists (select 1 from public.profiles where id = gone) then
    raise exception 'FAIL (R2100000): the leaver has no profile; the cascade below would prove nothing';
  end if;
  insert into public.decks (id, profile_id, name, cards, catalog_version) values
    (dropped, gone, 'Dropped', '[]', 'core-1'),
    (kept, gone, 'Kept', '[]', 'core-1'),
    (mine, p1, 'Mine', '[]', 'core-1');
  insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at) values
    ('e2100000-0000-4000-8000-000000000221', gone, dropped, 'Dropped', '[]', now() - interval '1 hour'),
    ('e2100000-0000-4000-8000-000000000222', gone, dropped, 'Dropped', '["core-001"]', now()),
    ('e2100000-0000-4000-8000-000000000223', gone, kept, 'Kept', '[]', now()),
    ('e2100000-0000-4000-8000-000000000224', p1, mine, 'Mine', '[]', now());

  delete from public.decks where id = dropped;
  if exists (select 1 from public.deck_versions where deck_id = dropped) then
    raise exception 'FAIL (R2100000): a deleted deck''s versions outlived it';
  end if;
  if (select count(*) from public.deck_versions where profile_id = gone) <> 1 then
    raise exception 'FAIL (R2100000): deleting one deck took another deck''s versions';
  end if;

  delete from auth.users where id = gone;
  if exists (select 1 from public.deck_versions where profile_id = gone) then
    raise exception 'FAIL (R2100000): a deleted account''s deck versions outlived it';
  end if;
  if not exists (select 1 from public.deck_versions where profile_id = p1 and deck_id = mine) then
    raise exception 'FAIL (R2100000): deleting one account took another''s deck versions';
  end if;
  raise notice 'OK (R2100000): deleting a deck deletes its versions, deleting an account deletes the rest, and no other account''s go';
end $$;
rollback;

\echo '### R2100000: 0065 gives a deck saved before it one version, itself as last saved, and re-applies adding none ###'
begin;
insert into public.decks (id, profile_id, name, cards, catalog_version, created_at, updated_at) values
  ('e2100000-0000-4000-8000-000000000031', '11111111-1111-1111-1111-111111111111', 'Old deck',
   '["core-002", "core-001"]', 'core-1', '2026-01-02 03:04:05.678+00', '2026-02-03 04:05:06.789+00'),
  ('e2100000-0000-4000-8000-000000000032', '11111111-1111-1111-1111-111111111111', 'Versioned',
   '["core-001"]', 'core-1', now(), now());
insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at) values
  ('e2100000-0000-4000-8000-000000000332', '11111111-1111-1111-1111-111111111111',
   'e2100000-0000-4000-8000-000000000032', 'Versioned', '["core-001"]', now());
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0065_deck_versions.sql
set local client_min_messages = notice;
create temp table r2100000_rows (n bigint not null) on commit drop;
insert into r2100000_rows select count(*) from public.deck_versions;
do $$
declare
  old       constant uuid := 'e2100000-0000-4000-8000-000000000031';
  versioned constant uuid := 'e2100000-0000-4000-8000-000000000032';
  v         record;
begin
  if (select count(*) from public.deck_versions where deck_id = old) <> 1 then
    raise exception 'FAIL (R2100000): the deck saved before 0065 has % versions, expected one',
      (select count(*) from public.deck_versions where deck_id = old);
  end if;
  select * into v from public.deck_versions where deck_id = old;
  if v.profile_id <> '11111111-1111-1111-1111-111111111111' or v.name <> 'Old deck'
     or v.cards <> '["core-002", "core-001"]'::jsonb or v.saved_at <> '2026-02-03 04:05:06.789+00'::timestamptz then
    raise exception 'FAIL (R2100000): the seeded version is %, expected the deck''s name, cards in order and updated_at',
      row_to_json(v);
  end if;
  if (select array_agg(id) from public.deck_versions where deck_id = versioned)
     is distinct from array['e2100000-0000-4000-8000-000000000332'::uuid] then
    raise exception 'FAIL (R2100000): a deck that had a version was given another';
  end if;
  if exists (select 1 from public.decks d
              where not exists (select 1 from public.deck_versions x
                                 where x.profile_id = d.profile_id and x.deck_id = d.id)) then
    raise exception 'FAIL (R2100000): a deck has no version after 0065';
  end if;
end $$;
set local client_min_messages = warning;
\i /tmp/0065_deck_versions.sql
set local client_min_messages = notice;
do $$
begin
  if (select count(*) from public.deck_versions) <> (select n from r2100000_rows) then
    raise exception 'FAIL (R2100000): re-applying 0065 changed the versions (% before, % after)',
      (select n from r2100000_rows), (select count(*) from public.deck_versions);
  end if;
  if (select count(*) from pg_policies where schemaname = 'public' and tablename = 'deck_versions') <> 1 then
    raise exception 'FAIL (R2100000): re-applying 0065 left other than one policy on deck_versions';
  end if;
  raise notice 'OK (R2100000): 0065 gives a deck with no version one, itself as last saved, leaves a versioned deck alone and re-applies adding none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
