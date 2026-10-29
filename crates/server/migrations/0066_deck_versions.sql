-- ============================================================================
-- Migration 0066: a saved deck keeps its earlier versions (issue #2100,
-- R2100000)
-- ============================================================================
-- Serves SPEC §9.4 (R250, R256) and R2100000.
--
-- Each save of a deck keeps a version of what it saved: the deck's name and
-- cards, at the instant of the save. Its portrait and its cosmetic picks are
-- no part of a version (R2100000). The server writes every row
-- (`deck_versions_keep`, src/db/pg.rs), under the rules the store holds for
-- both stores (`deck_version_step`, src/db/store.rs):
--   * a save within DECK_VERSION_DEBOUNCE_MS of the deck's newest version
--     replaces that version, so a burst of edits keeps one;
--   * a save whose name and cards are the newest version's keeps none;
--   * a deck keeps at most DECK_VERSIONS_KEPT versions, the oldest dropped;
--   * a restore keeps the version it restores as a new one, never replacing.
-- Both numbers are the server's (crates/server/src/config.rs), never written
-- here.
--
-- A version belongs to its deck: the composite foreign key on (profile_id,
-- deck_id) names only a deck of the same profile, as the trios' slots do
-- (0007), and cascades, so deleting a deck deletes its versions, and deleting
-- an account (0012: profiles cascade to decks) deletes them too.
--
-- A deck saved before this file starts with one version, itself as it was
-- last saved (its name, its cards and its updated_at), so its History opens
-- on what it holds.
--
-- TRUST BOUNDARY: as public.decks (0007). RLS on, one select-own policy to
-- authenticated, SELECT its only client grant, and no write path for any
-- client role (SPEC §9.1: "Server validates and stores | Propose").
--
-- Apply order: 0007 (decks) -> ... -> 0065 -> 0066 (this file).
-- This file only ADDS objects and rows. Safe to re-apply: create-if-not-
-- exists, drop-policy-if-exists then create, and the seed adds a version only
-- to a deck that has none.
-- ============================================================================

create table if not exists public.deck_versions (
  id         uuid        primary key,
  profile_id uuid        not null,
  deck_id    uuid        not null,
  name       text        not null,
  cards      jsonb       not null,
  saved_at   timestamptz not null,
  constraint deck_versions_cards_is_array_check check (jsonb_typeof(cards) = 'array'),
  constraint deck_versions_deck_fk foreign key (profile_id, deck_id)
    references public.decks (profile_id, id) on delete cascade
);

comment on table public.deck_versions is
  $$R2100000: the earlier versions of a profile's saved decks, each the name
  and cards one save wrote, at most DECK_VERSIONS_KEPT per deck. A save within
  DECK_VERSION_DEBOUNCE_MS of the newest replaces it; a restore always adds
  one. Deleted with the deck. Written only by the server; a client reads its
  own.$$;

comment on column public.deck_versions.saved_at is
  $$R2100000: the save this version holds, or the last of the burst it
  replaced. The list order, newest first: (saved_at desc, id desc).$$;

-- The one read path, a deck's versions newest first, in order without a sort;
-- it leads with the foreign key's columns, so a deck's delete finds its
-- versions through it (01_schema_invariants.sql, CHECK 19).
create index if not exists deck_versions_deck_saved_idx
  on public.deck_versions (profile_id, deck_id, saved_at desc, id desc);

alter table public.deck_versions enable row level security;

-- SPEC §9.1: a profile reads only its own rows, as public.decks. No
-- insert/update/delete policy exists for any client role.
drop policy if exists deck_versions_select_own on public.deck_versions;
create policy deck_versions_select_own
  on public.deck_versions
  for select
  to authenticated
  using (profile_id = app.current_profile_id());

revoke all on public.deck_versions from public, anon, authenticated;
grant select on public.deck_versions to authenticated;

-- R2100000: a deck saved before 0066 starts with one version, itself as last
-- saved.
insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
select gen_random_uuid(), d.profile_id, d.id, d.name, d.cards, d.updated_at
  from public.decks d
 where not exists (select 1 from public.deck_versions v
                    where v.profile_id = d.profile_id and v.deck_id = d.id);

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: SELECT its own deck_versions rows; no INSERT, UPDATE or
--                DELETE, and no write policy.
-- service_role:  BYPASSRLS covers every save, restore and read.
-- ============================================================================
