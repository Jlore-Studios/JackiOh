-- ============================================================================
-- Migration 0044: The economy's holdings and their ledgers (SPEC §14.11; issue #1299)
-- ============================================================================
-- Serves SPEC §14.11 (The ledger), quoted:
--   "Every holding of the economy follows that pattern, a current-state table
--   beside an append-only one, readable by its own account alone. Every grant
--   of anything (cards, Jlore, packs, tokens, cosmetics, XP) is one
--   append-only row, written server-side in the transaction that changes the
--   state; so is every spend, such as a pack bought or a token redeemed. A row
--   is never edited, and is deleted only with its account."
-- "That pattern" is the collection's (0002): `collection` beside the
-- append-only `collection_grants`. Each holding of §14 gets the same pair:
--
--   * public.wallets / public.jlore_grants -- §14.1's Jlore balance.
--   * public.inventory / public.inventory_grants -- unopened packs by kind
--     (a set's, the global one, a cosmetic pack; §14.3, §14.6) and card and
--     cosmetic tokens (§14.5), one count per (item, key).
--   * public.economy_counters / public.economy_counter_grants -- the pity
--     counters per rarity (§14.4), the two token bars' points and the
--     cosmetic token cycle's position (§14.5), and the Chase Shard bar
--     (§14.6), one value per (counter, pack).
--   * public.set_tracks / public.set_track_grants -- each set track's XP, the
--     level reached and the levels whose rewards were paid (§14.7).
--   * public.held_quests and public.quest_schedules / public.quest_grants --
--     the held quests, their progress and when each arrived, and when the
--     next is due (§14.9).
--   * public.card_cosmetics and public.account_cosmetics /
--     public.cosmetic_grants -- the per-card and account cosmetics owned
--     (§14.2), over public.cosmetics, the static projection of the
--     cosmetics registry (crates/server/cosmetics.json, R1295000) that
--     `seed-catalog` writes beside public.cards.
--
-- Every ledger row says why (its reason, §14.11) and whether a dev tool
-- wrote it (`dev`, §14.10): a `dev` reason is always marked, and a row of
-- another reason may be (a quest a dev account was given). collection_grants
-- gains the same `dev` column, false for every existing row.
--
-- TRUST BOUNDARY, exactly as the collection (0002): a profile reads its own
-- rows of every table here through the Data API and writes none; every
-- authenticated profile reads public.cosmetics, as public.cards. RLS default
-- deny is the enforcement: no client role holds an insert, update or delete
-- grant or policy. The server (service_role, BYPASSRLS; src/economy/store.rs)
-- is the only writer, and writes a holding and its ledger row in one
-- transaction. Each ledger is append-only by app.deny_row_mutation('profile_id'),
-- which lets its rows go only with their account (0012).
--
-- Account deletion (0012's rule): every row here goes with its profile
-- (`on delete cascade`). public.cosmetics names no profile and stays.
--
-- Nothing reads or writes these tables yet but their store methods: the
-- economy's parts build on them behind ECONOMY_LIVE (§14.12).
--
-- Apply order: ... -> 0042 -> 0043 -> 0044 (this file).
-- This file only ADDS objects. Safe to re-apply: create-if-not-exists,
-- add-column-if-not-exists, drop-and-create of triggers and policies,
-- comment-on, revoke and grant.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. Jlore (§14.1): the wallet and its ledger.
-- ----------------------------------------------------------------------------
create table if not exists public.wallets (
  profile_id uuid primary key references public.profiles(id) on delete cascade,
  jlore      bigint not null default 0 check (jlore >= 0),
  updated_at timestamptz not null default now()
);

comment on table public.wallets is
  $$SPEC §14.1: an account's Jlore, a whole number never below zero. No row
  reads as 0. Written only with a jlore_grants row, by the server.$$;

alter table public.wallets enable row level security;

drop trigger if exists wallets_set_updated_at on public.wallets;
create trigger wallets_set_updated_at
  before update on public.wallets
  for each row execute function app.set_updated_at();

drop policy if exists wallets_select_own on public.wallets;
create policy wallets_select_own on public.wallets for select to authenticated
  using (profile_id = app.current_profile_id());

create table if not exists public.jlore_grants (
  id         bigint generated always as identity primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  delta      bigint not null check (delta <> 0),
  reason     text not null,
  ref        text,
  dev        boolean not null default false,
  at         timestamptz not null default now(),
  constraint jlore_grants_reason_check check (reason in ('game', 'quest', 'track', 'pack_purchase', 'dev')),
  constraint jlore_grants_dev_check check (reason <> 'dev' or dev)
);

comment on table public.jlore_grants is
  $$SPEC §14.11: append-only, one row per change of wallets.jlore: a game's,
  a quest's or a track level's pay (§14.7-§14.9), a pack bought (§14.3), or a
  dev account's own grant or clear (§14.10). `ref` names what it was for.$$;

create index if not exists jlore_grants_profile_idx on public.jlore_grants (profile_id, id);
alter table public.jlore_grants enable row level security;
drop trigger if exists jlore_grants_deny_mutation on public.jlore_grants;
create trigger jlore_grants_deny_mutation before update or delete on public.jlore_grants
  for each row execute function app.deny_row_mutation('profile_id');
drop policy if exists jlore_grants_select_own on public.jlore_grants;
create policy jlore_grants_select_own on public.jlore_grants for select to authenticated
  using (profile_id = app.current_profile_id());


-- ----------------------------------------------------------------------------
-- 2. The inventory (§14.3, §14.5, §14.6): unopened packs and tokens.
-- ----------------------------------------------------------------------------
-- An item and its key: a set's card pack by the set (`set_pack`, 'Core'); the
-- global card pack and a cosmetic pack unkeyed; a card token unkeyed (any new
-- card, §14.5) or by a rarity; a cosmetic token by its rarity (§5's five).
create table if not exists public.inventory (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  item       text not null,
  key        text not null default '',
  count      bigint not null default 0 check (count >= 0),
  updated_at timestamptz not null default now(),
  primary key (profile_id, item, key),
  constraint inventory_key_check check (
    (item = 'set_pack' and key <> '')
    or (item in ('global_pack', 'cosmetic_pack') and key = '')
    or (item = 'card_token' and (key = '' or key in ('Common', 'Rare', 'Epic', 'Legendary', 'Mythic')))
    or (item = 'cosmetic_token' and key in ('Common', 'Rare', 'Epic', 'Legendary', 'Mythic'))
  )
);

comment on table public.inventory is
  $$SPEC §14.3, §14.5, §14.6: an account's unopened card and cosmetic packs and
  its unspent tokens, one count per (item, key). Written only with an
  inventory_grants row, by the server.$$;

alter table public.inventory enable row level security;

drop trigger if exists inventory_set_updated_at on public.inventory;
create trigger inventory_set_updated_at
  before update on public.inventory
  for each row execute function app.set_updated_at();

drop policy if exists inventory_select_own on public.inventory;
create policy inventory_select_own on public.inventory for select to authenticated
  using (profile_id = app.current_profile_id());

create table if not exists public.inventory_grants (
  id         bigint generated always as identity primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  item       text not null,
  key        text not null default '',
  delta      bigint not null check (delta <> 0),
  reason     text not null,
  ref        text,
  dev        boolean not null default false,
  at         timestamptz not null default now(),
  constraint inventory_grants_key_check check (
    (item = 'set_pack' and key <> '')
    or (item in ('global_pack', 'cosmetic_pack') and key = '')
    or (item = 'card_token' and (key = '' or key in ('Common', 'Rare', 'Epic', 'Legendary', 'Mythic')))
    or (item = 'cosmetic_token' and key in ('Common', 'Rare', 'Epic', 'Legendary', 'Mythic'))
  ),
  constraint inventory_grants_reason_check check (
    reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
               'chase_shard', 'dev')
  ),
  constraint inventory_grants_dev_check check (reason <> 'dev' or dev)
);

comment on table public.inventory_grants is
  $$SPEC §14.11: append-only, one row per change of an inventory count: a pack
  bought, earned or opened, a token paid by a bar or redeemed.$$;

create index if not exists inventory_grants_profile_idx on public.inventory_grants (profile_id, id);
alter table public.inventory_grants enable row level security;
drop trigger if exists inventory_grants_deny_mutation on public.inventory_grants;
create trigger inventory_grants_deny_mutation before update or delete on public.inventory_grants
  for each row execute function app.deny_row_mutation('profile_id');
drop policy if exists inventory_grants_select_own on public.inventory_grants;
create policy inventory_grants_select_own on public.inventory_grants for select to authenticated
  using (profile_id = app.current_profile_id());


-- ----------------------------------------------------------------------------
-- 3. The counters (§14.4-§14.6): pity, the token bars, the cycle, the Chase Shard.
-- ----------------------------------------------------------------------------
-- A pity counter counts card packs since its rarity last dropped (§14.4);
-- `pack` is '' for a counter an account keeps over all its packs or a pack's
-- key for one kept per kind of pack, which the pity math's sub-issue rules.
-- Every other counter is the account's one: a token bar's points (§14.5), the
-- cosmetic token cycle's steps paid (its position is that count modulo the
-- cycle's length) and the Chase Shard bar's cosmetic packs opened (§14.6).
create table if not exists public.economy_counters (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  counter    text not null,
  pack       text not null default '',
  value      bigint not null default 0 check (value >= 0),
  updated_at timestamptz not null default now(),
  primary key (profile_id, counter, pack),
  constraint economy_counters_counter_check check (
    counter in ('pity_epic', 'pity_legendary', 'pity_mythic', 'card_token_bar', 'cosmetic_token_bar',
                'cosmetic_token_cycle', 'chase_shard_bar')
  ),
  constraint economy_counters_pack_check check (
    counter in ('pity_epic', 'pity_legendary', 'pity_mythic') or pack = ''
  )
);

comment on table public.economy_counters is
  $$SPEC §14.4-§14.6: an account's pity counters, token bars, cosmetic token
  cycle and Chase Shard bar. No row reads as 0. Written only with an
  economy_counter_grants row, by the server.$$;

alter table public.economy_counters enable row level security;

drop trigger if exists economy_counters_set_updated_at on public.economy_counters;
create trigger economy_counters_set_updated_at
  before update on public.economy_counters
  for each row execute function app.set_updated_at();

drop policy if exists economy_counters_select_own on public.economy_counters;
create policy economy_counters_select_own on public.economy_counters for select to authenticated
  using (profile_id = app.current_profile_id());

create table if not exists public.economy_counter_grants (
  id         bigint generated always as identity primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  counter    text not null,
  pack       text not null default '',
  delta      bigint not null check (delta <> 0),
  reason     text not null,
  ref        text,
  dev        boolean not null default false,
  at         timestamptz not null default now(),
  constraint economy_counter_grants_counter_check check (
    counter in ('pity_epic', 'pity_legendary', 'pity_mythic', 'card_token_bar', 'cosmetic_token_bar',
                'cosmetic_token_cycle', 'chase_shard_bar')
  ),
  constraint economy_counter_grants_pack_check check (
    counter in ('pity_epic', 'pity_legendary', 'pity_mythic') or pack = ''
  ),
  constraint economy_counter_grants_reason_check check (
    reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
               'chase_shard', 'dev')
  ),
  constraint economy_counter_grants_dev_check check (reason <> 'dev' or dev)
);

comment on table public.economy_counter_grants is
  $$SPEC §14.11: append-only, one row per change of a counter: a pack opened
  moving pity and the bars, a bar paying and keeping its overflow, a dev
  account's clear.$$;

create index if not exists economy_counter_grants_profile_idx on public.economy_counter_grants (profile_id, id);
alter table public.economy_counter_grants enable row level security;
drop trigger if exists economy_counter_grants_deny_mutation on public.economy_counter_grants;
create trigger economy_counter_grants_deny_mutation before update or delete on public.economy_counter_grants
  for each row execute function app.deny_row_mutation('profile_id');
drop policy if exists economy_counter_grants_select_own on public.economy_counter_grants;
create policy economy_counter_grants_select_own on public.economy_counter_grants for select to authenticated
  using (profile_id = app.current_profile_id());


-- ----------------------------------------------------------------------------
-- 4. Set tracks (§14.7): XP, the level reached, the rewards paid.
-- ----------------------------------------------------------------------------
-- A track starts at its first level with no XP; `rewards_claimed` counts the
-- levels whose rewards were paid, never more than the level reached. No row
-- reads as a track at its first level with no XP and nothing paid. Whether a
-- track is unlocked is left to the sub-issue that builds the tracks (§14.7).
create table if not exists public.set_tracks (
  profile_id      uuid not null references public.profiles(id) on delete cascade,
  set_id          text not null,
  xp              bigint not null default 0 check (xp >= 0),
  -- MIRRORS crates/server/src/config.rs SET_TRACK_FIRST_LEVEL (1).
  level           bigint not null default 1 check (level >= 1),
  rewards_claimed bigint not null default 0,
  updated_at      timestamptz not null default now(),
  primary key (profile_id, set_id),
  constraint set_tracks_rewards_check check (rewards_claimed between 0 and level)
);

comment on table public.set_tracks is
  $$SPEC §14.7: an account's track of each set that ships: its XP, the level
  reached and how many levels have paid their rewards. Written only with a
  set_track_grants row, by the server.$$;

alter table public.set_tracks enable row level security;

drop trigger if exists set_tracks_set_updated_at on public.set_tracks;
create trigger set_tracks_set_updated_at
  before update on public.set_tracks
  for each row execute function app.set_updated_at();

drop policy if exists set_tracks_select_own on public.set_tracks;
create policy set_tracks_select_own on public.set_tracks for select to authenticated
  using (profile_id = app.current_profile_id());

create table if not exists public.set_track_grants (
  id         bigint generated always as identity primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  set_id     text not null,
  xp         bigint not null default 0,
  levels     bigint not null default 0,
  rewards    bigint not null default 0,
  reason     text not null,
  ref        text,
  dev        boolean not null default false,
  at         timestamptz not null default now(),
  constraint set_track_grants_change_check check (xp <> 0 or levels <> 0 or rewards <> 0),
  constraint set_track_grants_reason_check check (
    reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
               'chase_shard', 'dev')
  ),
  constraint set_track_grants_dev_check check (reason <> 'dev' or dev)
);

