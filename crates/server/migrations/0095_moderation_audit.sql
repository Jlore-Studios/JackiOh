-- ============================================================================
-- Migration 0095: the moderation levers write the dev dashboard's audit
-- (SPEC §9.4, §14.10; R3074000; issue #4141)
-- ============================================================================
-- Each lever of `jackioh-server moderate` (src/cli/moderate.rs) that changes
-- something writes one public.admin_actions row, with no profile, in the
-- transaction that pulls it. 0080's admin_actions_route_check took only
-- `<METHOD> /api/(dev|admin)/...`; this widens it by the four lever routes,
-- named exactly (an allow list, as src/admin/store.rs's check_route holds
-- MODERATE_ROUTES).
--
-- TRUST BOUNDARY: unchanged; no client role reads or writes admin_actions
-- (0080). A revoke-code row holds the code's stored hash, never the code.
--
-- Apply order: ... -> 0080 (dev dashboard) -> ... -> 0094 (revoke on profile
-- activated) -> 0095 (this file).
-- Safe to re-apply: the check is dropped if it exists and made again, and it
-- takes every row 0080's took.
-- ============================================================================

alter table public.admin_actions drop constraint if exists admin_actions_route_check;
alter table public.admin_actions
  add constraint admin_actions_route_check check (
    route ~ '^(GET|POST|PUT|PATCH|DELETE) /api/(dev|admin)/'
    or route in ('CLI moderate ban', 'CLI moderate unban', 'CLI moderate revoke-code',
                 'CLI moderate redemption')
  );

comment on table public.admin_actions is
  $$R3074000: one row per dev or admin action (§14.10), and per moderation lever
  pulled (0095, with no profile): who, which route, the request, the values
  before and after, and when. Append-only; the server alone writes and reads
  it, and the dev dashboard reads it through the server. An account's deletion
  forgets who, and keeps the row.$$;
