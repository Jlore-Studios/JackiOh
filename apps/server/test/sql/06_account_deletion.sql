-- Deleting an account (migration 0012). Runs after 05_tutorial_progress.sql; profile 2 is
-- active by then (03 activated it) and plays the other seat here.
\set ON_ERROR_STOP on

-- Same rules as 01-05: every check raises on failure, every expected refusal is matched on its
-- constraint name, and every block runs inside a transaction that is rolled back.

\echo '=== 0012: delete from auth.users removes the account and leaves the other player''s history ==='
begin;
do $$
declare
  gone   constant uuid := 'dddddddd-dddd-dddd-dddd-dddddddddddd';
  other  constant uuid := '22222222-2222-2222-2222-222222222222';
  played constant uuid := 'dddddddd-0000-0000-0000-000000000001';
  room   constant uuid := 'dddddddd-0000-0000-0000-000000000002';
  deck   constant uuid := 'dddddddd-0000-0000-0000-000000000003';
  trio   constant uuid := 'dddddddd-0000-0000-0000-000000000004';
  sid    constant uuid := 'dddddddd-0000-0000-0000-000000000005';
  n      int;
  v_row  record;
begin
  -- A player with everything a real account collects: activated (so the launch grant wrote the
  -- append-only ledger), a code attempt, a code it minted, a deck and a trio, tutorial progress, settings,
  -- a queue ticket, an open room nobody joined, and a finished match with its log, its result
  -- and a finished series against profile 2.
  insert into auth.users (id, email, email_confirmed_at) values (gone, 'gone@example.test', now());
  update public.profiles set status = 'active', activated_at = now() where id = gone;
  select count(*) into n from public.collection_grants where profile_id = gone;
  if n = 0 then
    raise exception 'FAIL (0012): activation granted no cards, so the ledger cascade is not tested';
  end if;

  insert into public.code_attempts (profile_id, ip_hash, succeeded) values (gone, 'ip-0012', false);
  insert into public.invite_codes (code_hash, created_by) values ('hash-0012', gone);
  insert into public.decks (id, profile_id, name, cards, catalog_version)
  values (deck, gone, 'Mine', '[]', 'core-1');
  insert into public.trios (id, profile_id, name, deck1_id) values (trio, gone, 'Trio', deck);
  insert into public.tutorial_progress (profile_id, completed) values (gone, '{basics}');
  insert into public.player_settings (profile_id, groups) values (gone, '{"audio": {"at": 1, "values": {}}}');
  insert into public.tickets (profile_id, slot, rating, frozen_deck, catalog_version, mode)
  values (gone, null, 1000, '[]', 'core-1', 'bo1');

  insert into public.matches (id, room_code, status, seed, p1_profile_id, p1_deck, catalog_version, ceiling_at)
  values (room, 'QWERTZ', 'open', '', gone, '[]', 'core-1', now() + interval '1 hour');

  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (played, 'over', 'seed-0012', gone, other, '[]', '[]', 'core-1', now(), now(), now());
  insert into public.match_actions (match_id, seq, player_id, player_seat, nonce, action)
  values (played, 1, gone, 'p1', 'n-1', '{"type":"endTurn"}'),
         (played, 2, other, 'p2', 'n-2', '{"type":"concede"}');
  insert into public.results (match_id, p1_profile_id, p2_profile_id, winner_profile_id, reason, turns,
                              p1_rating_before, p1_rating_after, p2_rating_before, p2_rating_after)
  values (played, gone, other, gone, 'concede', 2, 1000, 1016, 1000, 984);
  insert into public.series (id, p1_profile_id, p2_profile_id, status, next_match_id, version,
                             catalog_version, winner, state)
  values (sid, gone, other, 'over', gen_random_uuid(), 3, 'core-1', 'p1', '{}');

  -- The delete the Supabase dashboard and the admin API make.
  delete from auth.users where id = gone;

  -- Nothing is keyed to the deleted id any more.
  select (select count(*) from public.profiles where id = gone)
       + (select count(*) from public.collection where profile_id = gone)
       + (select count(*) from public.collection_grants where profile_id = gone)
       + (select count(*) from public.decks where profile_id = gone)
       + (select count(*) from public.trios where profile_id = gone)
       + (select count(*) from public.tutorial_progress where profile_id = gone)
       + (select count(*) from public.player_settings where profile_id = gone)
       + (select count(*) from public.tickets where profile_id = gone)
       + (select count(*) from public.code_attempts where profile_id = gone)
       + (select count(*) from public.invite_codes where created_by = gone)
       + (select count(*) from public.matches where gone in (p1_profile_id, p2_profile_id))
       + (select count(*) from public.match_actions where player_id = gone)
       + (select count(*) from public.results
           where gone in (p1_profile_id, p2_profile_id, winner_profile_id))
       + (select count(*) from public.series where gone in (p1_profile_id, p2_profile_id))
    into n;
  if n <> 0 then
    raise exception 'FAIL (0012): % rows still name the deleted profile', n;
  end if;

  -- The open room went with its host.
  if exists (select 1 from public.matches where id = room) then
    raise exception 'FAIL (0012): the room the deleted profile opened is still there';
  end if;

  -- The other player keeps the match, its whole log, the result and the series.
  select p1_profile_id, p2_profile_id, status into v_row from public.matches where id = played;
  if not found or v_row.p1_profile_id is not null or v_row.p2_profile_id <> other then
    raise exception 'FAIL (0012): the finished match reads %, expected an empty p1 and profile 2', v_row;
  end if;
  select count(*) into n from public.match_actions where match_id = played;
  if n <> 2 then
    raise exception 'FAIL (0012): the match log holds % actions, expected both', n;
  end if;
  select player_id, player_seat into v_row from public.match_actions where match_id = played and seq = 1;
  if v_row.player_id is not null or v_row.player_seat <> 'p1' then
    raise exception 'FAIL (0012): the deleted player''s action reads %, expected no player and seat p1', v_row;
  end if;
  select p1_profile_id, p2_profile_id, winner_profile_id, reason into v_row
    from public.results where match_id = played;
  if not found or v_row.p1_profile_id is not null or v_row.p2_profile_id <> other
     or v_row.winner_profile_id is not null or v_row.reason <> 'concede' then
    raise exception 'FAIL (0012): the result reads %', v_row;
  end if;
  if not exists (select 1 from public.series where id = sid and p1_profile_id is null and p2_profile_id = other) then
    raise exception 'FAIL (0012): the finished series did not keep profile 2 with an empty p1';
  end if;

  -- The attempt keeps what the per-IP limit counts; the code stays usable.
  if not exists (select 1 from public.code_attempts where ip_hash = 'ip-0012' and profile_id is null) then
    raise exception 'FAIL (0012): the code attempt was deleted instead of losing its profile';
  end if;
  if not exists (select 1 from public.invite_codes where code_hash = 'hash-0012' and created_by is null) then
    raise exception 'FAIL (0012): the minted code was deleted instead of losing its creator';
  end if;

  raise notice 'OK (0012): the account and its own rows are gone; the other player''s match, log, result and series stay';