comment on table public.set_track_grants is
  $$SPEC §14.11: append-only, one row per change of a track: the signed XP,
  levels and rewards paid it added (a game's or a quest's XP, a level paid, a
  dev account's grant or clear).$$;

create index if not exists set_track_grants_profile_idx on public.set_track_grants (profile_id, id);
alter table public.set_track_grants enable row level security;
drop trigger if exists set_track_grants_deny_mutation on public.set_track_grants;
create trigger set_track_grants_deny_mutation before update or delete on public.set_track_grants
  for each row execute function app.deny_row_mutation('profile_id');
drop policy if exists set_track_grants_select_own on public.set_track_grants;
create policy set_track_grants_select_own on public.set_track_grants for select to authenticated
  using (profile_id = app.current_profile_id());


-- ----------------------------------------------------------------------------
-- 5. Daily quests (§14.9): the held quests and when the next is due.
-- ----------------------------------------------------------------------------
-- A quest is held once at a time: its id from the pool, its progress, and
-- when it arrived. A quest that completes or is removed leaves the table; its
-- ledger rows stay.
create table if not exists public.held_quests (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  quest_id   text not null,
  progress   bigint not null default 0 check (progress >= 0),
  arrived_at timestamptz not null,
  updated_at timestamptz not null default now(),
  primary key (profile_id, quest_id)
);

comment on table public.held_quests is
  $$SPEC §14.9: the quests an account holds, each with its progress and when it
  arrived. Written only with a quest_grants row, by the server.$$;

alter table public.held_quests enable row level security;

drop trigger if exists held_quests_set_updated_at on public.held_quests;
create trigger held_quests_set_updated_at
  before update on public.held_quests
  for each row execute function app.set_updated_at();

drop policy if exists held_quests_select_own on public.held_quests;
create policy held_quests_select_own on public.held_quests for select to authenticated
  using (profile_id = app.current_profile_id());

-- When the next quest arrives (§14.9's QUEST_INTERVAL_HOURS after the last
-- one); null, or no row, while none is due. A schedule, not a holding: it is
-- set by the server's clock and has no ledger.
create table if not exists public.quest_schedules (
  profile_id  uuid primary key references public.profiles(id) on delete cascade,
  next_due_at timestamptz,
  updated_at  timestamptz not null default now()
);

comment on table public.quest_schedules is
  $$SPEC §14.9: when an account's next quest is due; null while none is.
  Written only by the server.$$;

alter table public.quest_schedules enable row level security;

drop trigger if exists quest_schedules_set_updated_at on public.quest_schedules;
create trigger quest_schedules_set_updated_at
  before update on public.quest_schedules
  for each row execute function app.set_updated_at();

drop policy if exists quest_schedules_select_own on public.quest_schedules;
create policy quest_schedules_select_own on public.quest_schedules for select to authenticated
  using (profile_id = app.current_profile_id());

create table if not exists public.quest_grants (
  id         bigint generated always as identity primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  quest_id   text not null,
  change     text not null,
  progress   bigint not null default 0,
  reason     text not null,
  ref        text,
  dev        boolean not null default false,
  at         timestamptz not null default now(),
  constraint quest_grants_change_check check (change in ('arrived', 'progressed', 'completed', 'removed')),
  constraint quest_grants_progress_check check ((change = 'progressed') = (progress <> 0)),
  constraint quest_grants_reason_check check (
    reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
               'chase_shard', 'dev')
  ),
  constraint quest_grants_dev_check check (reason <> 'dev' or dev)
);

