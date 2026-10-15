-- ============================================================================
-- Migration 0037: Custom cards on the account (SPEC §12.9, R915008; issue #918)
-- ============================================================================
-- Serves SPEC.md §12.9 and SPEC §11 R915008, quoted:
--   "An active account's custom cards are stored by the server in a table only
--   their owner can read or write: at most `CUSTOM_CARDS_PER_ACCOUNT` (50), each
--   saved whole by the routes under `/api/custom-cards` (list, get, create,
--   update, delete). The server checks the frame with the engine's validator
--   and the source's size, and never compiles or runs a script."
--
-- One row per custom card (§12.1, R915000): the id the server minted at the
-- first save (`custom-` and 32 lowercase hex digits), its owner, its version
-- (the count of its saves, from 1), its frame, its card-kit source and the
-- builder's model of it -- the engine's `CustomCard`
-- (crates/engine/src/wire/custom.rs), with when it was made and last saved.
--
-- TRUST BOUNDARY, exactly as saved decks (0007) and settings (0018): a profile
-- reads its own rows through the Data API and writes none. SPEC §9.1's
-- "Propose": the browser proposes a card to the server API, which checks it
-- with the engine's validator (R915001) and writes it as service_role through
-- the two functions below. A client write path would skip that check, so there
-- is none: RLS default deny is the enforcement, and no client role holds an
-- insert, update or delete grant or policy. Nothing here reads a script: the
-- source is text the database stores, and no role runs it.
--
-- The checks on the table restate the engine's limits (crates/engine/src/
-- config.rs) as a backstop for a writer that skipped the server, which applies
-- them first and exactly: the id's form (CUSTOM_ID_PREFIX, CUSTOM_ID_MAX_LENGTH
-- 64), a version from 1, the source's CUSTOM_SOURCE_MAX_BYTES (8192 bytes of
-- UTF-8, which octet_length counts as Rust's `str::len` does) and the builder's
-- CUSTOM_BUILDER_MAX_BYTES (16384), which the engine counts as compact JSON;
-- jsonb prints a space after every `:` and `,`, so its text is held to twice
-- that. The cap is app.settings' `max_custom_cards`, mirroring the server's
-- CUSTOM_CARDS_PER_ACCOUNT, counted under the profile row lock.
--
-- Apply order: ... -> 0035 -> 0036 -> 0037 (this file).
-- This file only ADDS objects. Safe to re-apply: create-if-not-exists,
-- drop-and-add constraints, create-or-replace, drop-policy-if-exists, and a
-- setting inserted with ON CONFLICT DO NOTHING.
-- ============================================================================

create table if not exists public.custom_cards (
  id         text primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  version    int not null default 1,
  frame      jsonb not null,
  source     text not null default '',
  builder    jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

alter table public.custom_cards drop constraint if exists custom_cards_id_check;
alter table public.custom_cards
  add constraint custom_cards_id_check check (id ~ '^custom-[a-z0-9-]{1,57}$');

alter table public.custom_cards drop constraint if exists custom_cards_version_check;
alter table public.custom_cards
  add constraint custom_cards_version_check check (version >= 1);

alter table public.custom_cards drop constraint if exists custom_cards_frame_check;
alter table public.custom_cards
  add constraint custom_cards_frame_check check (jsonb_typeof(frame) = 'object');

alter table public.custom_cards drop constraint if exists custom_cards_source_check;
alter table public.custom_cards
  add constraint custom_cards_source_check check (octet_length(source) <= 8192);

alter table public.custom_cards drop constraint if exists custom_cards_builder_check;
alter table public.custom_cards
  add constraint custom_cards_builder_check
  check (builder is null or octet_length(builder::text) <= 2 * 16384);

comment on table public.custom_cards is
  $$SPEC §12.9 / R915008: an active account's custom cards, private to it.
  Written only by app.create_custom_card and app.update_custom_card (SECURITY
  DEFINER, service_role only) after the server's validator (R915001), and
  deleted only by the server; a client reads its own rows and writes none
  (SPEC §9.1: "Propose"). No role runs a source stored here.$$;

comment on column public.custom_cards.id is
  $$R915000: minted by the server at the first save: `custom-` and 32 lowercase
  hex digits. No catalog id begins with `custom-`.$$;

comment on column public.custom_cards.version is
  $$§12.1: the count of the card's saves, from 1. An update names the version
  it replaces and is refused when another save came first (R915008).$$;

comment on column public.custom_cards.frame is
  $$R915001: the engine's CustomFrame (name, type, cost, rarity, stats,
  keywords, text, numbers), within §12.2's limits.$$;

