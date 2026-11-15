-- The daily games' player mail, counted per UTC day and held to DAILY_PLAYER_EMAILS_PER_DAY (migration
-- 0092, SPEC §9.13; issue #3843). Runs after 55_two_v_two.sql.
\set ON_ERROR_STOP on

-- Same rules as 01-55: every check raises on failure (`raise exception 'FAIL (…): …'`), every
-- expected refusal is matched on its sqlstate, each check that could pass vacuously carries a guard,
-- and every block that writes runs inside a transaction that is rolled back. Each heading names the
-- issue this file proves, as `### #3843: … ###`.

\echo '### #3843: public.daily_email_days holds one count per UTC day, no account, and is the server''s alone ###'
do $$
declare
  v_text text;
  op     text;
  who    text;
  wrong  text[] := array[]::text[];
begin
  if not exists (select 1 from pg_class
                  where oid = 'public.daily_email_days'::regclass and relrowsecurity) then
    raise exception 'FAIL (#3843): public.daily_email_days has no row level security';
  end if;

  -- The columns, as the store reads and writes them: a day and its count, no account.
  select string_agg(format('%s %s %s', column_name, data_type, is_nullable), ', ' order by ordinal_position)
    into v_text
    from information_schema.columns
   where table_schema = 'public' and table_name = 'daily_email_days';
  if v_text is distinct from 'day bigint NO, sent integer NO' then
    raise exception 'FAIL (#3843): public.daily_email_days has the columns [%]', coalesce(v_text, '(none)');
  end if;

  -- One count per day.
  select string_agg(a.attname, ', ' order by k.ord) into v_text
    from pg_constraint c
    cross join lateral unnest(c.conkey) with ordinality as k(attnum, ord)
    join pg_attribute a on a.attrelid = c.conrelid and a.attnum = k.attnum
   where c.conrelid = 'public.daily_email_days'::regclass and c.contype = 'p';
  if v_text is distinct from 'day' then
    raise exception 'FAIL (#3843): the primary key is (%), expected (day)', coalesce(v_text, 'none');
  end if;

  -- No foreign key: a count is no account's.
  if exists (select 1 from pg_constraint
              where conrelid = 'public.daily_email_days'::regclass and contype = 'f') then
    raise exception 'FAIL (#3843): daily_email_days has a foreign key, so a count would be someone''s';
  end if;

  -- No policy and no client privilege.
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'daily_email_days') then
    raise exception 'FAIL (#3843): daily_email_days has a policy, so a client could reach it';
  end if;
  foreach who in array array['authenticated', 'anon'] loop
    foreach op in array array['SELECT', 'INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'REFERENCES', 'TRIGGER'] loop
      if has_table_privilege(who, 'public.daily_email_days', op) then
        wrong := wrong || format('%s %s', who, op);
      end if;
    end loop;
  end loop;
  if cardinality(wrong) > 0 then
    raise exception 'FAIL (#3843): a client holds [%] on daily_email_days', array_to_string(wrong, ', ');
  end if;
  raise notice 'OK (#3843): daily_email_days has RLS on, its columns, one row per day, no foreign key, and no policy or client privilege';
end $$;

\echo '### #3843: a count below 0 and a second row for one day are refused, the claim stops at the cap, and the release never goes below 0 ###'
begin;
do $$
declare
  d   constant bigint  := 864000000;
  cap constant integer := 2;
  -- How many rows each claim answered, in order: the store reads one as "taken".
  answered integer[] := array[]::integer[];
  v_sent integer;
  n      integer;
  i      int;
begin
  begin
    insert into public.daily_email_days (day, sent) values (d, -1);
    raise exception 'FAIL (#3843): the schema took a count below 0';
  exception
    when check_violation then null;
  end;

  -- The store's claim (src/db/pg.rs, `daily_email_claim`): one statement that adds one only while the
  -- day's count is under the cap, and answers a row only when it did.
  for i in 1 .. 3 loop
    insert into public.daily_email_days as days (day, sent) values (d, 1)
    on conflict (day) do update set sent = days.sent + 1 where days.sent < cap
    returning sent into v_sent;
    get diagnostics n = row_count;
    answered := answered || n;
  end loop;
  if answered is distinct from array[1, 1, 0] then
    raise exception 'FAIL (#3843): three claims at a cap of 2 answered % rows, expected {1,1,0}', answered;
  end if;
  if (select sent from public.daily_email_days where day = d) is distinct from 2 then
    raise exception 'FAIL (#3843): after three claims at a cap of 2 the day holds %, expected 2',
      (select sent from public.daily_email_days where day = d);
  end if;

  -- One row per day.
  begin
    insert into public.daily_email_days (day, sent) values (d, 0);
    raise exception 'FAIL (#3843): a second row for one day was taken';
  exception
    when unique_violation then null;
  end;

  -- The store's release (`daily_email_release`): one back, never below 0.
  for i in 1 .. 3 loop
    update public.daily_email_days set sent = sent - 1 where day = d and sent > 0;
  end loop;
  if (select sent from public.daily_email_days where day = d) is distinct from 0 then
    raise exception 'FAIL (#3843): three releases of a count of 2 left %, expected 0',
      (select sent from public.daily_email_days where day = d);
  end if;
  raise notice 'OK (#3843): a count below 0 and a second row for one day are refused, three claims at a cap of 2 take 2, and three releases stop at 0';
end $$;
rollback;

\echo '### #3843: 0092 re-applies over a count of its own and changes none ###'
begin;
insert into public.daily_email_days (day, sent) values (864000000, 7);
-- What run.sh's apply_migration filters, a re-run's "already exists, skipping", is quieted here.
set local client_min_messages = warning;
\i /tmp/0092_daily_email_days.sql
set local client_min_messages = notice;
do $$
begin
  if (select count(*) from public.daily_email_days) <> 1 then
    raise exception 'FAIL (#3843): % counts after the re-apply, expected the one written',
      (select count(*) from public.daily_email_days);
  end if;
  if (select sent from public.daily_email_days where day = 864000000) is distinct from 7 then
    raise exception 'FAIL (#3843): re-applying 0092 changed a count';
  end if;
  if (select count(*) from pg_constraint
       where conrelid = 'public.daily_email_days'::regclass and conname = 'daily_email_days_sent_check') <> 1 then
    raise exception 'FAIL (#3843): re-applying 0092 left daily_email_days_sent_check other than once';
  end if;
  if not exists (select 1 from pg_class
                  where oid = 'public.daily_email_days'::regclass and relrowsecurity) then
    raise exception 'FAIL (#3843): re-applying 0092 turned row level security off';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'daily_email_days') then
    raise exception 'FAIL (#3843): re-applying 0092 left a policy on daily_email_days';
  end if;
  raise notice 'OK (#3843): 0092 re-applies over its own count and changes none';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
