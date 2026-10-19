-- ============================================================================
-- Migration 0044: the deck slots a deck carries (issue #1297, R1297000)
-- ============================================================================
-- Serves SPEC.md §9.4 D5, §10.10, §14.2 and SPEC §11 R1297000, R1295000, R641.
-- Three cosmetic picks each saved deck carries beside its hero portrait (0019):
-- a playmat ('table'), an ambient backdrop ('field') and a card back
-- ('cardBack'), each an id of `crates/server/cosmetics.json` of its kind, or
-- NULL, which every reader takes as "the account's choice, then the kind's
-- default" (R1297000). They are never part of PlayerView, the action log, a
-- replay hash or a game record.
--
--   * public.decks.table_cosmetic /
--     field_cosmetic /
--     card_back_cosmetic           -- the three picks a save carries;
--   * app.upsert_deck              -- gains p_table_cosmetic, p_field_cosmetic
--                                     and p_card_back_cosmetic as ninth to
--                                     eleventh arguments; the eight-argument
--                                     signature (0019) becomes a thin wrapper
--                                     passing three NULLs, and the seven-
--                                     argument one (0007) keeps delegating to
--                                     it, so every caller of record and every
--                                     older sql test keeps its address and its
--                                     result. A caller that saves a deck's
--                                     picks must use the eleven-argument
--                                     signature: the wrapper clears them.
--
-- The registry lives in the server (cosmetics.json, checked by
-- `cosmetics::is_id_of` before a save), exactly as the portrait roster does
-- (0019): a database copy could lag it, so what the columns and the function
-- check is shape only, a lowercase slug of at most 40 characters
-- (PLAYER_SETTINGS_TEXT_MAX_LENGTH, R633) or NULL.
--
-- TRUST BOUNDARY: unchanged. `decks` stays RLS select-own (0007) and the
-- function security-definer service_role only. A pick reveals nothing but
-- itself.
--
-- Apply order: ... -> 0019 (`app.upsert_deck`, eight arguments) -> ... ->
-- 0043 -> 0044.
-- Safe to re-apply: add-column-if-not-exists, drop-constraint-if-exists-
-- then-add, create-or-replace functions.
-- ============================================================================

alter table public.decks add column if not exists table_cosmetic text;
alter table public.decks add column if not exists field_cosmetic text;
alter table public.decks add column if not exists card_back_cosmetic text;

alter table public.decks drop constraint if exists decks_table_cosmetic_check;
alter table public.decks add constraint decks_table_cosmetic_check
  check (table_cosmetic is null or table_cosmetic ~ '^[a-z][a-z0-9-]{0,39}$');

alter table public.decks drop constraint if exists decks_field_cosmetic_check;
alter table public.decks add constraint decks_field_cosmetic_check
  check (field_cosmetic is null or field_cosmetic ~ '^[a-z][a-z0-9-]{0,39}$');

alter table public.decks drop constraint if exists decks_card_back_cosmetic_check;
alter table public.decks add constraint decks_card_back_cosmetic_check
  check (card_back_cosmetic is null or card_back_cosmetic ~ '^[a-z][a-z0-9-]{0,39}$');

comment on column public.decks.table_cosmetic is
  $$R1297000: the deck's playmat -- a 'table' id of cosmetics.json, NULL
  meaning the account's choice, then the kind's default. The registry is
  checked by the server before a save; this CHECK pins only the slug shape.$$;

comment on column public.decks.field_cosmetic is
  $$R1297000: the deck's ambient backdrop -- a 'field' id of cosmetics.json,
  NULL meaning the account's choice, then the kind's default. Shape-checked
  like table_cosmetic.$$;

comment on column public.decks.card_back_cosmetic is
  $$R1297000: the deck's card back -- a 'cardBack' id of cosmetics.json, NULL
  meaning the account's choice, then the kind's default. Shape-checked like
  table_cosmetic.$$;

-- ----------------------------------------------------------------------------
-- app.upsert_deck -- the three picks join the one write path (R250, R256,
-- R641, R1297000).
-- ----------------------------------------------------------------------------
-- The function is what migration 0019 wrote plus three writes: the picks on
-- insert and on update (created_at still kept), and a shape check each with the
-- same "deck:" prefix the other defensive checks raise.
create or replace function app.upsert_deck(
  p_profile_id          uuid,
  p_deck_id             uuid,
  p_name                text,
  p_cards               jsonb,
  p_catalog_version     text,
  p_at                  timestamptz,
  p_max_decks           int,
  p_portrait            text,
  p_table_cosmetic      text,
  p_field_cosmetic      text,
  p_card_back_cosmetic  text
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

  -- R641 D5, shape only: the roster is @jackioh/shared's, checked by
  -- @jackioh/validator before this is ever called.
  if p_portrait is not null and p_portrait !~ '^[a-z][a-z0-9-]{0,31}$' then
    raise exception 'deck: the portrait is not a portrait id';
  end if;

  -- R1297000, shape only: the ids belong to cosmetics.json and the server checked them
  -- (cosmetics::is_id_of) before this is ever called; a database copy could lag it.
  if p_table_cosmetic is not null and p_table_cosmetic !~ '^[a-z][a-z0-9-]{0,39}$' then
    raise exception 'deck: the table cosmetic is not a cosmetic id';
  end if;
  if p_field_cosmetic is not null and p_field_cosmetic !~ '^[a-z][a-z0-9-]{0,39}$' then
    raise exception 'deck: the field cosmetic is not a cosmetic id';
  end if;
  if p_card_back_cosmetic is not null and p_card_back_cosmetic !~ '^[a-z][a-z0-9-]{0,39}$' then
    raise exception 'deck: the card back cosmetic is not a cosmetic id';
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
           table_cosmetic = p_table_cosmetic,
           field_cosmetic = p_field_cosmetic,
           card_back_cosmetic = p_card_back_cosmetic,
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

  insert into public.decks (
    id, profile_id, name, cards, portrait, table_cosmetic, field_cosmetic, card_back_cosmetic,
    catalog_version, created_at, updated_at
  )
  values (
    p_deck_id, p_profile_id, p_name, p_cards, p_portrait, p_table_cosmetic, p_field_cosmetic,
    p_card_back_cosmetic, p_catalog_version, p_at, p_at
  )
  on conflict (id) do nothing
  returning id into v_written;
  if v_written is null then
    return 'not_owner';
  end if;
  return 'created';
end;
$$;

comment on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int, text, text, text, text) is
  $$R250 / R256 / R641 / R1297000: the one write path for a saved deck, with
  the deck's hero portrait (NULL = 'vanilla') and its playmat, backdrop and
  card back picks (NULL = the account's choice, then the default). Creates the
  deck or updates the profile's own (created_at kept), returning 'created',
  'updated', 'limit' or 'not_owner'. The cap is counted under the profile row
  lock. Shape checks (D1, D2, D4, the portrait's and each pick's slug, an
  active profile) raise with a "deck:" prefix; @jackioh/validator and the
  server's registry check are the authority and run first.
  service_role only.$$;

-- The eight-argument signature migration 0019 shipped stays callable and keeps
-- meaning "no deck picks": it delegates with three NULLs, so there is still one
-- body and one write path. Nothing that saves a deck's picks may call it: it
-- clears them.
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
language sql
security definer
set search_path = ''
as $$
  select app.upsert_deck(
    p_profile_id, p_deck_id, p_name, p_cards, p_catalog_version, p_at, p_max_decks, p_portrait,
    null, null, null
  );
$$;

comment on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int, text) is
  $$R250 / R256 / R641: the pre-cosmetics signature of the one write path for
  a saved deck, kept for callers and tests written before migration 0044.
  Delegates to the eleven-argument function with NULL picks, so a save through
  it clears a deck's playmat, backdrop and card back. service_role only.$$;

-- ----------------------------------------------------------------------------
-- Explicit grants, exactly as 0019 made them for the eight-argument function.
-- ----------------------------------------------------------------------------
revoke all on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int, text, text, text, text) from public;

grant execute on function app.upsert_deck(uuid, uuid, text, jsonb, text, timestamptz, int, text, text, text, text) to service_role;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do (unchanged from
-- 0007/0019).
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: SELECT on decks (own rows, RLS), which now includes the three
--                picks each carries. No write of any kind; a save still goes
--                through the server.
-- service_role:  BYPASSRLS covers every table directly; the new EXECUTE on the
--                eleven-argument app.upsert_deck is the one write path.
-- ============================================================================
