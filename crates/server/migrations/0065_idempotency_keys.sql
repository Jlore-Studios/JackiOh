-- ============================================================================
-- Migration 0065: idempotency keys (issue #2124) -- a retried spend runs once
-- ============================================================================
-- Serves SPEC §14.11 (every spend is written in the transaction that changes
-- the state) for a client that gave up on its answer (R2124000).
--
--   * public.idempotency_keys -- one row per (account, Idempotency-Key) a
--     spend was sent with and committed under: the route and a keyed hash of
--     the body it came with, and the status and body it answered. The server
--     writes the row in the spend's own transaction, under the account's
--     economy lock (R1301003), so a grant never commits without its answer;
--     the same key again with the same route and body replays the answer,
--     and with another is refused (src/economy/idempotency.rs). A refusal
--     writes no row. The retention loop deletes a row
--     IDEMPOTENCY_KEY_TTL_HOURS after `created_at`.
--
-- TRUST BOUNDARY: a row is one account's answer, and the key is what lets a
-- request replay it, so a client that read the table could replay another's
-- spend and one that wrote it could forge an answer. Like
-- public.practice_seeds (0047) it has RLS on, no policy and no grant: only
-- service_role (BYPASSRLS) reads or writes it.
--
-- Account deletion (0012's rule): every row goes with its profile
-- (`on delete cascade`).
--
-- Apply order: ... -> 0064 -> 0065 (this file).
-- Safe to re-apply: create-if-not-exists, comment-on, revoke.
-- ============================================================================

create table if not exists public.idempotency_keys (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  key        uuid not null,
  route      text not null,
  body_hash  text not null check (body_hash ~ '^[0-9a-f]{64}$'),
  status     smallint not null check (status between 200 and 299),
  body       jsonb not null,
  created_at timestamptz not null,
  primary key (profile_id, key)
);

create index if not exists idempotency_keys_created_at
  on public.idempotency_keys (created_at);

comment on table public.idempotency_keys is
  $$#2124, R2124000: a spend's Idempotency-Key and the answer it committed with,
  per account, kept IDEMPOTENCY_KEY_TTL_HOURS so a retry replays the answer and
  spends nothing again. Written by the server alone: RLS on, no policy, no
  grant.$$;

alter table public.idempotency_keys enable row level security;

revoke all on public.idempotency_keys from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.idempotency_keys -- no SELECT, INSERT,
--                UPDATE or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the server's writes, reads and the sweep.
-- ============================================================================
