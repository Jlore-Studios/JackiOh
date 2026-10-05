-- Glitch (migration 0023, issue #170, SPEC §7, R677, R678). Runs after 12_ranked.sql; profiles 1
-- and 2 are active by then (03 activated them).
\set ON_ERROR_STOP on

-- Same rules as 01-12: every check raises on failure, every expected refusal is matched on its
-- constraint name or its message, and every block runs inside a transaction that is rolled back.

\echo '### R677: a match keeps the Glitch boards it started with ###'
begin;
do $$
declare
  p1      constant uuid := '11111111-1111-1111-1111-111111111111';
  p2      constant uuid := '22222222-2222-2222-2222-222222222222';
  mid     constant uuid := 'eeeeeeee-0000-0000-0000-000000000023';
  old     constant uuid := 'eeeeeeee-0000-0000-0000-000000000024';
  v_board constant jsonb := '[{"defId": "core-012", "radiant": true}]';
  v_row   record;
  refused_by text;
begin
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              p1_glitch_board, p2_glitch_board, catalog_version, ceiling_at, started_at)
  values (mid, 'live', 'seed-0023', p1, p2, '[]', '[]', v_board, '[]', 'core-1', now() + interval '1 hour', now());
  -- A row written without them (an open room, any match from before 0023) reads the empty board.
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (old, 'over', 'seed-0022', p1, p2, '[]', '[]', 'core-1', now(), now(), now());

  select p1_glitch_board, p2_glitch_board into v_row from public.matches where id = mid;
  if v_row.p1_glitch_board <> v_board or v_row.p2_glitch_board <> '[]'::jsonb then
    raise exception 'FAIL (R677): the match''s Glitch boards read %, expected p1 % and an empty p2', v_row, v_board;
  end if;
  select p1_glitch_board, p2_glitch_board into v_row from public.matches where id = old;
  if v_row.p1_glitch_board <> '[]'::jsonb or v_row.p2_glitch_board <> '[]'::jsonb then
    raise exception 'FAIL (R677): a match written without Glitch boards reads %, expected two empty boards', v_row;
  end if;
  if not exists (select 1 from app.live_matches() m where m.id = mid and m.p1_glitch_board = v_board) then
    raise exception 'FAIL (R677): app.live_matches() does not carry the Glitch board a restart folds with';
  end if;

  begin
    update public.matches set p2_glitch_board = '[{"radiant": true}]' where id = mid;
    raise exception 'FAIL (R677): a Glitch board entry with no defId was accepted';
  exception when check_violation then
    get stacked diagnostics refused_by = constraint_name;
    if refused_by is distinct from 'matches_glitch_boards_check' then
      raise exception 'FAIL (R677): a malformed Glitch board refused by "%"', refused_by;
    end if;
  end;
  raise notice 'OK (R677): frozen on the row, empty by default, shape checked';
end $$;
rollback;

\echo '### R678: app.forget_voided_match erases a live match and its log, and nothing else ###'
begin;
do $$
declare
  p1       constant uuid := '11111111-1111-1111-1111-111111111111';
  p2       constant uuid := '22222222-2222-2222-2222-222222222222';
  voided   constant uuid := 'eeeeeeee-0000-0000-0000-000000000025';
  over_m   constant uuid := 'eeeeeeee-0000-0000-0000-000000000026';
  result_m constant uuid := 'eeeeeeee-0000-0000-0000-000000000027';
  n        int;
begin
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (voided, 'live', 's-void', p1, p2, '[]', '[]', 'core-1', now() + interval '1 hour', now(), null),
         (over_m, 'over', 's-over', p1, p2, '[]', '[]', 'core-1', now(), now(), now()),
         (result_m, 'live', 's-result', p1, p2, '[]', '[]', 'core-1', now() + interval '1 hour', now(), null);
  insert into public.match_actions (match_id, seq, player_id, player_seat, nonce, action)
  values (voided, 1, p1, 'p1', 'a', '{"type":"endTurn"}'), (voided, 2, p2, 'p2', 'b', '{"type":"endTurn"}'),
         (over_m, 1, p1, 'p1', 'a', '{"type":"concede"}');
  insert into public.results (match_id, p1_profile_id, p2_profile_id, winner_profile_id, reason, turns,
                              p1_rating_before, p1_rating_after, p2_rating_before, p2_rating_after, ended_at)
  values (result_m, p1, p2, null, 'draw-accepted', 2, 1000, 1000, 1000, 1000, now());
  update public.profiles set current_match_id = voided where id in (p1, p2);

  if not app.forget_voided_match(voided) then
    raise exception 'FAIL (R678): a live match with no result was not forgotten';
  end if;
  if exists (select 1 from public.matches where id = voided) then
    raise exception 'FAIL (R678): the voided match row survived';
  end if;
  if exists (select 1 from public.match_actions where match_id = voided) then
    raise exception 'FAIL (R678): the voided match kept its log';
  end if;
  select count(*) into n from public.profiles where id in (p1, p2) and current_match_id is not null;
  if n <> 0 then
    raise exception 'FAIL (R678): % player(s) still held in the voided match', n;
  end if;

  if app.forget_voided_match(over_m) or app.forget_voided_match(result_m) then
    raise exception 'FAIL (R678): a finished match, or one with a result, was forgotten';
  end if;
  select count(*) into n from public.matches where id in (over_m, result_m);
  if n <> 2 or not exists (select 1 from public.match_actions where match_id = over_m) then
    raise exception 'FAIL (R678): a finished match or its log went';
  end if;

  -- The permission ends with the function.
  begin
    delete from public.match_actions where match_id = over_m;
    raise exception 'FAIL (R678): a delete after the void got past the append-only guard';
  exception
    when raise_exception then
      if sqlerrm not like 'append-only table public.match_actions may not be updated or deleted%' then
        raise;
      end if;
  end;

  raise notice 'OK (R678): the voided match and its log are gone, its players free; finished games stay';
end $$;
rollback;

\echo '### ALL GLITCH CHECKS RAN ###'
