-- The moderation levers' audit rows (migration 0096, SPEC §9.4, §14.10; issue #4141). Runs after
-- 57_notifications.sql; profile 1 (11111111-…) is active by then (03 activated it). Each lever of
-- `jackioh-server moderate` that changes something writes one public.admin_actions row with no profile,
-- named by one of four exact routes (src/admin/store.rs's MODERATE_ROUTES); 0096 widens 0080's
-- admin_actions_route_check by those four names and no other.
\set ON_ERROR_STOP on

-- Same rules as 01-57: every check raises on failure (`raise exception 'FAIL (…): …'`), every expected
-- refusal is matched on its sqlstate, each check that could pass vacuously carries a guard, and every
-- block that writes runs inside a transaction that is rolled back.

\echo '### Migration 0096: the audit takes the four lever routes with no profile and refuses every name near them ###'
begin;
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  levers  constant text[] := array['CLI moderate ban', 'CLI moderate unban', 'CLI moderate revoke-code',
                                   'CLI moderate redemption'];
  refused constant text[] := array['CLI moderate ban ', ' CLI moderate ban', 'cli moderate ban', 'CLI moderate',
                                   'CLI moderate ban extra', 'CLI moderate chat-reports',
                                   'CLI moderate redemption on', 'CLI mint-code', 'GET /api/news',
                                   'POST /api/devtools/x'];
  i     int;
  v_con text;
begin
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
      values (null, refused[i], '{}', '{}', '{}', now());
      raise exception 'FAIL (0096): the audit took the route %', quote_literal(refused[i]);
    exception
      when check_violation then
        get stacked diagnostics v_con = constraint_name;
        if v_con is distinct from 'admin_actions_route_check' then
          raise exception 'FAIL (0096): % was refused by %, not admin_actions_route_check',
            quote_literal(refused[i]), v_con;
        end if;
    end;
  end loop;

  -- The vacuity guard: each lever's route is taken with no profile, and a dev route still is.
  for i in 1 .. array_length(levers, 1) loop
    insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
    values (null, levers[i], '{}', '{}', '{}', now());
  end loop;
  insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
  values (p1, 'POST /api/dev/jlore', '{"jlore": 5}', '{"jlore": 0}', '{"jlore": 5}', now());
  if (select count(*) from public.admin_actions where route = any(levers) and profile_id is null) <> 4 then
    raise exception 'FAIL (0096): the four lever rows were not all written with no profile';
  end if;
  if not exists (select 1 from public.admin_actions where profile_id = p1 and route = 'POST /api/dev/jlore') then
    raise exception 'FAIL (0096): a dev route''s row was not written';
  end if;
  raise notice 'OK (0096): % near names refused by admin_actions_route_check, the four lever routes and a dev route taken',
    array_length(refused, 1);
end $$;
rollback;

\echo '### Migration 0096: it re-applies over a lever row of its own and changes none ###'
begin;
insert into public.admin_actions (profile_id, route, request, before_value, after_value, at)
values (null, 'CLI moderate ban', '{"profileId": "11111111-1111-1111-1111-111111111111"}',
        '{"status": "active"}', '{"status": "banned"}', now());
-- What run.sh's apply_migration filters, a re-run's notices, is quieted here.
set local client_min_messages = warning;
\i /tmp/0096_moderation_audit.sql
set local client_min_messages = notice;
do $$
begin
  if (select count(*) from public.admin_actions
       where route = 'CLI moderate ban' and after_value = '{"status": "banned"}'::jsonb) <> 1 then
    raise exception 'FAIL (0096): re-applying 0096 lost or changed the lever row';
  end if;
  if (select count(*) from pg_constraint
       where conrelid = 'public.admin_actions'::regclass and conname = 'admin_actions_route_check') <> 1 then
    raise exception 'FAIL (0096): re-applying 0096 left admin_actions_route_check other than once';
  end if;
  raise notice 'OK (0096): 0096 re-applies over its own row and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
