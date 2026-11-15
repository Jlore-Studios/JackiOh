-- ============================================================================
-- Migration 0092: the daily games' player mail, counted by day (SPEC §9.13;
-- issue #3843)
-- ============================================================================
-- Serves SPEC §9.13 (telling a player it is their move). The "your move"
-- emails go through the same Resend account as the sign-in, password-reset
-- and alert mail, whose free plan sends 100 a day, so the server holds the
-- player mail to DAILY_PLAYER_EMAILS_PER_DAY a UTC day across every account
-- (crates/server/src/config.rs) and player mail never stops sign-in mail.
--
--   * public.daily_email_days -- one row per UTC day: `day` is the day's
--     start in epoch ms, as 0089's times are, and `sent` how many emails the
--     server took that day. Before each send the server takes one in a single
--     statement (an upsert that adds one only while `sent` is under the cap),
--     so the sweep and the after-move task together never pass the cap, and a
--     restart carries the day's count over; it gives one back after a send
--     that failed (src/daily/notify.rs).
--
-- TRUST BOUNDARY: a count alone, no account and no foreign key. Like
-- public.practice_claim_counts (0078) it has RLS on, no policy and no grant:
-- only service_role (BYPASSRLS) reads or writes it.
--
-- One row a day, so nothing purges it.
--
-- Apply order: ... -> 0089 (daily games) -> 0090 (lobbies) -> 0091 (2v2) ->
-- 0092 (this file).
-- Safe to re-apply: create-if-not-exists, comment-on, revoke.
-- ============================================================================

create table if not exists public.daily_email_days (
  day  bigint  primary key,
  sent integer not null default 0 constraint daily_email_days_sent_check check (sent >= 0)
);

comment on table public.daily_email_days is
  $$SPEC §9.13, #3843: how many daily games' "your move" emails the server took
  each UTC day (day: its start, epoch ms), held to DAILY_PLAYER_EMAILS_PER_DAY so
  player mail never spends the Resend day the sign-in and password mail share.
  The server's alone: RLS on, no policy, no grant.$$;

alter table public.daily_email_days enable row level security;

revoke all on public.daily_email_days from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.daily_email_days -- no SELECT, INSERT,
--                UPDATE or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the server's claims and releases.
-- ============================================================================
