-- ============================================================================
-- Migration 0046: Dev accounts (SPEC §14.10, R1300000, R1300001; issue #1300)
-- ============================================================================
-- Serves SPEC §14.10 (Dev accounts), quoted:
--   "Every dev grant and clear is written as ledger rows marked `dev`, so a
--   dev grant can always be told apart. R481's grant of each new catalog
--   version skips a dev account, so a clear stays cleared."
--
-- A dev account is one the server's DEV_PROFILE_IDS environment variable
-- lists; the list lives in the environment, not in a table, and the server
-- alone reads it. Two changes follow from it:
--
--   * public.collection_grants takes the reason 'dev' (R1300000): a dev
--     account's own fill and clear of its cards (crates/server/src/api/dev.rs),
--     always marked `dev`, as every economy ledger of 0045 holds its own 'dev'
--     reason (`collection_grants_dev_check`, as 0045's `<ledger>_dev_check`).
--   * app.grant_launch_collection_all, which 0016 runs on every catalog stamp
--     (R481), passes over the dev accounts it is handed (R1300001):
--     `seed-catalog` sets `jackioh.dev_profile_ids`, comma-separated profile
--     ids, for its own transaction before the stamp. A stamp without the
--     setting grants every active person, as before. The activation grant
--     (R111, app.on_profile_activated) is not changed.
--
-- Apply order: ... -> 0002 (`public.collection_grants`) -> ... -> 0016 (the
-- stamp's trigger) -> ... -> 0032 (`app.grant_launch_collection_all` over
-- people only) -> ... -> 0045 (`collection_grants.dev`) -> 0046 (this file).
-- Safe to re-apply: drop-constraint-if-exists before each add, create-or-replace
-- of the function and comment-on are all idempotent.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The collection's dev reason (R1300000).
-- ----------------------------------------------------------------------------
alter table public.collection_grants drop constraint if exists collection_grants_reason_check;
alter table public.collection_grants add constraint collection_grants_reason_check check (
  reason in ('pack', 'craft', 'reward', 'refund', 'admin', 'launch', 'dev'));

alter table public.collection_grants drop constraint if exists collection_grants_dev_check;
alter table public.collection_grants add constraint collection_grants_dev_check check (
  reason <> 'dev' or dev);


-- ----------------------------------------------------------------------------
-- 2. A new catalog version's grant passes over the dev accounts (R1300001).
-- ----------------------------------------------------------------------------
-- 0032's function, as it was, but over the people seed-catalog does not name
-- as dev accounts.
create or replace function app.grant_launch_collection_all()
returns int
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_count    int := 0;
  v_profile  record;
  -- R1300001: the dev accounts seed-catalog handed this transaction; none
  -- when the setting is unset or blank.
  v_dev      uuid[] := coalesce(
    string_to_array(nullif(current_setting('jackioh.dev_profile_ids', true), ''), ',')::uuid[],
    '{}'::uuid[]);
begin
  -- SPEC §9.6 build order step 3: "Collection ledger; grant everything to
  -- everyone." Backfill for every currently active person's profile; safe to
  -- run repeatedly because app.grant_launch_collection is itself idempotent.
  -- R1454: a ladder bot (kind 'bot') owns nothing. R1300001: a dev account
  -- keeps what it cleared.
  for v_profile in
    select id from public.profiles
     where status = 'active' and kind = 'human' and id <> all (v_dev)
  loop
    perform app.grant_launch_collection(v_profile.id);
    v_count := v_count + 1;
  end loop;

  return v_count;
end;
$$;

comment on function app.grant_launch_collection_all() is
  $$Backfill: calls app.grant_launch_collection for every active person's
  profile (kind human; a ladder bot owns no collection, R1454) but the dev
  accounts named by the transaction's jackioh.dev_profile_ids setting, which
  seed-catalog sets before its stamp (R1300001). Returns the number of
  profiles touched (visited), not the number that received new grants -- a
  repeat run touches the same count safely.$$;
