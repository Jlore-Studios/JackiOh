-- ============================================================================
-- Migration 0052: the seat ledger's epoch (SPEC §9.5; R1523000; issue #1523)
-- ============================================================================
-- Serves R1523000: one seat ledger in the server holds every profile's game
-- (crates/server/src/seats.rs), and the in-match flag is its record and a
-- second guard. A deploy runs two servers side by side for a short time, each
-- with its own ledger, so every claiming transaction first takes this table's
-- one row FOR SHARE while it still holds the epoch its server booted on
-- (`seats_fence`, src/db/pg.rs). A booting server bumps the epoch before it
-- fills its ledger from the database (`seats_bump_epoch`): the bump waits for
-- every claim in flight, so the new ledger reads every seat the old server
-- committed, and a claim the old server makes after it finds no row and writes
-- nothing. Shared locks never wait on each other, so claims never queue behind
-- one another here.
--
-- A row lock lasts one transaction. A session advisory lock would stay held on
-- a pooled backend behind Supavisor's transaction-mode pooler
-- (docs/architecture.md), so none is used.
--
-- TRUST BOUNDARY: server only. No client role reads or writes the table; the
-- server's service_role selects it FOR SHARE (which takes UPDATE as well as
-- SELECT) and bumps it.
--
-- Apply order: ... -> 0050 -> 0051 -> 0052 (this file).
-- Safe to re-apply: create-if-not-exists, an insert that skips an existing
-- row, and idempotent grants.
-- ============================================================================

create table if not exists app.seat_epoch (
  epoch bigint not null
);

comment on table app.seat_epoch is
  'R1523000: the seat ledger''s epoch, one row. A booting server bumps it before '
  'it fills its ledger; every claim takes the row FOR SHARE while it holds the '
  'epoch its server booted on, and writes nothing once it does not.';

-- One row, ever: a second could hold another epoch and fence nothing.
create unique index if not exists seat_epoch_one_row on app.seat_epoch ((true));

insert into app.seat_epoch (epoch)
select 0
 where not exists (select 1 from app.seat_epoch);

revoke all on app.seat_epoch from public, anon, authenticated;
grant select, update on app.seat_epoch to service_role;
