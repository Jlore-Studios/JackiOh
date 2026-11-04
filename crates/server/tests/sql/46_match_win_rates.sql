-- A match keeps the win-rate snapshot it started with (migration 0080, SPEC §8.8 row 50; R3049000;
-- issue #3049). Runs after 45_card_versions.sql; profiles 1 and 2 (11111111-…, 22222222-…) exist by
-- then.
\set ON_ERROR_STOP on

-- Same rules as 01-45: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a
-- guard, and every block that writes runs inside a transaction that is rolled back. Each SPEC §11
-- row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R3049000: matches.win_rates is a jsonb object or null, null on a row written without it ###'
begin;
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, started_at, ceiling_at)
values ('e3049000-0000-4000-8000-000000000001', 'live', 'seed-3049',
        '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour');

do $$
declare
  live constant uuid := 'e3049000-0000-4000-8000-000000000001';
  -- [what, value] the check refuses.
  refused constant text[][] := array[
    ['the snapshot as an array', '[]'],
    ['the snapshot as a number', '7'],
    ['the snapshot as a string', '"v0.5.11o"'],
    ['a json null for the snapshot', 'null']];
  col record;
  i   int;
begin
  select data_type, is_nullable, column_default into col
    from information_schema.columns
   where table_schema = 'public' and table_name = 'matches' and column_name = 'win_rates';
  if not found then
    raise exception 'FAIL (R3049000): public.matches has no win_rates column';
  end if;
  if col.data_type <> 'jsonb' or col.is_nullable <> 'YES' or col.column_default is not null then
    raise exception 'FAIL (R3049000): matches.win_rates is % (nullable %, default %), expected nullable jsonb with no default',
      col.data_type, col.is_nullable, coalesce(col.column_default, '(none)');
  end if;
  if (select win_rates from public.matches where id = live) is not null then
    raise exception 'FAIL (R3049000): a match written without a snapshot holds one';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      execute format('update public.matches set win_rates = %L::jsonb where id = %L', refused[i][2], live);
      raise exception 'FAIL (R3049000): % was accepted', refused[i][1];
    exception when check_violation then
      null;
    end;
  end loop;
  update public.matches
     set win_rates = '{"patch": "v0.5.11o", "source": "live", "cards": [{"id": "core-001", "wins": 25, "games": 32}]}'
   where id = live;
  if (select win_rates -> 'cards' -> 0 ->> 'id' from public.matches where id = live) is distinct from 'core-001' then
    raise exception 'FAIL (R3049000): the snapshot did not read back as written';
  end if;
  raise notice 'OK (R3049000): matches.win_rates takes a snapshot object and refuses every other shape';
end $$;
rollback;

\echo '### R3049000: 0080 re-applies over its own rows and changes none ###'
begin;
insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                            catalog_version, started_at, ceiling_at, win_rates)
values ('e3049000-0000-4000-8000-000000000002', 'live', 'seed-3049-again',
        '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222',
        '[]', '[]', 'core-1', now(), now() + interval '1 hour', '{"patch": "v0.5.11o", "cards": []}');
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0080_match_win_rates.sql
set local client_min_messages = notice;
do $$
begin
  if (select win_rates ->> 'patch' from public.matches where id = 'e3049000-0000-4000-8000-000000000002')
     is distinct from 'v0.5.11o' then
    raise exception 'FAIL (R3049000): re-applying 0080 changed a match''s snapshot';
  end if;
  begin
    update public.matches set win_rates = '[]'
     where id = 'e3049000-0000-4000-8000-000000000002';
    raise exception 'FAIL (R3049000): after re-applying 0080, an array was accepted';
  exception when check_violation then
    null;
  end;
  raise notice 'OK (R3049000): 0080 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
