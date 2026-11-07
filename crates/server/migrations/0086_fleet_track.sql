-- ============================================================================
-- Migration 0086: The Fleet track and the Starship cosmetics, the Jlorious
-- Starship among them (SPEC §14.2, §14.11, §14.15; R3335000, R3335001,
-- R3335002, R3335003, R2755000; issue #3335)
-- ============================================================================
-- Serves R3335000 to R3335003. Four account kinds dress every Starship an
-- account launches (`starshipHull`, `starshipLivery`, `starshipTrail`,
-- `starshipPieceSkin`), each with a default every account owns. The Fleet
-- track (§14.15) pays one Stinkyscape cosmetic at each of its levels, earned
-- only by playing Stinkyscape in the games the server records, with the ledger
-- reason 'track' (src/economy/grants.rs). `starship-hull-jlorious` is granted
-- as `card-back-jlorious` is: by a rated game that leaves a person in Jlorious
-- (src/api/ranked.rs, `commit_ranked_game`), with the reason 'jlorious'. This
-- file:
--
--   1. Writes the 35 new rows of public.cosmetics, as crates/shared/cosmetics.json
--      has them: the four Starship defaults, the Jlorious Starship and the
--      Fleet track's thirty rewards. `seed-catalog` upserts the whole registry
--      (0045), but `release` runs the migrations before it, and the grants'
--      foreign keys need the rows. A row seed-catalog already wrote is left as
--      it wrote it. public.cosmetics.kind has no closed set, so the new kinds
--      need no constraint.
--   2. Creates public.fleet_tracks and its append-only ledger
--      public.fleet_track_grants, as 0045 created set_tracks and
--      set_track_grants: one row per account, its XP and the level reached
--      (from 0 to FLEET_TRACK_LEVELS), each change a ledger row of the reason
--      'game' (a recorded game's XP) or 'dev'. A level's reward is a row of
--      public.cosmetic_grants (reason 'track', which 0050 already allows), so
--      the Fleet ledger records no reward.
--   3. Grants the Jlorious Starship to every person who has held Jlorious
--      before this file, as 0074 granted the card back: a human profile (a
--      ladder bot is never Jlorious, R640000) with a season row whose
--      peak_jlorious is set (R607), in public.account_cosmetics with its
--      public.cosmetic_grants row: no card, +1, reason 'jlorious' and ref
--      'jlorious:0086'. One who owns it already is passed over and gets no
--      ledger row, so each ledger still sums to what is held (§14.11).
--
-- ACCESS: the two new tables are readable by their own account alone (RLS
-- select-own, as set_tracks), and only the server (service_role, BYPASSRLS;
-- src/economy/store.rs) writes them. No data is changed or lost: this file
-- only ADDS rows and objects.
--
-- Apply order: ... -> 0022 (season_ranks.peak_jlorious) -> ... -> 0045
-- (public.cosmetics, account_cosmetics, cosmetic_grants, set_tracks) -> ... ->
-- 0050 (cosmetic_grants' reasons) -> ... -> 0074 (the Jlorious card back) ->
-- ... -> 0084 -> 0085 -> 0086 (this file). Safe to re-apply: every statement is
-- insert-on-conflict-do-nothing, create-if-not-exists or a policy, trigger or
-- constraint dropped and created again, and a second grant finds every holder.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The 35 rows of the registry's projection (R3335000, R3335001, R3335003).
-- ----------------------------------------------------------------------------
-- MIRRORS crates/shared/cosmetics.json (its last 35 entries).
insert into public.cosmetics (id, kind, name, rarity, is_default, chase)
values ('starship-hull-standard', 'starshipHull', 'Its name''s hull', 'Common', true, false),
       ('starship-livery-standard', 'starshipLivery', 'The hull''s own colors', 'Common', true, false),
       ('starship-trail-standard', 'starshipTrail', 'The hull''s own burn', 'Common', true, false),
       ('starship-piece-skin-standard', 'starshipPieceSkin', 'Each module''s own colors', 'Common', true, false),
       ('starship-hull-jlorious', 'starshipHull', 'Jlorious', 'Mythic', false, false),
       ('starship-livery-hazard', 'starshipLivery', 'Hazard stripes', 'Epic', false, false),
       ('starship-trail-ion', 'starshipTrail', 'Ion wake', 'Epic', false, false),
       ('starship-hull-scout', 'starshipHull', 'Scout', 'Epic', false, false),
       ('starship-piece-skin-gunmetal', 'starshipPieceSkin', 'Gunmetal modules', 'Epic', false, false),
       ('starship-livery-nebula', 'starshipLivery', 'Nebula', 'Epic', false, false),
       ('starship-trail-plasma', 'starshipTrail', 'Plasma burn', 'Epic', false, false),
       ('starship-hull-courier', 'starshipHull', 'Courier', 'Epic', false, false),
       ('card-back-stinkyscape', 'cardBack', 'Stinkyscape', 'Epic', false, false),
       ('starship-livery-rust', 'starshipLivery', 'Rust bucket', 'Epic', false, false),
       ('starship-trail-comet', 'starshipTrail', 'Comet tail', 'Epic', false, false),
       ('starship-hull-freighter', 'starshipHull', 'Freighter', 'Epic', false, false),
       ('starship-piece-skin-verdigris', 'starshipPieceSkin', 'Verdigris modules', 'Epic', false, false),
       ('starship-livery-arctic', 'starshipLivery', 'Arctic', 'Epic', false, false),
       ('starship-trail-stardust', 'starshipTrail', 'Stardust', 'Legendary', false, false),
       ('starship-hull-corsair', 'starshipHull', 'Corsair', 'Legendary', false, false),
       ('table-stinkyscape', 'table', 'Stinkyscape', 'Epic', false, false),
       ('starship-livery-midnight', 'starshipLivery', 'Midnight', 'Legendary', false, false),
       ('starship-trail-aurora', 'starshipTrail', 'Aurora trail', 'Legendary', false, false),
       ('starship-hull-cathedral', 'starshipHull', 'Cathedral', 'Legendary', false, false),
       ('starship-piece-skin-neon', 'starshipPieceSkin', 'Neon modules', 'Legendary', false, false),
       ('starship-livery-coral', 'starshipLivery', 'Coral reef', 'Legendary', false, false),
       ('starship-trail-prism', 'starshipTrail', 'Prism trail', 'Legendary', false, false),
       ('starship-hull-leviathan', 'starshipHull', 'Leviathan', 'Legendary', false, false),
       ('field-stinkyscape', 'field', 'Stinkyscape', 'Epic', false, false),
       ('starship-livery-supernova', 'starshipLivery', 'Supernova', 'Mythic', false, false),
       ('starship-trail-singularity', 'starshipTrail', 'Singularity', 'Mythic', false, false),
       ('starship-hull-ark', 'starshipHull', 'Ark', 'Mythic', false, false),
       ('starship-piece-skin-chrome', 'starshipPieceSkin', 'Chrome modules', 'Mythic', false, false),
       ('starship-livery-flagship', 'starshipLivery', 'Flagship colors', 'Mythic', false, false),
       ('starship-hull-stinkyscape', 'starshipHull', 'Stinkyscape', 'Mythic', false, false)
on conflict (id) do nothing;


-- ----------------------------------------------------------------------------
-- 2. The Fleet track and its ledger (§14.15, §14.11; R3335001).
-- ----------------------------------------------------------------------------
-- A track starts at level 0 with no XP; no row reads as that. Every
-- FLEET_TRACK_XP_PER_LEVEL XP is a level, up to FLEET_TRACK_LEVELS.
create table if not exists public.fleet_tracks (
  profile_id uuid not null primary key references public.profiles(id) on delete cascade,
  xp         bigint not null default 0 check (xp >= 0),
  -- The server holds it to crates/shared/src/config.rs FLEET_TRACK_LEVELS
  -- (src/economy/store.rs, `fleet_after`), so the constant retunes it alone.
  level      bigint not null default 0 check (level >= 0),
  updated_at timestamptz not null default now()
);

comment on table public.fleet_tracks is
  $$SPEC §14.15, R3335001: an account's Fleet track: its XP and the level it
  reached. Written only with a fleet_track_grants row, by the server.$$;

alter table public.fleet_tracks enable row level security;

drop trigger if exists fleet_tracks_set_updated_at on public.fleet_tracks;
create trigger fleet_tracks_set_updated_at
  before update on public.fleet_tracks
  for each row execute function app.set_updated_at();

drop policy if exists fleet_tracks_select_own on public.fleet_tracks;
create policy fleet_tracks_select_own on public.fleet_tracks for select to authenticated
  using (profile_id = app.current_profile_id());

create table if not exists public.fleet_track_grants (
  id         bigint generated always as identity primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  xp         bigint not null default 0,
  levels     bigint not null default 0,
  reason     text not null,
  ref        text,
  dev        boolean not null default false,
  at         timestamptz not null default now(),
  constraint fleet_track_grants_change_check check (xp <> 0 or levels <> 0),
  constraint fleet_track_grants_reason_check check (reason in ('game', 'dev')),
  constraint fleet_track_grants_dev_check check (reason <> 'dev' or dev)
);

comment on table public.fleet_track_grants is
  $$SPEC §14.11, R3335001: append-only, one row per change of a Fleet track:
  the signed XP and levels it added (a recorded game's XP, a dev account's
  grant or clear). The reward a level pays is a cosmetic_grants row.$$;

create index if not exists fleet_track_grants_profile_idx on public.fleet_track_grants (profile_id, id);
alter table public.fleet_track_grants enable row level security;
drop trigger if exists fleet_track_grants_deny_mutation on public.fleet_track_grants;
create trigger fleet_track_grants_deny_mutation before update or delete on public.fleet_track_grants
  for each row execute function app.deny_row_mutation('profile_id');
drop policy if exists fleet_track_grants_select_own on public.fleet_track_grants;
create policy fleet_track_grants_select_own on public.fleet_track_grants for select to authenticated
  using (profile_id = app.current_profile_id());

-- Default-deny first, then the one read back, as 0045's tables.
revoke all on public.fleet_tracks, public.fleet_track_grants from public, anon, authenticated;
grant select on public.fleet_tracks, public.fleet_track_grants to authenticated;


-- ----------------------------------------------------------------------------
-- 3. Every person who has held Jlorious owns the Jlorious Starship (R3335003,
--    R607), as 0074 gave them the card back.
-- ----------------------------------------------------------------------------
-- Each owned row and its ledger row are written by one statement: a ledger
-- row only for a row actually inserted.
with granted as (
  insert into public.account_cosmetics (profile_id, cosmetic_id)
  select distinct r.profile_id, 'starship-hull-jlorious'
    from public.season_ranks r
    join public.profiles p on p.id = r.profile_id
   where r.peak_jlorious is not null
     and p.kind = 'human'
  on conflict (profile_id, cosmetic_id) do nothing
  returning profile_id, cosmetic_id
)
insert into public.cosmetic_grants (profile_id, card_id, cosmetic_id, delta, reason, ref)
select g.profile_id, null, g.cosmetic_id, 1, 'jlorious', 'jlorious:0086'
  from granted g;