comment on column public.custom_cards.source is
  $$§12.3: the card-kit Luau, at most CUSTOM_SOURCE_MAX_BYTES bytes; empty is a
  vanilla card. Stored, never run.$$;

comment on column public.custom_cards.builder is
  $$§12.2: the no-code builder's model of the card, which only its creator
  reads; null for a card edited in code beyond what the builder shows.$$;

comment on column public.custom_cards.created_at is
  $$The list order: a profile's cards are read ordered by (created_at, id).$$;

create index if not exists custom_cards_profile_created_idx
  on public.custom_cards (profile_id, created_at, id);

alter table public.custom_cards enable row level security;

-- SPEC §12.9: a profile reads only its own cards, exactly as its decks (0007).
-- No insert/update/delete policy exists for any client role.
drop policy if exists custom_cards_select_own on public.custom_cards;
create policy custom_cards_select_own
  on public.custom_cards
  for select
  to authenticated
  using (profile_id = app.current_profile_id());

-- ============================================================================
-- Server config the create reads.
-- ============================================================================
-- MIRRORS crates/server/src/config.rs CUSTOM_CARDS_PER_ACCOUNT (50, R915008),
-- as 0007 mirrors the deck caps: the server passes its own cap to each create
-- and the function applies the smaller of the two, so the database refuses an
-- overfull account even from a caller that skipped the server. Changing the
-- number for a deployment means config.rs AND a new migration updating this row.
insert into app.settings (key, value) values ('max_custom_cards', to_jsonb(50))
on conflict (key) do nothing;

-- ----------------------------------------------------------------------------
-- app.create_custom_card -- the one write path for a new card (R915008).
-- ----------------------------------------------------------------------------
-- Returns 'created' (written at version 1) or 'limit' (the profile already
-- holds least(p_max_cards, max_custom_cards) cards; nothing written). An id
-- already held raises custom_cards_pkey's unique violation: the server mints
-- 128 random bits, so it is a fault, never a player's answer.
create or replace function app.create_custom_card(
  p_profile_id uuid,
  p_card_id    text,
  p_frame      jsonb,
  p_source     text,
  p_builder    jsonb,
  p_at         timestamptz,
  p_max_cards  int
) returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_status text;
  v_cap    int;
  v_count  int;
begin
  -- SPEC §12.9: an ACTIVE account's cards. Checked against p_profile_id itself,
  -- as app.upsert_deck does, because the only caller is service_role, for whom
  -- auth.uid() means nothing. `for update` is the lock the cap is counted under,
  -- so two creates at once cannot both pass it.
  select p.status into v_status
    from public.profiles p
   where p.id = p_profile_id
     for update;
  if not found then
    raise exception 'custom card: profile % not found', p_profile_id;
  elsif v_status <> 'active' then
    raise exception 'custom card: profile % is not active (status=%)', p_profile_id, v_status;
  end if;

  v_cap := least(p_max_cards, (app.setting('max_custom_cards'))::text::int);
  select count(*) into v_count from public.custom_cards c where c.profile_id = p_profile_id;
  if v_count >= v_cap then
    return 'limit';
  end if;

  insert into public.custom_cards (id, profile_id, version, frame, source, builder, created_at, updated_at)
  values (p_card_id, p_profile_id, 1, p_frame, p_source, p_builder, p_at, p_at);
  return 'created';