comment on table public.quest_grants is
  $$SPEC §14.11: append-only, one row per change of a held quest: it arrived,
  progressed by `progress`, completed, or was removed. What a completed quest
  pays is written to jlore_grants and set_track_grants.$$;

create index if not exists quest_grants_profile_idx on public.quest_grants (profile_id, id);
alter table public.quest_grants enable row level security;
drop trigger if exists quest_grants_deny_mutation on public.quest_grants;
create trigger quest_grants_deny_mutation before update or delete on public.quest_grants
  for each row execute function app.deny_row_mutation('profile_id');
drop policy if exists quest_grants_select_own on public.quest_grants;
create policy quest_grants_select_own on public.quest_grants for select to authenticated
  using (profile_id = app.current_profile_id());


-- ----------------------------------------------------------------------------
-- 6. Cosmetics (§14.2): the registry's projection and what each account owns.
-- ----------------------------------------------------------------------------
-- public.cosmetics is to the cosmetics registry (crates/server/cosmetics.json,
-- R1295000) what public.cards is to the catalog: the server's static
-- projection, written by `seed-catalog` (the second step of `release`) with
-- the catalog, so an owned cosmetic has a real foreign key. It is not the
-- client's source of cosmetics, which reads the registry's own generated copy.
create table if not exists public.cosmetics (
  id         text primary key check (id ~ '^[a-z][a-z0-9-]*$'),
  kind       text not null,
  name       text not null,
  rarity     text not null check (rarity in ('Common', 'Rare', 'Epic', 'Legendary', 'Mythic')),
  is_default boolean not null default false,
  chase      boolean not null default false,
  created_at timestamptz not null default now()
);

