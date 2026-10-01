-- ============================================================================
-- Migration 0017: last boards (Classic+ #29 Portal to the Past)
-- ============================================================================
-- Serves SPEC.md §8.7 row C+ #29, §9.3 and SPEC §11 R417, R565. "Your last
-- game" is your last finished game of the same kind, and Portal to the Past
-- offers the cards on the field when it ended: both sides, minus the
-- opponent's face-down cards, each as {"defId": text, "radiant": boolean}
-- (card and face, never stats; a fused card by its R179 id).
--
--   * public.last_boards -- one row per profile and kind, replaced by the
--     server in the same transaction as each finished game's result
--     (apps/server/src/api/results.ts). Only 'server' is written today;
--     'practice' is admitted so practice's on-device board could be synced
--     later without a migration.
--   * public.matches.p1_last_board / p2_last_board -- the boards a match
--     STARTED with, an input of the engine's createGame beside the decks,
--     frozen so a restarted server folds the same game (SPEC §9.3, §9.5).
--     '[]' for every older row and for an open room: the empty board such a
--     match was always played with.
--
-- Whether an entry names a card the catalog holds is the engine's question
-- when the match is created (R564), not this table's.
--
-- TRUST BOUNDARY: no client role may read or write a last board. Nothing on
-- the client needs one -- Portal's options reach their chooser through the
-- prompt in that player's own viewFor (SPEC §10.8), and viewFor never sends a
-- last board -- so last_boards is like matches (0004): RLS on, NO policy, no
-- grant to anon or authenticated. The server (service_role, BYPASSRLS) is
-- the only reader and writer.
--
-- Apply order: ... -> 0004 (`public.matches`) -> ... -> 0016 -> 0017.
-- Safe to re-apply: create-if-not-exists, add-column-if-not-exists and
-- drop-constraint-if-exists-then-add.
-- ============================================================================

-- A board's shape, as a CHECK can say it without a function (a jsonpath
-- filter is immutable): an array whose every element has a string defId and
-- a boolean radiant.
create table if not exists public.last_boards (
  profile_id uuid not null references public.profiles(id) on delete cascade,
  kind       text not null,
  board      jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (profile_id, kind),
  constraint last_boards_kind_check check (kind in ('server', 'practice')),
  constraint last_boards_board_check check (
    jsonb_typeof(board) = 'array'
    and not jsonb_path_exists(board, '$[*] ? (!(@.defId.type() == "string" && @.radiant.type() == "boolean"))')
  )
);

comment on table public.last_boards is
  $$SPEC §8.7 C+ #29 / R417, R565: each profile's last finished game's
  board, per game kind. Replaced by the server in each finished game's
  result transaction. No client access: RLS on, no policy, no grant.$$;

alter table public.last_boards enable row level security;
revoke all on public.last_boards from public, anon, authenticated;

alter table public.matches add column if not exists p1_last_board jsonb not null default '[]'::jsonb;
alter table public.matches add column if not exists p2_last_board jsonb not null default '[]'::jsonb;

alter table public.matches drop constraint if exists matches_last_boards_check;
alter table public.matches add constraint matches_last_boards_check check (
  jsonb_typeof(p1_last_board) = 'array'
  and jsonb_typeof(p2_last_board) = 'array'
  and not jsonb_path_exists(p1_last_board, '$[*] ? (!(@.defId.type() == "string" && @.radiant.type() == "boolean"))')
  and not jsonb_path_exists(p2_last_board, '$[*] ? (!(@.defId.type() == "string" && @.radiant.type() == "boolean"))')
);

comment on column public.matches.p1_last_board is
  $$R417, SPEC §9.3: p1's last board as this match started, frozen beside
  p1_deck so a rebuilt actor folds the same game. p2_last_board is p2's.$$;

-- anon, authenticated: nothing on last_boards; matches stays unreachable (0004).
-- service_role: BYPASSRLS covers both tables directly.
