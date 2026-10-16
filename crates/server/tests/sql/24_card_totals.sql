-- The card statistics' totals and a game record's detail kept a week (migration 0041, SPEC §9.11,
-- R1044000, R1044001). Runs after 23_duos.sql. The client's refusal on the three totals tables is
-- 02_rls_as_client.sql's CHECK 3, with the other server-only tables.
\set ON_ERROR_STOP on

-- Same rules as 01-23: every check raises on failure and runs inside a rolled-back transaction.
-- Each SPEC §11 row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R1044000: the totals hold a live or dev count under its keys, added to in place, and refuse what no record could make ###'
begin;
do $$
declare
  n bigint;
begin
  insert into public.record_totals (patch, source, mode, p1_pilot, p2_pilot, records)
  values ('v0.2.5', 'dev', 'random', 'ai', 'ai', 2);
  -- The store's fold: a count already held is added to, never written over.
  insert into public.record_totals (patch, source, mode, p1_pilot, p2_pilot, records)
  values ('v0.2.5', 'dev', 'random', 'ai', 'ai', 3)
  on conflict (patch, source, mode, p1_pilot, p2_pilot)
  do update set records = public.record_totals.records + excluded.records;
  select records into n from public.record_totals where patch = 'v0.2.5' and source = 'dev';
  if n <> 5 then
    raise exception 'FAIL (R1044000): the record count reads %, expected 2 + 3', n;
  end if;

  begin
    insert into public.card_totals (patch, source, mode, pilot, card, stats)
    values ('v0.2.5', 'practice', 'random', 'ai', 'core-001', '{}'::jsonb);
    raise exception 'FAIL (R1044000): a card total was filed under the source practice';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.card_totals (patch, source, mode, pilot, card, stats)
    values ('v0.2.5', 'dev', 'random', 'bot', 'core-001', '{}'::jsonb);
    raise exception 'FAIL (R1044000): a card total was filed under the pilot bot';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.decklist_totals (patch, list, deck, games, wins, first_record, first_seat)
    values ('v0.2.5', 'core-001', '["core-001"]'::jsonb, 1, 0, 'm-1', 2);
    raise exception 'FAIL (R1044000): a decklist was first seen in a third seat';
  exception
    when check_violation then null;
  end;

  raise notice 'OK (R1044000): counts add in place; no source, pilot or seat a record cannot have';
end $$;
rollback;

\echo '### R1044001: the purge empties a folded game record''s detail past its cutoff, keeps its row, and never empties one not folded ###'
begin;
do $$
declare
  rec jsonb := jsonb_build_object(
    'id', 'dev:totals:0', 'source', 'dev', 'mode', 'random', 'patch', 'v0.2.5',
    'pilots', jsonb_build_object('p1', 'ai', 'p2', 'ai'),
    'game', jsonb_build_object('first', 'p1', 'winner', 'p2', 'reason', 'hero-death', 'turns', 9,
      'seats', jsonb_build_object(
        'p1', jsonb_build_object('deck', '["core-001"]'::jsonb, 'opening', '[]'::jsonb, 'drawn', '[]'::jsonb, 'played', '[]'::jsonb),
        'p2', jsonb_build_object('deck', '["core-002"]'::jsonb, 'opening', '[]'::jsonb, 'drawn', '[]'::jsonb, 'played', '[]'::jsonb))));
  v_row record;
  n     int;
begin
  -- A folded record eight days old, a folded one six days old, and one eight days old not folded.
  insert into public.game_records (id, source, mode, patch, record, folded, recorded_at)
  values ('dev:totals:old', 'dev', 'random', 'v0.2.5', rec || '{"id":"dev:totals:old"}', true,
          now() - interval '8 days'),
         ('dev:totals:new', 'dev', 'random', 'v0.2.5', rec || '{"id":"dev:totals:new"}', true,
          now() - interval '6 days'),
         ('dev:totals:unfolded', 'dev', 'random', 'v0.2.5', rec || '{"id":"dev:totals:unfolded"}', false,
          now() - interval '8 days');

  -- As the server calls it: 0029's three cutoffs and the game records' detail at seven days.
  select * into v_row from app.purge_expired_rows(now() - interval '30 days', now() - interval '90 days',
                                                  now() - interval '365 days', now() - interval '7 days');
  if v_row.game_records <> 1 then
    raise exception 'FAIL (R1044001): the purge reports % records emptied, expected the old folded one', v_row.game_records;
  end if;
  if not exists (select 1 from public.game_records where id = 'dev:totals:old' and record is null and folded) then
    raise exception 'FAIL (R1044001): the old folded record still holds its detail, or its row went';
  end if;
  select count(*) into n from public.game_records
   where id in ('dev:totals:new', 'dev:totals:unfolded') and record is not null;
  if n <> 2 then
    raise exception 'FAIL (R1044001): % of the new and the unfolded record kept their detail, expected both', n;
  end if;

  -- The emptied row still holds the id: the same game written again writes nothing.
  insert into public.game_records (id, source, mode, patch, record, folded)
  values ('dev:totals:old', 'dev', 'random', 'v0.2.5', rec || '{"id":"dev:totals:old"}', true)
  on conflict (id) do nothing;
  get diagnostics n = row_count;
  if n <> 0 then
    raise exception 'FAIL (R1044001): a game whose detail was emptied was written again';
  end if;

  -- No path empties a record the totals do not hold.
  begin
    update public.game_records set record = null where id = 'dev:totals:unfolded';
    raise exception 'FAIL (R1044001): an unfolded record''s detail was emptied';
  exception
    when check_violation then null;
  end;

  raise notice 'OK (R1044001): the old folded record emptied and kept by id; the new and the unfolded kept their detail';
end $$;
rollback;

\echo '### ALL CARD TOTALS CHECKS RAN ###'
