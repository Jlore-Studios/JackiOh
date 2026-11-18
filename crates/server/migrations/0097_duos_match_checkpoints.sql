-- ============================================================================
-- Migration 0097: live co-op checkpoints (issue #4169) -- a co-op or 2v2
-- rebuild folds only the log's tail
-- ============================================================================
-- Serves SPEC §9.3 (a live match's checkpoint, held to its checks) and §13.9
-- (a crashed co-op actor rebuilds by folding (seed, options, log)).
--
--   * public.duos_match_checkpoints -- one row per live co-op or 2v2 match at
--     most: the DuosState its actor held after the log row `seq` (logged
--     under `nonce`), written by the server as the restart drain stops the
--     actor and every MATCH_CHECKPOINT_MIN_ACTIONS-th row while the match is
--     played (src/duos/checkpoint.rs). The next rebuild starts from it and
--     folds only the rows after it, when every check a 1v1 checkpoint passes
--     (0042) passes: the same checkpoint version, rules fingerprint (`rules`)
--     and catalog, its row still logged under its nonce, and `body_hash`
--     (FNV-1a over `body`'s exact text, which is why `body` is text).
--     Otherwise it folds the whole log, as before this migration.
--     The server writes a row only while the match's `ended_at` is null and
--     its log holds that row, replaces it only with a later seq, and deletes
--     it when the match ends; a deleted match's goes with it (on delete
--     cascade). The log stays whole and authoritative: no replay (B41) reads
--     this.
--
-- A table of its own rather than 0042's: public.match_checkpoints' rows
-- reference public.matches, and a co-op match is a public.duos_matches row.
--
-- TRUST BOUNDARY (0039's): the body holds all four hands and libraries,
-- exactly what public.duos_matches and public.duos_match_actions guard, so
-- like them it has RLS on, no policy and no grant: only service_role
-- (BYPASSRLS) reads or writes it.
--
-- Apply order: ... -> 0096 -> 0097 (this file).
-- Safe to re-apply: create-if-not-exists, comment-on, revoke.
-- ============================================================================

create table if not exists public.duos_match_checkpoints (
  match_id           uuid primary key references public.duos_matches(id) on delete cascade,
  seq                bigint not null check (seq >= 1),
  nonce              text not null,
  catalog_version    text not null,
  rules              text not null,
  checkpoint_version integer not null,
  body               text not null,
  body_hash          text not null,
  at                 timestamptz not null default now()
);

comment on table public.duos_match_checkpoints is
  $$#4169, SPEC §9.3, §13.9: a live co-op or 2v2 match's state after its log
  row `seq`, which a rebuild resumes from when its checks pass, folding only
  the rows after it. Written by the server's restart drain and every
  MATCH_CHECKPOINT_MIN_ACTIONS rows of play, deleted when the match ends.
  Holds hidden information: RLS on, no policy, no grant.$$;

alter table public.duos_match_checkpoints enable row level security;

revoke all on public.duos_match_checkpoints from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.duos_match_checkpoints -- no SELECT,
--                INSERT, UPDATE or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the server's writes and reads.
-- ============================================================================