comment on table public.cosmetics is
  $$SPEC §14.2: the cosmetics registry (R1295000) as rows, upserted by
  seed-catalog like public.cards, for card_cosmetics, account_cosmetics and
  cosmetic_grants to reference. Every authenticated profile reads it.$$;

alter table public.cosmetics enable row level security;

drop policy if exists cosmetics_select_authenticated on public.cosmetics;
create policy cosmetics_select_authenticated on public.cosmetics for select to authenticated
  using (true);

-- A per-card cosmetic owned on one card (a foil, a finish, a border, a
-- full-art face). The starter cosmetics are owned by every account without a
-- row (§14.2).
create table if not exists public.card_cosmetics (
  profile_id  uuid not null references public.profiles(id) on delete cascade,
  card_id     text not null references public.cards(id),
  cosmetic_id text not null references public.cosmetics(id),
  primary key (profile_id, card_id, cosmetic_id)
);

comment on table public.card_cosmetics is
  $$SPEC §14.2: the per-card cosmetics an account owns, one row per card and
  cosmetic. Written only with a cosmetic_grants row, by the server.$$;

alter table public.card_cosmetics enable row level security;

drop policy if exists card_cosmetics_select_own on public.card_cosmetics;
create policy card_cosmetics_select_own on public.card_cosmetics for select to authenticated
  using (profile_id = app.current_profile_id());

