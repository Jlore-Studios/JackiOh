-- R1434's input: profiles signed up before migration 0028 gave every profile a username.
--
-- run.sh applies this AFTER migrations 0001-0027 and BEFORE 0028, which is the moment a real
-- database that predates usernames meets 0028: its profiles exist, and none has a name. 0028's
-- backfill then names each one `Player#n` in order of sign-up (created_at, then id), and
-- 15_usernames.sql checks the numbers it gave.
--
-- Everything here is chosen so that sign-up order is the only order that gives the right answer:
--
--   * three profiles of their own, b1400000-…-1, -2 and -3: 01 signs up 11111111-… and 22222222-…,
--     03 signs up 33333333-…, 03b signed up 44444444-…, and none of those exists before 0028 but
--     03b's;
--   * signed up in 2020, before 03b's profile (signed up as this run started), so 03b's profile,
--     whose id sorts first, is numbered last: Player#1 is 2020-01-01 (-3), #2 is 2020-01-02 (-2),
--     #3 is 2020-01-03 (-1) and #4 is 03b's;
--   * inserted newest first, and with ids that sort newest first, so neither the order the rows
--     were written in (a sequential scan's) nor id order is sign-up order.
--
-- The profiles stay pending: every launch grant and every count 01-14 make is of active profiles
-- or of their own ids, so these three disturb nothing they measure.
\set ON_ERROR_STOP on

\echo '--- 14b: seeding profiles that predate usernames ---'

do $$
begin
  -- Vacuity guard for the whole backfill check: if profiles had a username already, 0028 has run
  -- and these profiles would be named by the trigger, not by the backfill.
  if exists (select 1 from information_schema.columns
              where table_schema = 'public' and table_name = 'profiles'
                and column_name like 'username%') then
    raise exception 'FAIL (14b): public.profiles has a username column already — this seed must run before migration 0028';
  end if;
  if not exists (select 1 from information_schema.columns
                  where table_schema = 'public' and table_name = 'profiles'
                    and column_name = 'display_name') then
    raise exception 'FAIL (14b): public.profiles has no display_name — this is not the schema 0028 replaces';
  end if;
end $$;

-- Newest first. Each signup goes through 0001's app.handle_new_user, as a real one does; the
-- sign-up time is then set to the day the account is meant to have signed up on.
insert into auth.users (id, email, email_confirmed_at)
values ('b1400000-0000-4000-8000-000000000001', 'third@example.test', now());
insert into auth.users (id, email, email_confirmed_at)
values ('b1400000-0000-4000-8000-000000000002', 'second@example.test', now());
insert into auth.users (id, email, email_confirmed_at)
values ('b1400000-0000-4000-8000-000000000003', 'first@example.test', now());

update public.profiles p
   set created_at = signed_up.at
  from (values ('b1400000-0000-4000-8000-000000000001'::uuid, timestamptz '2020-01-03 00:00:00+00'),
               ('b1400000-0000-4000-8000-000000000002'::uuid, timestamptz '2020-01-02 00:00:00+00'),
               ('b1400000-0000-4000-8000-000000000003'::uuid, timestamptz '2020-01-01 00:00:00+00'))
       as signed_up (id, at)
 where p.id = signed_up.id;

do $$
declare
  n   bigint;
  bad text;
begin
  select count(*) into n from public.profiles
   where id in ('b1400000-0000-4000-8000-000000000001', 'b1400000-0000-4000-8000-000000000002',
                'b1400000-0000-4000-8000-000000000003')
     and status = 'pending' and created_at < timestamptz '2020-01-04 00:00:00+00';
  if n <> 3 then
    raise exception 'FAIL (14b): % of the 3 seeded profiles exist, pending and signed up in 2020', n;
  end if;
  -- The whole input of 0028's backfill: these three and 03b's profile, signed up later than all of
  -- them. 15 expects exactly four `Player#n`s from the backfill, so a fifth profile here would be one
  -- it does not account for.
  select string_agg(format('%s at %s', id, created_at), ', ' order by created_at, id) into bad
    from public.profiles
   where id not in ('b1400000-0000-4000-8000-000000000001', 'b1400000-0000-4000-8000-000000000002',
                    'b1400000-0000-4000-8000-000000000003', '44444444-4444-4444-4444-444444444444')
      or (id = '44444444-4444-4444-4444-444444444444' and created_at < timestamptz '2020-01-04 00:00:00+00');
  if bad is not null then
    raise exception 'FAIL (14b): profiles other than 03b''s and these three exist before 0028, or 03b''s signed up first: %', bad;
  end if;
  if not exists (select 1 from public.profiles where id = '44444444-4444-4444-4444-444444444444') then
    raise exception 'FAIL (14b): 03b''s profile is gone, so the backfill numbers only three profiles';
  end if;
  raise notice 'OK (14b): 4 profiles predate 0028, signed up 2020-01-01, 2020-01-02, 2020-01-03 (inserted newest first) and 03b''s last';
end $$;
