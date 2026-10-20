-- ============================================================================
-- Migration 0046: practice seeds (issue #1303) -- a practice game earns once
-- the server has checked it
-- ============================================================================
-- Serves SPEC §14.8 ("Practice pays half rates ... the server checks the
-- game itself") and §9.9 (practice runs in the browser).
--
--   * public.practice_seeds -- one row per seed the server issued an
--     account for a practice game, with the game's setup (`setup`: the AI
--     tier, the human's seat, its deck, the last board it brought and the
--     catalog version), when it was issued and when it was claimed. The
--     server folds a claim's log from this setup and pays it once
--     (src/economy/practice.rs, R1303000-R1303003): a claim burns the seed,
--     paid or refused (`claimed_at`), and a seed unclaimed after
--     PRACTICE_SEED_TTL_SECONDS lapses; the account's next issue deletes its
--     lapsed seeds.
--
-- TRUST BOUNDARY: a seed is what lets a claim pay, and the setup names the
-- AI's tier, so a client that read or wrote it could claim another's game or
-- shape its own. Like public.match_checkpoints (0042) it has RLS on, no
-- policy and no grant: only service_role (BYPASSRLS) reads or writes it.
--
-- Account deletion (0012's rule): every row goes with its profile
-- (`on delete cascade`).
--
-- Apply order: ... -> 0045 -> 0046 (this file).
-- Safe to re-apply: create-if-not-exists, comment-on, revoke.
-- ============================================================================

create table if not exists public.practice_seeds (
  seed       text primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  setup      jsonb not null,
  issued_at  timestamptz not null,
  claimed_at timestamptz
);

create index if not exists practice_seeds_profile_issued
  on public.practice_seeds (profile_id, issued_at);

comment on table public.practice_seeds is
  $$#1303, SPEC §14.8: a practice game's server-issued seed and its setup, which
  a claim's log is folded from and paid once (R1303001). Written by the server
  alone: RLS on, no policy, no grant.$$;

alter table public.practice_seeds enable row level security;

revoke all on public.practice_seeds from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.practice_seeds -- no SELECT, INSERT,
--                UPDATE or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the server's writes and reads.
-- ============================================================================