-- An account cosmetic, owned for the whole account (a board, a field, a card
-- back).
create table if not exists public.account_cosmetics (
  profile_id  uuid not null references public.profiles(id) on delete cascade,
  cosmetic_id text not null references public.cosmetics(id),
  primary key (profile_id, cosmetic_id)
);

comment on table public.account_cosmetics is
  $$SPEC §14.2: the account cosmetics an account owns. Written only with a
  cosmetic_grants row, by the server.$$;

alter table public.account_cosmetics enable row level security;

drop policy if exists account_cosmetics_select_own on public.account_cosmetics;
create policy account_cosmetics_select_own on public.account_cosmetics for select to authenticated
  using (profile_id = app.current_profile_id());

create table if not exists public.cosmetic_grants (
  id          bigint generated always as identity primary key,
  profile_id  uuid not null references public.profiles(id) on delete cascade,
  card_id     text references public.cards(id),
  cosmetic_id text not null references public.cosmetics(id),
  delta       bigint not null check (delta in (-1, 1)),
  reason      text not null,
  ref         text,
  dev         boolean not null default false,
  at          timestamptz not null default now(),
  constraint cosmetic_grants_reason_check check (
    reason in ('game', 'quest', 'track', 'pack_purchase', 'pack_open', 'token_bar', 'token_redeem',
               'chase_shard', 'dev')
  ),
  constraint cosmetic_grants_dev_check check (reason <> 'dev' or dev)
);

