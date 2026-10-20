-- Game records for the card statistics (migration 0014, SPEC §9.11, R376-R378). Runs after
-- 07_retention_purge.sql. The client's refusal on the table is 02_rls_as_client.sql's CHECK 3, with
-- the other server-only tables.
\set ON_ERROR_STOP on

-- Same rules as 01-07: every check raises on failure and runs inside a rolled-back transaction.

\echo '=== 0014 (R376): one record per game, its filter columns equal to the record ==='
begin;
do $$
declare
  rec jsonb := jsonb_build_object(
    'id', 'dev:check:1', 'source', 'dev', 'mode', 'random', 'patch', 'v0.2.5',
    'pilots', jsonb_build_object('p1', 'ai', 'p2', 'ai'),
    'game', jsonb_build_object('first', 'p1', 'winner', 'p2', 'reason', 'hero-death', 'turns', 9,
      'seats', jsonb_build_object(
        'p1', jsonb_build_object('deck', '["core-001"]'::jsonb, 'opening', '[]'::jsonb, 'drawn', '[]'::jsonb, 'played', '[]'::jsonb),
        'p2', jsonb_build_object('deck', '["core-002"]'::jsonb, 'opening', '[]'::jsonb, 'drawn', '[]'::jsonb, 'played', '[]'::jsonb))));
  n int;
begin
  insert into public.game_records (id, source, mode, patch, record)
  values ('dev:check:1', 'dev', 'random', 'v0.2.5', rec);

  -- The store's `on conflict (id) do nothing`: a second write of the same game writes nothing.
  insert into public.game_records (id, source, mode, patch, record)
  values ('dev:check:1', 'dev', 'random', 'v0.2.5', rec)
  on conflict (id) do nothing;
  select count(*) into n from public.game_records where id = 'dev:check:1';
  if n <> 1 then
    raise exception 'FAIL (0014): % rows for one game id', n;
  end if;

  -- R378: a record's source is live or dev. A column that disagrees with the record it files is
  -- refused, so a filter on the columns can never read a record as something it is not.
  begin
    insert into public.game_records (id, source, mode, patch, record)
    values ('dev:check:2', 'practice', 'random', 'v0.2.5', rec || '{"id":"dev:check:2","source":"practice"}');
    raise exception 'FAIL (0014): a record with the source practice was written';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_records (id, source, mode, patch, record)
    values ('dev:check:3', 'live', 'random', 'v0.2.5', rec || '{"id":"dev:check:3"}');
    raise exception 'FAIL (0014): a dev record was filed under the source live';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_records (id, source, mode, patch, record)
    values ('dev:check:4', 'dev', 'bo1', 'v0.2.5', rec || '{"id":"dev:check:4"}');
    raise exception 'FAIL (0014): a random record was filed under the mode bo1';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_records (id, source, mode, patch, record)
    values ('dev:check:5', 'dev', 'random', 'v0.1.1', rec || '{"id":"dev:check:5"}');
    raise exception 'FAIL (0014): a v0.2.5 record was filed under the patch v0.1.1';
  exception
    when check_violation then null;
  end;

  -- R378: a development record keeps to its own ids, and a live one stays out of them.
  begin
    insert into public.game_records (id, source, mode, patch, record)
    values ('00000000-0000-0000-0000-000000000014', 'dev', 'random', 'v0.2.5',
            rec || '{"id":"00000000-0000-0000-0000-000000000014"}');
    raise exception 'FAIL (0014): a dev record took a match id';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_records (id, source, mode, patch, record)
    values ('dev:check:6', 'live', 'random', 'v0.2.5', rec || '{"id":"dev:check:6","source":"live"}');
    raise exception 'FAIL (0014): a live record took a dev id';
  exception
    when check_violation then null;
  end;

  raise notice 'OK (0014): one record per id, source, mode and patch always equal to the record''s own, dev ids for dev records only';
end $$;
rollback;

\echo '### ALL GAME RECORD CHECKS RAN ###'
