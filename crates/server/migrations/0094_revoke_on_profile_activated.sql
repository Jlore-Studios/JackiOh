-- ============================================================================
-- Migration 0094: revoke the launch trigger's public execute (SPEC §9.4; #4012)
-- ============================================================================
-- Serves CHECK 25 of crates/server/tests/sql/01_schema_invariants.sql: a
-- function defaults to EXECUTE for PUBLIC, and app.on_profile_activated()
-- (0002, the pending-to-active launch-grant trigger) never had that default
-- revoked, so anon and authenticated could call privileged code directly.
-- Every other app.* SECURITY DEFINER function already revokes PUBLIC
-- execute; this one was missed because it fires from a trigger, not an RPC.
--
-- TRUST BOUNDARY: trigger firing needs no EXECUTE grant — Postgres fires
-- triggers as the trigger's runner regardless (precedent:
-- app.handle_new_user() has EXECUTE revoked from PUBLIC with no grant-back
-- at 0001_profiles_and_invites.sql:355 and CHECK 8 signup passes). The
-- service_role grant keeps the server's own path working wherever it calls
-- the function directly.
--
-- Apply order: ... -> 0092 (daily email days) -> 0093 (notifications) ->
-- 0094 (this file).
-- Safe to re-apply: revoke-if-held and grant-if-missing are both no-ops.
-- ============================================================================

revoke all on function app.on_profile_activated() from public;
grant execute on function app.on_profile_activated() to service_role;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing new: EXECUTE on app.on_profile_activated() is gone.
-- authenticated: nothing new: EXECUTE on app.on_profile_activated() is gone.
-- service_role:  EXECUTE on app.on_profile_activated(), explicitly, as with
--                every other app.* function the server calls.
-- ============================================================================
