-- Each patch's whole games, kept past the week's purge (migration 0063, SPEC §9.11, R1994000).
-- Runs after 34_cosmetic_loadouts.sql. The client's refusal on game_totals is 02_rls_as_client.sql's
-- CHECK 3, with the other server-only tables.
\set ON_ERROR_STOP on

-- Same rules as 01-34: every check raises on failure and runs inside a rolled-back transaction.
-- Each SPEC §11 row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R1994000: game_totals keeps one key''s whole games, refuses what no record makes, and is the server''s alone ###'
begin;
do $$
declare
  n bigint;
begin
  insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
  values ('v0.5.0b', 'live', 'bo1', 'human', 'ai', '{"games": 1, "firstWins": 1}');
  -- The store's write-back: the key's total, read and added to in Rust, replaces the one held.
  insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
  values ('v0.5.0b', 'live', 'bo1', 'human', 'ai', '{"games": 3, "firstWins": 2}')
  on conflict (patch, source, mode, p1_pilot, p2_pilot) do update set total = excluded.total;
  select (total ->> 'games')::bigint into n
    from public.game_totals
   where patch = 'v0.5.0b' and source = 'live' and mode = 'bo1' and p1_pilot = 'human' and p2_pilot = 'ai';
  if n is distinct from 3 then
    raise exception 'FAIL (R1994000): the write-back left % games, expected the 3 written', n;
  end if;
  select count(*) into n from public.game_totals where patch = 'v0.5.0b';
  if n <> 1 then
    raise exception 'FAIL (R1994000): one key holds % rows, expected 1', n;
  end if;

  -- What no folded record makes: a practice or tutorial game, a pilot that is neither, a total of no
  -- games or with no count of them.
  begin
    insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
    values ('v0.5.0b', 'practice', 'bo1', 'human', 'human', '{"games": 1}');
    raise exception 'FAIL (R1994000): a total of source practice was kept';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
    values ('v0.5.0b', 'live', 'tutorial', 'human', 'human', '{"games": 1}');
    raise exception 'FAIL (R1994000): a total of mode tutorial was kept';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
    values ('v0.5.0b', 'live', 'bo1', 'human', 'bot', '{"games": 1}');
    raise exception 'FAIL (R1994000): a total with pilot bot was kept';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
    values ('v0.5.0b', 'live', 'bo3', 'human', 'human', '{"games": 0}');
    raise exception 'FAIL (R1994000): a total of no games was kept';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
    values ('v0.5.0b', 'live', 'random', 'human', 'human', '{}');
    raise exception 'FAIL (R1994000): a total with no count of games was kept';
  exception
    when check_violation then null;
  end;

  if not (select c.relrowsecurity from pg_class c where c.oid = 'public.game_totals'::regclass) then
    raise exception 'FAIL (R1994000): game_totals has RLS off';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'game_totals') then
    raise exception 'FAIL (R1994000): game_totals has a policy, so a client could reach it';
  end if;
  raise notice 'OK (R1994000): a key''s total is replaced whole, what no record makes is refused, and the table is the server''s alone';
end $$;
rollback;

\echo '### R1994000: 0063 re-applies over rows of its own and changes none ###'
begin;
insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
values ('v0.5.0b', 'dev', 'random', 'ai', 'ai', '{"games": 2}');
create temp table r1994000_rows (n bigint not null) on commit drop;
insert into r1994000_rows select count(*) from public.game_totals;
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0063_game_totals.sql
set local client_min_messages = notice;
do $$
begin
  if (select n from r1994000_rows) <> 1 then
    raise exception 'FAIL (R1994000): % totals before the re-apply, expected the one written', (select n from r1994000_rows);
  end if;
  if (select count(*) from public.game_totals) <> (select n from r1994000_rows) then
    raise exception 'FAIL (R1994000): re-applying 0063 changed the totals';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'game_totals') then
    raise exception 'FAIL (R1994000): re-applying 0063 left a policy on game_totals';
  end if;
  raise notice 'OK (R1994000): 0063 re-applies over its own rows and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
