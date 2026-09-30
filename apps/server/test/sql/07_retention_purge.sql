-- The retention purge (migration 0013). Runs after 06_account_deletion.sql; profiles 1 and 2 are
-- active by then and play the matches here.
\set ON_ERROR_STOP on

-- Same rules as 01-06: every check raises on failure and runs inside a rolled-back transaction.

\echo '=== 0013: app.purge_expired_rows deletes old attempts and old logs, and nothing newer ==='
begin;
do $$
declare
  p1     constant uuid := '11111111-1111-1111-1111-111111111111';
  p2     constant uuid := '22222222-2222-2222-2222-222222222222';
  old_m  constant uuid := 'eeeeeeee-0000-0000-0000-000000000001';
  new_m  constant uuid := 'eeeeeeee-0000-0000-0000-000000000002';
  live_m constant uuid := 'eeeeeeee-0000-0000-0000-000000000003';
  cutoff constant timestamptz := now() - interval '90 days';
  v_row  record;
  n      int;
begin
  insert into public.code_attempts (profile_id, ip_hash, succeeded, at)
  values (p1, 'ip-0013-old', false, now() - interval '31 days'),
         (p1, 'ip-0013-new', false, now() - interval '29 days');

  -- A match that ended 91 days ago, one that ended 89 days ago, and a live one.
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (old_m, 'over', 's-old', p1, p2, '[]', '[]', 'core-1', now(), now(), now() - interval '91 days'),
         (new_m, 'over', 's-new', p1, p2, '[]', '[]', 'core-1', now(), now(), now() - interval '89 days'),
         (live_m, 'live', 's-live', p1, p2, '[]', '[]', 'core-1', now() + interval '1 hour', now(), null);
  insert into public.match_actions (match_id, seq, player_id, player_seat, nonce, action)
  values (old_m, 1, p1, 'p1', 'a', '{"type":"endTurn"}'), (old_m, 2, p2, 'p2', 'b', '{"type":"concede"}'),
         (new_m, 1, p1, 'p1', 'a', '{"type":"endTurn"}'),
         (live_m, 1, p1, 'p1', 'a', '{"type":"endTurn"}');
  insert into public.results (match_id, p1_profile_id, p2_profile_id, winner_profile_id, reason, turns,
                              p1_rating_before, p1_rating_after, p2_rating_before, p2_rating_after, ended_at)
  values (old_m, p1, p2, p1, 'concede', 2, 1000, 1016, 1000, 984, now() - interval '91 days');

  -- As service_role in `pnpm test:db` (the store's own call); here as the owner, since this
  -- database grants service_role no table privileges.
  select * into v_row from app.purge_expired_rows(now() - interval '30 days', cutoff);

  if v_row.match_actions <> 2 then
    raise exception 'FAIL (0013): the purge reports % actions, expected the old match''s 2', v_row.match_actions;
  end if;
  if v_row.code_attempts < 1 then
    raise exception 'FAIL (0013): the purge reports % attempts, expected the 31-day-old one at least', v_row.code_attempts;
  end if;
  if exists (select 1 from public.code_attempts where ip_hash = 'ip-0013-old') then
    raise exception 'FAIL (0013): a 31-day-old attempt survived';
  end if;
  if not exists (select 1 from public.code_attempts where ip_hash = 'ip-0013-new') then
    raise exception 'FAIL (0013): a 29-day-old attempt was purged';
  end if;
  select count(*) into n from public.match_actions where match_id in (new_m, live_m);
  if n <> 2 then
    raise exception 'FAIL (0013): % actions left of the recent and the live match, expected 2', n;
  end if;
  if exists (select 1 from public.match_actions where match_id = old_m) then
    raise exception 'FAIL (0013): the old match kept its log';
  end if;
  if not exists (select 1 from public.results where match_id = old_m) then
    raise exception 'FAIL (0013): the old match''s result was purged; ratings need it';
  end if;

  -- The purge's permission ends with the function.
  begin
    delete from public.match_actions where match_id = new_m;
    raise exception 'FAIL (0013): a delete after the purge got past the append-only guard';
  exception
    when raise_exception then
      if sqlerrm not like 'append-only table public.match_actions may not be updated or deleted%' then
        raise;
      end if;
  end;

  raise notice 'OK (0013): old attempts and old logs are purged; recent ones, live logs and results stay';
end $$;
rollback;

\echo '### ALL RETENTION CHECKS RAN ###'
