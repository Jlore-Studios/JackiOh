-- ============================================================================
-- Migration 0042: live-match checkpoints (issue #1169) -- a rebuild folds only
-- the log's tail
-- ============================================================================
-- Serves SPEC §9.3 ("snapshots ... while the log stays") and §9.5 (a crashed
-- actor rebuilds by folding (seed, log)).
--
--   * public.match_checkpoints -- one row per live match at most: the engine
--     state, the play telemetry (R1442) and the bot turns (R639000) an actor
--     held after the log row `seq` (logged under `nonce`), written by the
--     server as the restart drain stops the actor (src/actor/checkpoint.rs).
--     The next rebuild starts from it and folds only the rows after it, when
--     every check passes: the same checkpoint version, rules fingerprint
--     (`rules`) and catalog, its row still logged under its nonce, and
--     `body_hash` (FNV-1a over `body`'s exact text, which is why `body` is
--     text). Otherwise it folds the whole log, as before this migration.
--     The server writes a row only while the match is live and its log holds
--     that row, replaces it only with a later seq, and deletes it when the
--     match ends; a voided match's goes with the match (on delete cascade).
--     The log stays whole and authoritative: no replay (R768) reads this.
--
-- TRUST BOUNDARY: the body holds both hands and both libraries, exactly what
-- public.matches and public.match_actions guard (0004), so like them it has
-- RLS on, no policy and no grant: only service_role (BYPASSRLS) reads or
-- writes it.
--
-- Apply order: ... -> 0041 -> 0042 (this file).
-- Safe to re-apply: create-if-not-exists, comment-on, revoke.
-- ============================================================================

create table if not exists public.match_checkpoints (
  match_id           uuid primary key references public.matches(id) on delete cascade,
  seq                bigint not null check (seq >= 1),
  nonce              text not null,
  catalog_version    text not null,
  rules              text not null,
  checkpoint_version integer not null,
  body               text not null,
  body_hash          text not null,
  at                 timestamptz not null default now()
);

comment on table public.match_checkpoints is
  $$#1169, SPEC §9.3, §9.5: a live match's state after its log row `seq`, which
  a rebuild resumes from when its checks pass, folding only the rows after it.
  Written by the server's restart drain, deleted when the match ends. Holds
  hidden information: RLS on, no policy, no grant.$$;

alter table public.match_checkpoints enable row level security;

revoke all on public.match_checkpoints from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.match_checkpoints -- no SELECT, INSERT,
--                UPDATE or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the server's writes and reads.
-- ============================================================================
