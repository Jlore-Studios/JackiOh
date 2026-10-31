-- ============================================================================
-- Migration 0071: a practice claim kept until it settles (issue #2422) -- a
-- claim burned but never paid is paid later, once
-- ============================================================================
-- Serves SPEC §14.8 (a claim that passes is paid; a seed pays once) for a
-- pay that never landed: every try the store failed (R1688000), or the server
-- stopped or crashed between the burn and the pay (R2422000).
--
--   * public.practice_seeds gains three columns:
--       - `claim` -- the claim as it was sent (its result and the game's
--         log), written in the transaction that burns the seed, so a claim
--         whose server went away can be checked again. Cleared once the
--         claim settles.
--       - `settled`, `settled_at` -- how the claim settled, `paid` or
--         `refused`, and when. The pay writes `paid` in the rewards' own
--         transaction, and only where the seed has no mark yet, so a pay
--         lands once whoever runs it, the live claim or the retention
--         loop's sweep; a refusal writes `refused` in a transaction of its
--         own.
--     The retention loop settles any claim still unsettled
--     PRACTICE_SETTLE_GRACE_SECONDS after `claimed_at` (src/api/practice.rs,
--     `sweep_unsettled`), and the prune keeps such a claim until it settles.
--   * practice_seeds_unsettled -- the sweep's partial index: the claims kept
--     and unsettled, by `claimed_at`.
--   * practice_seeds_settled_check -- a mark comes with its time and is one
--     of the two, and only a claimed seed holds a claim or a mark.
--
-- No row is rewritten. A seed claimed before this file holds no claim, so
-- nothing settles it, and the prune deletes it as it did before.
--
-- TRUST BOUNDARY: 0047's. The claim is what a player sent and the mark is
-- what was paid, so the table stays the server's alone: RLS on, no policy,
-- no grant; only service_role (BYPASSRLS) reads or writes it.
--
-- Apply order: ... -> 0047 (public.practice_seeds) -> ... -> 0070 -> 0071
-- (this file).
-- Safe to re-apply: add-column-if-not-exists, drop-constraint-if-exists-then-
-- add, create-index-if-not-exists, comment-on.
-- ============================================================================

alter table public.practice_seeds
  add column if not exists claim      jsonb,
  add column if not exists settled    text,
  add column if not exists settled_at timestamptz;

alter table public.practice_seeds drop constraint if exists practice_seeds_settled_check;
alter table public.practice_seeds add constraint practice_seeds_settled_check check (
  (settled is null) = (settled_at is null)
  and (settled is null or settled in ('paid', 'refused'))
  and (claimed_at is not null or (claim is null and settled is null))
);

create index if not exists practice_seeds_unsettled
  on public.practice_seeds (claimed_at) where claim is not null and settled_at is null;

comment on column public.practice_seeds.claim is
  $$R2422000: the claim as it was sent, { result, log }, written when it burned
  the seed and cleared once it settles, so a claim whose pay never landed is
  checked again and paid once by the retention loop.$$;

comment on column public.practice_seeds.settled is
  $$R2422000: how the claim settled, paid or refused; null while it is
  unsettled. The pay sets it in its own transaction, only where it is null.$$;

comment on column public.practice_seeds.settled_at is
  $$R2422000: when the claim settled; null exactly while `settled` is.$$;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.practice_seeds -- no SELECT, INSERT,
--                UPDATE or DELETE, and no policy (0047's, unchanged).
-- service_role:  BYPASSRLS covers the server's writes, reads and the sweep.
-- ============================================================================
