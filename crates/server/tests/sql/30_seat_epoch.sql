-- The seat ledger's epoch (migration 0054, SPEC §9.5; R1523000; issue #1523).
-- Runs after 29_going_live.sql. No check here needs a row 01-29 wrote: each reads app.seat_epoch,
-- which only 0054 writes.
\set ON_ERROR_STOP on

-- Same rules as 01-29: every check raises on failure (`raise exception 'FAIL (…): …'`), each check
-- that could pass vacuously carries a guard, and every block runs inside a transaction that is
-- rolled back. Nothing here selects into the log: the do blocks raise or say OK.

\echo '### R1523000: one epoch row, and a fence on an older epoch finds nothing ###'
begin;
do $$
declare
  n       bigint;
  older   bigint;
  current bigint;
begin
  select count(*) into n from app.seat_epoch;
  if n <> 1 then
    raise exception 'FAIL (R1523000): app.seat_epoch holds % rows, not one', n;
  end if;
  select epoch into older from app.seat_epoch;

  -- A booting server's bump, as `seats_bump_epoch` makes it.
  update app.seat_epoch set epoch = epoch + 1 returning epoch into current;
  if current <> older + 1 then
    raise exception 'FAIL (R1523000): the bump moved the epoch from % to %', older, current;
  end if;

  -- The fence, as `seats_fence` takes it: the older epoch finds no row, the current one its row.
  perform 1 from app.seat_epoch where epoch = older for share;
  if found then
    raise exception 'FAIL (R1523000): a fence on the older epoch % still found the row', older;
  end if;
  perform 1 from app.seat_epoch where epoch = current for share;
  if not found then
    raise exception 'FAIL (R1523000): a fence on the current epoch % found no row', current;
  end if;

  raise notice 'OK (R1523000): one row; after a bump a fence on the older epoch finds nothing and on the current one its row';
end $$;
rollback;

\echo '### R1523000: a second epoch row is refused ###'
begin;
do $$
begin
  begin
    insert into app.seat_epoch (epoch) values (0);
    raise exception 'FAIL (R1523000): a second epoch row was written';
  exception when unique_violation then
    null;
  end;
  raise notice 'OK (R1523000): a second row is refused';
end $$;
rollback;

\echo '### R1523000: the server fences and bumps; no client role reads the epoch ###'
begin;
do $$
declare
  who     text;
  current bigint;
begin
  execute 'set local role service_role';
  if current_user <> 'service_role' then
    raise exception 'FAIL (R1523000): running as %, not service_role — SET LOCAL ROLE did not take', current_user;
  end if;
  update app.seat_epoch set epoch = epoch + 1 returning epoch into current;
  perform 1 from app.seat_epoch where epoch = current for share;
  if not found then
    raise exception 'FAIL (R1523000): service_role''s fence on the epoch it bumped found no row';
  end if;
  reset role;

  foreach who in array array['authenticated', 'anon'] loop
    execute format('set local role %I', who);
    if current_user <> who then
      raise exception 'FAIL (R1523000): running as %, not % — SET LOCAL ROLE did not take', current_user, who;
    end if;
    begin
      perform count(*) from app.seat_epoch;
      raise exception 'FAIL (R1523000): % read app.seat_epoch', who;
    exception when insufficient_privilege then
      null;
    end;
    begin
      update app.seat_epoch set epoch = epoch + 1;
      raise exception 'FAIL (R1523000): % bumped app.seat_epoch', who;
    exception when insufficient_privilege then
      null;
    end;
    reset role;
  end loop;

  raise notice 'OK (R1523000): service_role fences and bumps, and authenticated and anon neither read nor bump the epoch';
end $$;
rollback;