end $$;
rollback;

\echo '=== 0012: a live match or an unfinished series refuses the delete ==='
begin;
do $$
declare
  gone   constant uuid := 'dddddddd-dddd-dddd-dddd-dddddddddddd';
  other  constant uuid := '22222222-2222-2222-2222-222222222222';
  refused_by text;
begin
  insert into auth.users (id, email, email_confirmed_at) values (gone, 'gone@example.test', now());
  update public.profiles set status = 'active', activated_at = now() where id = gone;
  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at)
  values (gen_random_uuid(), 'live', 'seed-live', other, gone, '[]', '[]', 'core-1',
          now() + interval '1 hour', now());

  begin
    delete from auth.users where id = gone;
    raise exception 'FAIL (0012): a player in a live match was deleted';
  exception
    when check_violation then
      get stacked diagnostics refused_by = constraint_name;
      if refused_by is distinct from 'matches_seats_filled_unless_over_check' then
        raise exception 'FAIL (0012): the live match refused by "%"', refused_by;
      end if;
  end;

  delete from public.matches where p2_profile_id = gone;
  insert into public.series (id, p1_profile_id, p2_profile_id, status, next_match_id, version,
                             catalog_version, state)
  values (gen_random_uuid(), gone, other, 'picking', gen_random_uuid(), 0, 'core-1', '{}');
  begin
    delete from auth.users where id = gone;
    raise exception 'FAIL (0012): a player in an unfinished series was deleted';
  exception
    when check_violation then
      get stacked diagnostics refused_by = constraint_name;
      if refused_by is distinct from 'series_players_seated_unless_over_check' then
        raise exception 'FAIL (0012): the unfinished series refused by "%"', refused_by;
      end if;
  end;

  raise notice 'OK (0012): a live match and an unfinished series keep both players';
end $$;
rollback;

\echo '=== 0012: the ledgers of a profile that still exists stay append-only ==='
begin;
do $$
declare
  other constant uuid := '22222222-2222-2222-2222-222222222222';
  played constant uuid := 'dddddddd-0000-0000-0000-000000000009';
begin
  -- A delete of a living profile's ledger row, and an update that only empties its profile
  -- column (what a set-null would do), are both still refused.
  begin
    delete from public.collection_grants where profile_id = other;
    raise exception 'FAIL (0012): a living profile''s grant was deleted';
  exception
    when raise_exception then
      if sqlerrm not like 'append-only table public.collection_grants may not be updated or deleted%' then
        raise;
      end if;
  end;

  insert into public.matches (id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
                              catalog_version, ceiling_at, started_at, ended_at)
  values (played, 'over', 'seed-0012b', other, '11111111-1111-1111-1111-111111111111', '[]', '[]',
          'core-1', now(), now(), now());
  insert into public.match_actions (match_id, seq, player_id, player_seat, nonce, action)
  values (played, 1, other, 'p1', 'n-1', '{"type":"endTurn"}');
  begin
    update public.match_actions set player_id = null where match_id = played;
    raise exception 'FAIL (0012): a living player''s action lost its player';
  exception
    when raise_exception then
      if sqlerrm not like 'append-only table public.match_actions may not be updated or deleted%' then
        raise;
      end if;
  end;
  begin
    delete from public.match_actions where match_id = played;
    raise exception 'FAIL (0012): a logged action was deleted outside the purge';
  exception
    when raise_exception then
      if sqlerrm not like 'append-only table public.match_actions may not be updated or deleted%' then
        raise;
      end if;
  end;

  raise notice 'OK (0012): only a deleted profile''s rows get past the append-only guard';
end $$;
rollback;

\echo '### ALL ACCOUNT DELETION CHECKS RAN ###'
