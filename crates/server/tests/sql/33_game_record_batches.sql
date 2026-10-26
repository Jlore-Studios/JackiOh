-- The files stats-import --totals-only folds into the totals (migration 0060, SPEC §9.11, R1976000).
-- Runs after 32_portrait_cosmetics.sql. The client's refusal on game_record_batches is
-- 02_rls_as_client.sql's CHECK 3, with the other server-only tables.
\set ON_ERROR_STOP on

-- Same rules as 01-32: every check raises on failure and runs inside a rolled-back transaction.
-- Each SPEC §11 row this file proves is named in a `### Rnnn: … ###` heading.

\echo '### R1976000: a file is claimed once, by the SHA-256 of its contents, with the records it folded ###'
begin;
do $$
declare
  sha constant text := repeat('0123456789abcdef', 4);
  n bigint;
begin
  insert into public.game_record_batches (batch, records) values (sha, 2);
  -- The store's claim: a batch held already claims nothing, and its count stays.
  insert into public.game_record_batches (batch, records) values (sha, 5)
  on conflict (batch) do nothing;
  get diagnostics n = row_count;
  if n <> 0 then
    raise exception 'FAIL (R1976000): a file was claimed twice';
  end if;
  select records into n from public.game_record_batches where batch = sha;
  if n <> 2 then
    raise exception 'FAIL (R1976000): the claim holds % records, expected the first claim''s 2', n;
  end if;

  -- A file whose records were all imported a record at a time folds none, and is still claimed.
  insert into public.game_record_batches (batch, records) values (repeat('f', 64), 0);

  begin
    insert into public.game_record_batches (batch, records) values ('not-a-sha', 1);
    raise exception 'FAIL (R1976000): a batch that is no SHA-256 was claimed';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_record_batches (batch, records) values (upper(sha), 1);
    raise exception 'FAIL (R1976000): a batch in uppercase hex was claimed';
  exception
    when check_violation then null;
  end;
  begin
    insert into public.game_record_batches (batch, records) values (repeat('e', 64), -1);
    raise exception 'FAIL (R1976000): a batch folded fewer than no records';
  exception
    when check_violation then null;
  end;

  if not (select c.relrowsecurity from pg_class c where c.oid = 'public.game_record_batches'::regclass) then
    raise exception 'FAIL (R1976000): game_record_batches has RLS off';
  end if;
  if exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'game_record_batches') then
    raise exception 'FAIL (R1976000): game_record_batches has a policy, so a client could reach it';
  end if;
  raise notice 'OK (R1976000): a file is claimed once by its SHA-256, its count kept, and the table is the server''s alone';
end $$;
rollback;

\echo '=== ALL CHECKS RAN ==='
