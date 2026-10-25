-- ============================================================================
-- Migration 0058: per-card picks on the account, and the loadouts a match
-- freezes (SPEC §14.2, §10.10; R1834000, R1834002, R1834005; issue #1834)
-- ============================================================================
-- Serves SPEC §14.2, quoted:
--   "A player's equipped, owned cosmetics show on every board that shows their
--   seat: ... a loadout per seat, frozen with the match"
-- and R1834000: the picks a player makes for one card (a foil, a finish, a
-- border and whether its full art prints, R1655000) move from the device to
-- the account, so the server, which shows them to an opponent, holds them.
-- R1834002: when a ticket, a room or a trio freezes its deck it freezes the
-- deck's three slots with it, as R642 froze the portrait, and the match row
-- keeps both seats' resolved loadouts beside its portraits. R1834005: a
-- ladder bot keeps its wardrobe and its picks on its own row, never in the
-- ledger, so R1454's "owns no collection" holds.
--
--   1. public.card_cosmetic_picks -- one row per account and card, current
--      state with no ledger (a pick is no holding, §14.11). Each id is kept as
--      it was written, one the registry does not know among them (a newer
--      client's), so it has no foreign key to public.cosmetics or
--      public.cards: the server judges ownership where a pick is worn. RLS:
--      an account reads its own rows and writes none; every write is the
--      server's (PUT /api/card-picks, POST /api/card-picks/upload).
--   2. public.tickets.deck_cosmetics -- the Best-of-1 deck's slots, frozen
--      with it: a jsonb object of up to three ids, or NULL for none. A
--      Conquest trio's decks carry theirs inside frozen_trio, as their
--      portraits do.
--   3. public.matches.room_deck_cosmetics -- the host deck's slots while the
--      room waits for a guest, as p1_portrait holds its portrait (0019).
--   4. public.matches.loadouts -- both seats' frozen loadouts, seat order: a
--      jsonb array of two, or NULL on a match from before this file, which
--      every reader takes as all defaults. Never part of PlayerView, the
--      action log, a replay hash or a game record.
--   5. public.bots.cosmetics -- a ladder bot's wardrobe and picks, or NULL
--      before the pool draws them.
--
-- The shape checks pin a slug of at most 40 characters (0044's, the
-- PLAYER_SETTINGS_TEXT_MAX_LENGTH of R633) and the jsonb types; what each id
-- names is the server's to check, against cosmetics.json.
--
-- Apply order: ... -> 0045 (the economy) -> ... -> 0057 -> 0058 (this file).
-- This file only ADDS objects. Safe to re-apply: create-if-not-exists,
-- add-column-if-not-exists, drop-constraint-or-policy-if-exists then add.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. public.card_cosmetic_picks (R1834000).
-- ----------------------------------------------------------------------------
create table if not exists public.card_cosmetic_picks (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  card_id    text not null,
  foil       text,
  finish     text,
  border     text,
  full_art   boolean,
  updated_at timestamptz not null default now(),
  primary key (profile_id, card_id),
  constraint card_cosmetic_picks_card_check check (card_id ~ '^[a-z][a-z0-9-]{0,39}$'),
  constraint card_cosmetic_picks_foil_check check (foil is null or foil ~ '^[a-z][a-z0-9-]{0,39}$'),
  constraint card_cosmetic_picks_finish_check check (finish is null or finish ~ '^[a-z][a-z0-9-]{0,39}$'),
  constraint card_cosmetic_picks_border_check check (border is null or border ~ '^[a-z][a-z0-9-]{0,39}$'),
  constraint card_cosmetic_picks_some_check
    check (foil is not null or finish is not null or border is not null or full_art is not null)
);

comment on table public.card_cosmetic_picks is
  $$R1834000: what one card of an account wears of its own -- a foil, a finish,
  a border and whether its full art prints (R1655000) -- one row per account
  and card, at least one pick in it. Current state, no ledger. Ids are kept as
  written, unknown ones included; ownership is judged where they are worn
  (R1834002). A client reads its own rows and writes none.$$;

alter table public.card_cosmetic_picks enable row level security;

-- SPEC §9.1: a profile reads only its own rows, as player_settings (0018). No
-- insert/update/delete policy exists for any client role.
drop policy if exists card_cosmetic_picks_select_own on public.card_cosmetic_picks;
create policy card_cosmetic_picks_select_own
  on public.card_cosmetic_picks
  for select
  to authenticated
  using (profile_id = app.current_profile_id());

revoke all on public.card_cosmetic_picks from public, anon, authenticated;
grant select on public.card_cosmetic_picks to authenticated;


-- ----------------------------------------------------------------------------
-- 2-5. The frozen slots and loadouts (R1834002, R1834005).
-- ----------------------------------------------------------------------------
alter table public.tickets add column if not exists deck_cosmetics jsonb;
alter table public.tickets drop constraint if exists tickets_deck_cosmetics_check;
alter table public.tickets add constraint tickets_deck_cosmetics_check
  check (deck_cosmetics is null or jsonb_typeof(deck_cosmetics) = 'object');

comment on column public.tickets.deck_cosmetics is
  $$R1834002: the Best-of-1 deck's playmat, backdrop and card back as the
  ticket froze them ({ table?, field?, cardBack? }), NULL for none.$$;

alter table public.matches add column if not exists room_deck_cosmetics jsonb;
alter table public.matches drop constraint if exists matches_room_deck_cosmetics_check;
alter table public.matches add constraint matches_room_deck_cosmetics_check
  check (room_deck_cosmetics is null or jsonb_typeof(room_deck_cosmetics) = 'object');

comment on column public.matches.room_deck_cosmetics is
  $$R1834002: the host deck's slots while the room waits, as p1_portrait holds
  its portrait; NULL for none.$$;

alter table public.matches add column if not exists loadouts jsonb;
alter table public.matches drop constraint if exists matches_loadouts_check;
alter table public.matches add constraint matches_loadouts_check
  check (loadouts is null or (jsonb_typeof(loadouts) = 'array' and jsonb_array_length(loadouts) = 2));

comment on column public.matches.loadouts is
  $$R1834002: both seats' frozen loadouts, seat order, each { seat, cards }:
  every id one the seat's account owned as the match began, the kind's default
  otherwise. NULL on a match from before 0058, read as all defaults. Cosmetic
  only: never part of PlayerView, the log, a replay hash or a game record.$$;

alter table public.bots add column if not exists cosmetics jsonb;
alter table public.bots drop constraint if exists bots_cosmetics_check;
alter table public.bots add constraint bots_cosmetics_check
  check (cosmetics is null or jsonb_typeof(cosmetics) = 'object');

comment on column public.bots.cosmetics is
  $$R1834005: the bot's wardrobe (what it may wear, drawn from the pool seed)
  and the picks it wears, NULL until the pool draws them. Never in the ledger,
  collection_grants or GET /api/cosmetics.$$;
