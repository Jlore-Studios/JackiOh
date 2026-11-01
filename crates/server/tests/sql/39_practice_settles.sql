-- A practice claim kept until it settles, once (migration 0071, SPEC §14.8; R2422000; issue #2422).
-- Runs after 38_v0_5_retcon.sql; profile 1 (11111111-…) exists by then. The client's refusal on
-- practice_seeds is 02_rls_as_client.sql's CHECK 3, with the other server-only tables, and 0071
-- grants nothing.
\set ON_ERROR_STOP on

-- Same rules as 01-38: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a
-- guard, and every block that writes runs inside a transaction that is rolled back. Each SPEC §11
-- row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R2422000: public.practice_seeds keeps a claim, its mark and the mark''s time, and the sweep''s index ###'
do $$
declare
  v_text text;
begin
  -- The columns, as the store reads and writes them: 0047's five, then 0071's three.
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'practice_seeds';
  if v_text is distinct from 'seed text NO, profile_id uuid NO, setup jsonb NO, '
                             'issued_at timestamp with time zone NO, claimed_at timestamp with time zone YES, '
                             'claim jsonb YES, settled text YES, settled_at timestamp with time zone YES' then
    raise exception 'FAIL (R2422000): public.practice_seeds has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- The sweep's index: the claims kept and unsettled, by when they were made.
  select pg_get_indexdef(i.indexrelid) into v_text
    from pg_index i
    join pg_class c on c.oid = i.indexrelid
   where i.indrelid = 'public.practice_seeds'::regclass and c.relname = 'practice_seeds_unsettled';
  if v_text is null then
    raise exception 'FAIL (R2422000): practice_seeds_unsettled, the sweep''s index, is missing';
  end if;
  if v_text not like '%(claimed_at)%' or v_text not like '%claim IS NOT NULL%'
     or v_text not like '%settled_at IS NULL%' then
    raise exception 'FAIL (R2422000): practice_seeds_unsettled is [%], not the unsettled claims by claimed_at', v_text;
  end if;

  if not exists (select 1 from pg_class
                  where oid = 'public.practice_seeds'::regclass and relrowsecurity) then
    raise exception 'FAIL (R2422000): public.practice_seeds lost its row level security';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'practice_seeds') then
    raise exception 'FAIL (R2422000): practice_seeds has a policy, so a client could reach a claim';
  end if;
  raise notice 'OK (R2422000): practice_seeds has 0047''s columns and 0071''s claim, settled and settled_at, the sweep''s partial index, RLS on and no policy';
end $$;

\echo '### R2422000: a mark comes with its time and is paid or refused, and only a claimed seed holds a claim or a mark ###'
begin;
do $$
declare
  p1 constant uuid := '11111111-1111-1111-1111-111111111111';
  -- [what, claimed, claim, settled, settled_at]. Each breaks practice_seeds_settled_check alone.
  refused constant text[][] := array[
    ['a mark without its time', 'yes', null, 'paid', null],
    ['a time without its mark', 'yes', null, null, 'yes'],
    ['the outcome lost', 'yes', null, 'lost', 'yes'],
    ['a mark on an unclaimed seed', null, null, 'refused', 'yes'],
    ['a claim on an unclaimed seed', null, '{"result": "loss", "log": []}', null, null]];
  -- [what, claimed, claim, settled, settled_at]: the four shapes the store writes.
  taken constant text[][] := array[
    ['an unclaimed seed', null, null, null, null],
    ['a claim kept, unsettled', 'yes', '{"result": "win", "log": []}', null, null],
    ['a claim paid', 'yes', null, 'paid', 'yes'],
    ['a claim refused', 'yes', null, 'refused', 'yes']];
  i int;
begin
  if not exists (select 1 from public.profiles where id = p1) then
    raise exception 'FAIL (R2422000): profile 1 is missing, so every insert below would fail on its foreign key';
  end if;
  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.practice_seeds (seed, profile_id, setup, issued_at, claimed_at, claim, settled, settled_at)
      values (format('r2422000-refused-%s', i), p1, '{}', now(),
              case when refused[i][2] is null then null else now() end,
              refused[i][3]::jsonb, refused[i][4],
              case when refused[i][5] is null then null else now() end);
      raise exception 'FAIL (R2422000): the schema took %', refused[i][1];
    exception
      when check_violation then null;
    end;
  end loop;

  -- The vacuity guard: every row the store writes is taken.
  for i in 1 .. array_length(taken, 1) loop
    insert into public.practice_seeds (seed, profile_id, setup, issued_at, claimed_at, claim, settled, settled_at)
    values (format('r2422000-taken-%s', i), p1, '{}', now(),
            case when taken[i][2] is null then null else now() end,
            taken[i][3]::jsonb, taken[i][4],
            case when taken[i][5] is null then null else now() end);
  end loop;
  if (select count(*) from public.practice_seeds where seed like 'r2422000-taken-%') <> array_length(taken, 1) then
    raise exception 'FAIL (R2422000): the rows the check allows were not all written';
  end if;
  raise notice 'OK (R2422000): % rows practice_seeds_settled_check refuses are refused, and the % shapes the store writes are taken',
    array_length(refused, 1), array_length(taken, 1);
end $$;
rollback;

\echo '### R2422000: 0071 re-applies over a kept claim and changes none ###'
begin;
insert into public.practice_seeds (seed, profile_id, setup, issued_at, claimed_at, claim)
values ('r2422000-kept', '11111111-1111-1111-1111-111111111111', '{}', now(), now(),
        '{"result": "loss", "log": [{"type": "concede", "playerId": "p2", "nonce": "h0"}]}');
create temp table r2422000_before (claim jsonb) on commit drop;
insert into r2422000_before select claim from public.practice_seeds where seed = 'r2422000-kept';
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0071_practice_settles.sql
set local client_min_messages = notice;
do $$
begin
  if (select count(*) from r2422000_before where claim is not null) <> 1 then
    raise exception 'FAIL (R2422000): the kept claim was not written before the re-apply';
  end if;
  if (select claim from public.practice_seeds where seed = 'r2422000-kept')
     is distinct from (select claim from r2422000_before) then
    raise exception 'FAIL (R2422000): re-applying 0071 changed a kept claim';
  end if;
  if (select settled from public.practice_seeds where seed = 'r2422000-kept') is not null then
    raise exception 'FAIL (R2422000): re-applying 0071 marked a claim settled';
  end if;
  if (select count(*) from pg_constraint
       where conrelid = 'public.practice_seeds'::regclass and conname = 'practice_seeds_settled_check') <> 1 then
    raise exception 'FAIL (R2422000): re-applying 0071 left practice_seeds_settled_check other than once';
  end if;
  raise notice 'OK (R2422000): 0071 re-applies over a kept claim and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
