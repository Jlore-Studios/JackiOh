-- Game records carry their build (migration 0047, R1367000). Runs after 26_dev_accounts.sql.
\set ON_ERROR_STOP on

-- Same rules as 01-26: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, and every block runs inside a transaction that is
-- rolled back.

\echo '### 0047: the migration re-applies over itself ###'
begin;
set local client_min_messages = warning;
\i /tmp/0047_game_record_builds.sql
\i /tmp/0047_game_record_builds.sql
set local client_min_messages = notice;
do $$
begin
  if not exists (select 1 from pg_constraint where conname = 'game_records_build_check') then
    raise exception 'FAIL (0047): game_records_build_check is gone after a re-apply';
  end if;
  raise notice 'OK (0047): the migration re-applies over itself';
end $$;
rollback;

\echo '=== R1367000: a record holds a well-formed build, or none ==='
begin;
do $$
declare
  rec jsonb := jsonb_build_object(
    'id', 'dev:build:1', 'source', 'dev', 'mode', 'random', 'patch', 'v0.2.5',
    'pilots', jsonb_build_object('p1', 'ai', 'p2', 'ai'),
    'game', jsonb_build_object('first', 'p1', 'winner', 'p2', 'reason', 'hero-death', 'turns', 9,
      'seats', jsonb_build_object(
        'p1', jsonb_build_object('deck', '["core-001"]'::jsonb, 'opening', '[]'::jsonb, 'drawn', '[]'::jsonb, 'played', '[]'::jsonb),
        'p2', jsonb_build_object('deck', '["core-002"]'::jsonb, 'opening', '[]'::jsonb, 'drawn', '[]'::jsonb, 'played', '[]'::jsonb))));
  n int;
  bad jsonb;
  i int := 0;
begin
  -- A record written before builds were recorded, and one with each form of a build.
  insert into public.game_records (id, source, mode, patch, record)
  values ('dev:build:1', 'dev', 'random', 'v0.2.5', rec);
  insert into public.game_records (id, source, mode, patch, record)
  values ('dev:build:2', 'dev', 'random', 'v0.2.5',
          rec || '{"id":"dev:build:2","build":{"commit":"0123abc","aiGeneration":12}}');
  insert into public.game_records (id, source, mode, patch, record)
  values ('dev:build:3', 'dev', 'random', 'v0.2.5', rec || '{"id":"dev:build:3","build":{"commit":"0123abc"}}');
  select count(*) into n from public.game_records where id like 'dev:build:%';
  if n <> 3 then
    raise exception 'FAIL (0047): % of the 3 well-formed records were written', n;
  end if;

  foreach bad in array array[
    '"0123abc"'::jsonb,
    '{"aiGeneration":12}'::jsonb,
    '{"commit":""}'::jsonb,
    '{"commit":7}'::jsonb,
    '{"commit":"a","aiGeneration":-1}'::jsonb,
    '{"commit":"a","aiGeneration":1.5}'::jsonb,
    '{"commit":"a","aiGeneration":"12"}'::jsonb,
    '{"commit":"a","aiGeneration":"twelve"}'::jsonb
  ] loop
    i := i + 1;
    begin
      insert into public.game_records (id, source, mode, patch, record)
      values ('dev:bad:' || i, 'dev', 'random', 'v0.2.5',
              rec || jsonb_build_object('id', 'dev:bad:' || i, 'build', bad));
      raise exception 'FAIL (0047): a record with the build % was written', bad;
    exception
      when check_violation then null;
    end;
  end loop;
  raise notice 'OK (R1367000): a build is absent or a commit with an optional whole AI generation';
end $$;
rollback;