comment on table public.cosmetic_grants is
  $$SPEC §14.11: append-only, one row per cosmetic owned (+1) or cleared (-1):
  on card_id, or on the account where card_id is null.$$;

create index if not exists cosmetic_grants_profile_idx on public.cosmetic_grants (profile_id, id);
alter table public.cosmetic_grants enable row level security;
drop trigger if exists cosmetic_grants_deny_mutation on public.cosmetic_grants;
create trigger cosmetic_grants_deny_mutation before update or delete on public.cosmetic_grants
  for each row execute function app.deny_row_mutation('profile_id');
drop policy if exists cosmetic_grants_select_own on public.cosmetic_grants;
create policy cosmetic_grants_select_own on public.cosmetic_grants for select to authenticated
  using (profile_id = app.current_profile_id());


-- ----------------------------------------------------------------------------
-- 7. The collection's ledger gains the dev marker (§14.10, §14.11).
-- ----------------------------------------------------------------------------
-- "Every dev grant and clear is written as ledger rows marked `dev`, so a dev
-- grant can always be told apart" (§14.10): a dev account's own card grants
-- and clears go to collection_grants like any other, with their reason, and
-- this marks them. Every row written before it was no dev tool's.
alter table public.collection_grants add column if not exists dev boolean not null default false;

comment on column public.collection_grants.dev is
  $$SPEC §14.10: true on a row a dev tool wrote for a dev account itself.$$;


-- ============================================================================
-- Explicit grants (default-deny first: revoke everything, then grant back
-- only what SPEC §9.1 and §14 allow), exactly as 0002 did.
-- ============================================================================
revoke all on public.wallets, public.jlore_grants, public.inventory, public.inventory_grants,
  public.economy_counters, public.economy_counter_grants, public.set_tracks, public.set_track_grants,
  public.held_quests, public.quest_schedules, public.quest_grants, public.cosmetics,
  public.card_cosmetics, public.account_cosmetics, public.cosmetic_grants
  from public, anon, authenticated;

grant select on public.wallets, public.jlore_grants, public.inventory, public.inventory_grants,
  public.economy_counters, public.economy_counter_grants, public.set_tracks, public.set_track_grants,
  public.held_quests, public.quest_schedules, public.quest_grants, public.cosmetics,
  public.card_cosmetics, public.account_cosmetics, public.cosmetic_grants
  to authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing on any of the fifteen tables.
-- authenticated: SELECT on public.cosmetics (the whole registry projection),
--                and SELECT on the fourteen others, filtered by RLS to the
--                rows where profile_id = app.current_profile_id() -- its own
--                wallet, inventory, counters, tracks, quests, cosmetics and
--                ledger rows only (§14). No INSERT, UPDATE or DELETE grant or
--                policy exists for authenticated on any of them: there is no
--                client-writable path to the economy (§9.1, §14.11).
-- service_role:  BYPASSRLS covers reads and writes to every table (the
--                project's own grants, as for every table); the server
--                (src/economy/store.rs, cli/seed_catalog.rs) is the only
--                writer, and no role edits or deletes a ledger row but its
--                account's deletion (app.deny_row_mutation('profile_id')).
-- ============================================================================
