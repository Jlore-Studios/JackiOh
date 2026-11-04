-- ============================================================================
-- Migration 0080: Card proficiency, one row per account and card
-- (SPEC §14.14; R3013000, R3013001; issue #3013)
-- ============================================================================
-- Serves R3013000. An account's proficiency of a card is how often it has
-- played that card from hand in a game the server pays rewards for (ladder,
-- unranked, Conquest; §14.8), and the level those plays reached on the curve
-- of PROFICIENCY_LEVEL_PLAYS (src/config.rs). The server counts the plays
-- from its own log and writes them in the same transaction that records the
-- game (src/economy/proficiency.rs). Each level unlocks one stamp, a marker
-- the account may wear on that card, at most one per card (R3013001):
-- `PUT /api/proficiency/stamp` (src/api/proficiency.rs). This file:
--
--   1. Creates public.card_proficiency: the plays, the level and the stamp
--      worn, current state with no ledger. A level and a stamp are progress
--      and a pick, no holding (§14.11), so no grant records them. The card is
--      named by its id (a slug), with no foreign key to public.cards: the
--      catalog is data, as for card_cosmetic_picks (0061). A row goes only
--      with its account (0012's rule, `on delete cascade`). Kept apart from
--      card_cosmetic_picks and the cosmetics registry's versions and augments
--      (0079), which another issue rebuilds.
--
-- TRUST BOUNDARY, as card_cosmetic_picks (0061): a profile reads its own rows
-- through the Data API and writes none; the server (service_role, BYPASSRLS;
-- src/economy/store.rs) is the only writer.
--
-- Apply order: ... -> 0045 (the economy) -> ... -> 0061 (card picks) -> ...
-- -> 0078 -> 0079 -> 0080 (this file). This file only ADDS objects. Safe to
-- re-apply: create-if-not-exists, drop-policy-if-exists then create.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. public.card_proficiency (R3013000, R3013001).
-- ----------------------------------------------------------------------------
create table if not exists public.card_proficiency (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  card_id    text not null,
  plays      bigint not null,
  level      bigint not null,
  stamp      bigint,
  updated_at timestamptz not null default now(),
  primary key (profile_id, card_id),
  constraint card_proficiency_card_check check (card_id ~ '^[a-z][a-z0-9-]{0,39}$'),
  constraint card_proficiency_plays_check check (plays > 0),
  constraint card_proficiency_level_check check (level >= 0),
  -- R3013001: level n unlocks stamp n, so a worn stamp is one the row's level reached.
  constraint card_proficiency_stamp_check check (stamp is null or (stamp >= 1 and stamp <= level))
);

comment on table public.card_proficiency is
  $$R3013000: how often an account played one card from hand in games that
  pay rewards, and the proficiency level those plays reached (§14.14), one row
  per account and card. R3013001: the stamp the card wears, one its level
  unlocked, or NULL for none. Current state, no ledger. A client reads its own
  rows and writes none.$$;

alter table public.card_proficiency enable row level security;

-- SPEC §9.1: a profile reads only its own rows, as player_settings (0018). No
-- insert/update/delete policy exists for any client role.
drop policy if exists card_proficiency_select_own on public.card_proficiency;
create policy card_proficiency_select_own
  on public.card_proficiency
  for select
  to authenticated
  using (profile_id = app.current_profile_id());

revoke all on public.card_proficiency from public, anon, authenticated;
grant select on public.card_proficiency to authenticated;
