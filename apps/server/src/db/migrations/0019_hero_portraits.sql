-- ============================================================================
-- Migration 0019: hero portraits (patch v0.2.X, R635–R637)
-- ============================================================================
-- Serves SPEC.md §9.4 D5, §9.5, §10.10 and SPEC §11 R635, R636. A hero
-- portrait is a cosmetic pick each saved deck carries: one of the six ids in
-- @jackioh/shared's PORTRAIT_IDS ('vanilla', 'gary', 'timmy', 'dfender',
-- 'felinors', 'shredder'), or NULL, which every reader takes as 'vanilla'.
-- Portraits are never part of PlayerView, the action log, a replay hash or a
-- game record (R637).
--
--   * public.decks.portrait          -- the pick a save carries (D5);
--   * public.tickets.portrait        -- a Best-of-1 ticket's frozen pick
--                                       (R636: "frozen into the ticket");
--   * public.matches.p1_portrait /
--     p2_portrait                    -- the pair dealt to a match's seats at
--                                       creation (R636). On an `open` row --
--                                       a waiting room -- p1_portrait holds
--                                       the host's pick beside p1_deck, the
--                                       same trick room_trio plays; a queue
--                                       skeleton leaves both NULL until
--                                       matches.create completes the row.
--   * app.upsert_deck                -- gains p_portrait as an eighth
--                                       argument; the seven-argument
--                                       signature stays as a thin wrapper
--                                       passing NULL, so every caller of
--                                       record (and every older sql test)
--                                       keeps its address and its result.
--
-- A portrait's roster lives in the server (PORTRAIT_IDS, checked by
-- @jackioh/validator's D5 before a save), exactly as a card's membership of
-- the catalog lives there and is deliberately NOT re-checked inside
-- app.upsert_deck (migration 0007, "D3 ... is deliberately NOT re-checked
-- here"). What the columns check is shape: a lowercase slug or NULL.
--
-- TRUST BOUNDARY: unchanged. `decks` stays RLS select-own (0007) and the
-- function security-definer service_role only; matches and tickets stay
-- unreachable to client roles. A portrait reveals nothing but itself.
--
-- Apply order: ... -> 0007 (`public.decks`, `app.upsert_deck`) -> 0008
-- -> 0009 -> ... -> 0018 -> 0019.
-- Safe to re-apply: add-column-if-not-exists, drop-constraint-if-exists-
-- then-add, create-or-replace functions.
-- ============================================================================

alter table public.decks add column if not exists portrait text;

alter table public.decks drop constraint if exists decks_portrait_check;
alter table public.decks add constraint decks_portrait_check
  check (portrait is null or portrait ~ '^[a-z][a-z0-9-]{0,31}$');

comment on column public.decks.portrait is
  $$R635: the deck's hero portrait -- a PORTRAIT_IDS id (@jackioh/shared),
  NULL meaning 'vanilla'. D5 checks the roster in @jackioh/validator before a
  save; this CHECK pins only the slug shape, like 0007 leaves D3 to the
  server.$$;

alter table public.tickets add column if not exists portrait text;

alter table public.tickets drop constraint if exists tickets_portrait_check;
alter table public.tickets add constraint tickets_portrait_check
  check (portrait is null or portrait ~ '^[a-z][a-z0-9-]{0,31}$');

comment on column public.tickets.portrait is
  $$R636: a Best-of-1 ticket's frozen hero portrait, written beside
  frozen_deck at enqueue; NULL in the other two modes and on tickets from
  before this migration (reads as 'vanilla').$$;

alter table public.matches add column if not exists p1_portrait text;
alter table public.matches add column if not exists p2_portrait text;

alter table public.matches drop constraint if exists matches_portraits_check;
alter table public.matches add constraint matches_portraits_check
  check (
    (p1_portrait is null or p1_portrait ~ '^[a-z][a-z0-9-]{0,31}$')
    and (p2_portrait is null or p2_portrait ~ '^[a-z][a-z0-9-]{0,31}$')
  );

comment on column public.matches.p1_portrait is
  $$R636: p1's hero portrait, dealt at match creation; on an `open` row the
  host's, waiting beside p1_deck (rooms.ts reads it as Room.hostPortrait).
  NULL on a match from before this migration, read as 'vanilla'. p2_portrait
  is p2's.$$;

-- ----------------------------------------------------------------------------
-- app.upsert_deck -- p_portrait joins the one write path (R250, R256, R635).
-- ----------------------------------------------------------------------------
-- The function is what migration 0007 wrote plus one write: `portrait` on
-- insert and on update (created_at still kept), and a shape check with the
-- same "deck:" prefix the other defensive checks raise. The roster itself is
-- not re-checked here for the reason D3 is not: the ids live in the server's
-- package and a database copy could lag it.
create or replace function app.upsert_deck(
  p_profile_id      uuid,
  p_deck_id         uuid,
  p_name            text,
  p_cards           jsonb,
  p_catalog_version text,
  p_at              timestamptz,
  p_max_decks       int,
  p_portrait        text
) returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_status     text;
  v_owner      uuid;
  v_name_max   int;
  v_deck_size  int;
  v_max_copies int;
  v_cap        int;
  v_count      int;
  v_written    uuid;
begin
  select p.status into v_status
    from public.profiles p
   where p.id = p_profile_id
     for update;
  if not found then
    raise exception 'deck: profile % not found', p_profile_id;
  elsif v_status <> 'active' then
    raise exception 'deck: profile % is not active (status=%)', p_profile_id, v_status;
  end if;

  -- R250 D1, defensively: a name, not too long, no control characters.
  select (app.setting('deck_name_max_length'))::text::int into v_name_max;
  if p_name is null or btrim(p_name) = '' then
    raise exception 'deck: a deck needs a name';
  end if;
  if char_length(p_name) > v_name_max then
    raise exception 'deck: the name is % characters, at most % allowed',
      char_length(p_name), v_name_max;
  end if;
  if p_name ~ '[[:cntrl:]\u00AD\u061C\u180E\u200B\u200E\u200F\u202A-\u202E\u2060-\u2064\u2066-\u206F\uFEFF]' then
    raise exception 'deck: the name contains a control character';
  end if;

  -- R250 D2 and D4, defensively, plus the shape the Store port reads back: a
  -- jsonb array of card-id strings.
  if p_cards is null or jsonb_typeof(p_cards) <> 'array' then
    raise exception 'deck: p_cards must be a JSON array of card ids';
  end if;
  if exists (select 1 from jsonb_array_elements(p_cards) as e(card) where jsonb_typeof(e.card) <> 'string') then
    raise exception 'deck: every entry of p_cards must be a card id string';
  end if;
  select (app.setting('deck_size'))::text::int  into v_deck_size;
  select (app.setting('max_copies'))::text::int into v_max_copies;
  if jsonb_array_length(p_cards) > v_deck_size then
    raise exception 'deck: D2 % cards, at most % allowed', jsonb_array_length(p_cards), v_deck_size;
  end if;
  if exists (
    select 1 from jsonb_array_elements_text(p_cards) as c(card_id)
     group by c.card_id having count(*) > v_max_copies
  ) then
    raise exception 'deck: D4 a card appears more than % time(s)', v_max_copies;
  end if;

  -- R635 D5, shape only: the roster is @jackioh/shared's, checked by
  -- @jackioh/validator before this is ever called.
  if p_portrait is not null and p_portrait !~ '^[a-z][a-z0-9-]{0,31}$' then
    raise exception 'deck: the portrait is not a portrait id';
  end if;

  select d.profile_id into v_owner from public.decks d where d.id = p_deck_id for update;
  if found then
    if v_owner <> p_profile_id then
      return 'not_owner';
    end if;
    update public.decks
       set name = p_name,
           cards = p_cards,
           portrait = p_portrait,
           catalog_version = p_catalog_version,
           updated_at = p_at
     where id = p_deck_id;
    return 'updated';
  end if;

  v_cap := least(p_max_decks, (app.setting('max_saved_decks'))::text::int);
  select count(*) into v_count from public.decks d where d.profile_id = p_profile_id;
  if v_count >= v_cap then
    return 'limit';
  end if;

  insert into public.decks (id, profile_id, name, cards, portrait, catalog_version, created_at, updated_at)
  values (p_deck_id, p_profile_id, p_name, p_cards, p_portrait, p_catalog_version, p_at, p_at)
  on conflict (id) do nothing
  returning id into v_written;
  if v_written is null then
    return 'not_owner';
  end if;
  return 'created';
end;
$$;

comment on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int, text) is
  $$R250 / R256 / R635: the one write path for a saved deck, with the deck's
  hero portrait (NULL = 'vanilla'). Creates the deck or updates the profile's
  own (created_at kept), returning 'created', 'updated', 'limit' or
  'not_owner'. The cap is counted under the profile row lock. Shape checks
  (D1, D2, D4, the portrait's slug, an active profile) raise with a "deck:"
  prefix; @jackioh/validator is the authority and runs first.
  service_role only.$$;

-- The seven-argument signature migration 0007 shipped stays callable and
-- keeps meaning "no portrait": it delegates, so there is still one body and
-- one write path. (A default on the new parameter would not have added an
-- overload at all -- this keeps the address the sql tests pin.)
create or replace function app.upsert_deck(
  p_profile_id      uuid,
  p_deck_id         uuid,
  p_name            text,
  p_cards           jsonb,
  p_catalog_version text,
  p_at              timestamptz,
  p_max_decks       int
) returns text
language sql
security definer
set search_path = ''
as $$
  select app.upsert_deck(
    p_profile_id, p_deck_id, p_name, p_cards, p_catalog_version, p_at, p_max_decks, null
  );
$$;

comment on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int) is
  $$R250 / R256: the pre-portrait signature of the one write path for a saved
  deck, kept for callers and tests written before migration 0019. Delegates
  to the eight-argument function with a NULL portrait ('vanilla').
  service_role only.$$;

-- ----------------------------------------------------------------------------
-- Explicit grants, exactly as 0007 made them for the seven-argument function.
-- ----------------------------------------------------------------------------
revoke all on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int)       from public;
revoke all on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int, text) from public;

grant execute on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int)       to service_role;
grant execute on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int, text) to service_role;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do (unchanged from
-- 0007/0008/0017).
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: SELECT on decks (own rows, RLS), which now includes the
--                portrait each carries. No write of any kind; a save still
--                goes through the server.
-- service_role:  BYPASSRLS covers every table directly; the new EXECUTE on
--                the eight-argument app.upsert_deck is the one write path.
-- ============================================================================