end;
$$;

comment on function app.create_custom_card(uuid, text, jsonb, text, jsonb, timestamptz, int) is
  $$R915008: the one write path for a new custom card. Writes it at version 1
  and returns 'created', or returns 'limit' (past least(p_max_cards,
  app.settings max_custom_cards); nothing written). The cap is counted under the
  profile row lock. An inactive or missing profile raises with a "custom card:"
  prefix, an id already held raises custom_cards_pkey; the engine's validator
  (R915001) is the authority and runs first. service_role only.$$;

-- ----------------------------------------------------------------------------
-- app.update_custom_card -- the one write path for a save of a card (R915008).
-- ----------------------------------------------------------------------------
-- p_version is the version the save replaces. Returns:
--   'updated' -- the frame, source and builder replaced whole, the version
--                counted (p_version + 1), updated_at set, created_at kept;
--   'stale'   -- the card's version is no longer p_version: another save came
--                first; nothing written;
--   'missing' -- the profile has no card with this id (another profile's card
--                answers the same); nothing written.
-- Under the card's row lock, so two saves of one version cannot both land.
create or replace function app.update_custom_card(
  p_profile_id uuid,
  p_card_id    text,
  p_version    int,
  p_frame      jsonb,
  p_source     text,
  p_builder    jsonb,
  p_at         timestamptz
) returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_status  text;
  v_version int;
begin
  select p.status into v_status
    from public.profiles p
   where p.id = p_profile_id;
  if not found then
    raise exception 'custom card: profile % not found', p_profile_id;
  elsif v_status <> 'active' then
    raise exception 'custom card: profile % is not active (status=%)', p_profile_id, v_status;
  end if;

  select c.version into v_version
    from public.custom_cards c
   where c.id = p_card_id
     and c.profile_id = p_profile_id
     for update;
  if not found then
    return 'missing';
  end if;
  if v_version <> p_version then
    return 'stale';
  end if;

  update public.custom_cards
     set version = v_version + 1,
         frame = p_frame,
         source = p_source,
         builder = p_builder,
         updated_at = p_at
   where id = p_card_id;
  return 'updated';
end;
$$;

comment on function app.update_custom_card(uuid, text, int, jsonb, text, jsonb, timestamptz) is
  $$R915008: the one write path for a save of an existing custom card. Replaces
  the profile's own card whole when its version is still p_version, counting
  the save, and returns 'updated'; 'stale' when another save came first,
  'missing' when the profile has no card with this id (nothing written for
  either). An inactive or missing profile raises with a "custom card:" prefix.
  service_role only.$$;

-- ============================================================================
-- Explicit grants (default-deny first: revoke everything, then grant back
-- only what SPEC §9.1 allows), exactly as 0007 and 0018 did.
-- ============================================================================
revoke all on public.custom_cards from public, anon, authenticated;
grant select on public.custom_cards to authenticated;

revoke all on function app.create_custom_card(uuid, text, jsonb, text, jsonb, timestamptz, int) from public;
revoke all on function app.update_custom_card(uuid, text, int, jsonb, text, jsonb, timestamptz) from public;
grant execute on function app.create_custom_card(uuid, text, jsonb, text, jsonb, timestamptz, int) to service_role;
grant execute on function app.update_custom_card(uuid, text, int, jsonb, text, jsonb, timestamptz) to service_role;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing on custom_cards, and no EXECUTE on either function.
-- authenticated: SELECT on custom_cards, filtered by RLS to the rows where
--                profile_id = app.current_profile_id() -- its own cards only.
--                No INSERT, UPDATE or DELETE grant or policy and no EXECUTE on
--                either function: a client proposes a card to the server API,
--                which validates it and calls them.
-- service_role:  BYPASSRLS covers reads and deletes of the table directly (GET
--                and DELETE /api/custom-cards); the two functions are granted
--                EXECUTE explicitly, and every create and save goes through
--                them.
-- ============================================================================
