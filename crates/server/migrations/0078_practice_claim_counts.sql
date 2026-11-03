-- ============================================================================
-- Migration 0078: practice claim counts (issue #2884) -- how each claim ended
-- ============================================================================
-- Serves SPEC §14.8 (a practice game pays once the server has checked it): a
-- count of how practice claims ended, so the seed TTL and the abuse surface
-- (§9.8) are tuned on numbers. It changes no claim's answer.
--
--   * public.practice_claim_counts -- one row per (UTC day, outcome): how
--     many claims ended that day as `paid` or as one refusal, its reason as
--     the claim's answer names it (src/economy/practice.rs, `ClaimBucket`).
--     The server adds one in the transaction that decides the outcome, as
--     that transaction's last statement (src/api/practice.rs): a claim whose
--     seed it burned counts once, in the settle that marks the seed paid or
--     refused (R2422000); a refusal before the burn counts once per request.
--     `expired` measures PRACTICE_SEED_TTL_SECONDS (R1303001); `does_not_fold`
--     is what a forged log answers.
--
-- TRUST BOUNDARY: aggregates alone, no account, seed or log, and no foreign
-- key. Like public.practice_seeds (0047) it has RLS on, no policy and no
-- grant: only service_role (BYPASSRLS) reads or writes it, and only the
-- offline export (`jackioh-server stats-export --claims`) reads it.
--
-- Nine rows a day at most, so nothing purges it.
--
-- Apply order: ... -> 0071 (practice settles) -> ... -> 0077 (news claims) ->
-- 0078 (this file).
-- Safe to re-apply: create-if-not-exists, comment-on, revoke.
-- ============================================================================

create table if not exists public.practice_claim_counts (
  day     date   not null,
  outcome text   not null constraint practice_claim_counts_outcome_check check (outcome in
            ('paid', 'economy_off', 'unknown_seed', 'already_claimed', 'expired',
             'does_not_fold', 'not_finished', 'voided', 'result_mismatch')),
  count   bigint not null constraint practice_claim_counts_count_check check (count > 0),
  primary key (day, outcome)
);

comment on table public.practice_claim_counts is
  $$#2884: how many practice claims ended each UTC day as paid or as each
  refusal, for tuning the seed TTL and spotting forged logs. Aggregates
  alone, no account. Written by the server alone: RLS on, no policy, no
  grant.$$;

alter table public.practice_claim_counts enable row level security;

revoke all on public.practice_claim_counts from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.practice_claim_counts -- no SELECT,
--                INSERT, UPDATE or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the server's counts and the export's read.
-- ============================================================================
