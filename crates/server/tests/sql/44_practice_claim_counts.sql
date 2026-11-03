-- How practice claims ended, counted per UTC day and outcome (migration 0078, SPEC §14.8; issue #2884).
-- Runs after 42_animated_art.sql. The client's refusal on practice_claim_counts is also
-- 02_rls_as_client.sql's CHECK 3, with the other server-only tables.
\set ON_ERROR_STOP on

-- Same rules as 01-42: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a guard,
-- and every block that writes runs inside a transaction that is rolled back. Each heading names the
-- issue this file proves, as `### #2884: … ###`.

\echo '### #2884: public.practice_claim_counts holds one count per day and outcome, no account, and is the server''s alone ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  if not exists (select 1 from pg_class
                  where oid = 'public.practice_claim_counts'::regclass and relrowsecurity) then
    raise exception 'FAIL (#2884): public.practice_claim_counts has no row level security';
  end if;

  -- The columns, as the store reads and writes them: aggregates, no account.
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'practice_claim_counts';
  if v_text is distinct from 'day date NO, outcome text NO, count bigint NO' then
    raise exception 'FAIL (#2884): public.practice_claim_counts has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- One count per day and outcome.
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.practice_claim_counts'::regclass and c.contype = 'p';
  if v_text is distinct from 'day, outcome' then
    raise exception 'FAIL (#2884): the primary key is (%), expected (day, outcome)', coalesce(v_text, 'none');
  end if;

  -- No foreign key: a count is no account's.
  if exists (select 1 from pg_constraint
              where conrelid = 'public.practice_claim_counts'::regclass and contype = 'f') then
    raise exception 'FAIL (#2884): practice_claim_counts has a foreign key, so a count would be someone''s';
  end if;

  -- No policy and no client privilege.
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'practice_claim_counts') then
    raise exception 'FAIL (#2884): practice_claim_counts has a policy, so a client could reach it';
  end if;
  foreach who in array array['authenticated', 'anon'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.practice_claim_counts', op) then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (#2884): a client holds [%] on practice_claim_counts', array_to_string(wrong, ', ');
  end if;
  raise notice 'OK (#2884): practice_claim_counts has RLS on, its columns, one row per (day, outcome), no foreign key, and no policy or client privilege';
end $$;

\echo '### #2884: the checks take every outcome a claim ends in and refuse another, and a count is above 0 and one row per day and outcome ###'
begin;
do $$
declare
  d constant date := date '1970-01-11';
  -- `ClaimBucket::name` of every bucket (src/economy/practice.rs).
  outcomes constant text[] := array['paid', 'economy_off', 'unknown_seed', 'already_claimed', 'expired',
                                    'does_not_fold', 'not_finished', 'voided', 'result_mismatch'];
  -- [what, outcome, count]. Each breaks one of 0078's checks and nothing else.
  refused constant text[][] := array[
    ['an outcome no claim ends in', 'lost', '1'],
    ['a count of 0', 'paid', '0'],
    ['a count below 0', 'paid', '-1']];
  i int;
begin
  -- The vacuity guard: every outcome the store writes is taken.
  for i in 1 .. array_length(outcomes, 1) loop
    insert into public.practice_claim_counts (day, outcome, count) values (d, outcomes[i], 1);
  end loop;
  if (select count(*) from public.practice_claim_counts where day = d) <> array_length(outcomes, 1) then
    raise exception 'FAIL (#2884): the nine outcomes were not all written';
  end if;

  for i in 1 .. array_length(refused, 1) loop
    begin
      insert into public.practice_claim_counts (day, outcome, count)
      values (d + 1, refused[i][2], refused[i][3]::bigint);
      raise exception 'FAIL (#2884): the schema took %', refused[i][1];
    exception
      when check_violation then null;
    end;
  end loop;

  -- One row per day and outcome.
  begin
    insert into public.practice_claim_counts (day, outcome, count) values (d, 'paid', 1);
    raise exception 'FAIL (#2884): a second row for one day and outcome was taken';
  exception
    when unique_violation then null;
  end;

  -- The store's count: one more on the day's row.
  insert into public.practice_claim_counts as counts (day, outcome, count)
  values (d, 'paid', 1)
  on conflict (day, outcome) do update set count = counts.count + 1;
  if (select count from public.practice_claim_counts where day = d and outcome = 'paid') <> 2 then
    raise exception 'FAIL (#2884): the store''s upsert did not add one to the day''s paid row';
  end if;
  raise notice 'OK (#2884): the nine outcomes are taken, % rows the checks refuse are refused, a second row for one day and outcome too, and the upsert adds one',
    array_length(refused, 1);
end $$;
rollback;

\echo '### #2884: 0078 re-applies over counts of its own and changes none ###'
begin;
insert into public.practice_claim_counts (day, outcome, count) values (date '1970-01-11', 'expired', 3);
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0078_practice_claim_counts.sql
set local client_min_messages = notice;
do $$
declare
  con text;
begin
  if (select count(*) from public.practice_claim_counts) <> 1 then
    raise exception 'FAIL (#2884): % counts after the re-apply, expected the one written',
      (select count(*) from public.practice_claim_counts);
  end if;
  if (select count from public.practice_claim_counts where day = date '1970-01-11' and outcome = 'expired') <> 3 then
    raise exception 'FAIL (#2884): re-applying 0078 changed a count';
  end if;
  foreach con in array array['practice_claim_counts_outcome_check', 'practice_claim_counts_count_check'] loop
    if (select count(*) from pg_constraint
         where conrelid = 'public.practice_claim_counts'::regclass and conname = con) <> 1 then
      raise exception 'FAIL (#2884): re-applying 0078 left % other than once', con;
    end if;
  end loop;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'practice_claim_counts') then
    raise exception 'FAIL (#2884): re-applying 0078 left a policy on practice_claim_counts';
  end if;
  raise notice 'OK (#2884): 0078 re-applies over its own counts and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
