-- ============================================================================
-- Migration 0036: "More cards from" picks a set (SPEC §9.5, §9.9, R1372,
-- R924000)
-- ============================================================================
-- 0027 stored All Random's "More cards from the newest set" as a boolean:
-- whether the deck dealt to a seat leans on the newest set that ships. Since
-- R924000 the player picks which set to lean on: "newest" (the newest set
-- that ships when the deck is dealt, as before) or one shipped set by name,
-- or none. So each boolean becomes the pick, as text:
--
--   * public.tickets.lean_newest      -> public.tickets.lean_set
--   * public.matches.room_lean_newest -> public.matches.room_lean_set
--
-- `null` is no lean. A row stored with `true` meant "newest", and becomes
-- 'newest'; a row stored with `false` meant none, and becomes null.
--
-- The old columns stay, unread: Render starts the new server, which migrates
-- at boot, while the old one still serves, and every ticket and room query
-- the old one makes names them. No migration here has dropped a column a
-- running server reads, and this one does not either. Nothing after this
-- migration reads or writes them.
--
-- The server is the only writer (no client role may insert a ticket or a
-- match row, 0004), and it writes only "newest" or the name of a set that
-- ships (`LeanPick`, R1420). The check below holds the column to that shape
-- without naming the sets, so a set that ships later needs no migration of
-- its own: the server refuses a name it does not know, and reading a stored
-- one it does not know is an error, never another lean.
--
-- TRUST BOUNDARY: unchanged. `tickets` keeps 0022's column whitelist for
-- `authenticated` (own rows only, RLS); the player's own pick is theirs to
-- read, so `lean_set` joins it beside `lean_newest`. `matches` stays
-- unreachable to client roles (0004).
--
-- Apply order: ... -> 0034 -> 0035 -> 0036.
-- Safe to re-apply: the columns are added if missing, the copy fills only an
-- empty pick, and the constraints, comments and grant are idempotent.
-- ============================================================================

alter table public.tickets add column if not exists lean_set text;

alter table public.tickets drop constraint if exists tickets_lean_set_check;
alter table public.tickets add constraint tickets_lean_set_check
  check (lean_set is null or lean_set ~ '^[A-Za-z][A-Za-z+-]{0,31}$');

alter table public.matches add column if not exists room_lean_set text;

alter table public.matches drop constraint if exists matches_room_lean_set_check;
alter table public.matches add constraint matches_room_lean_set_check
  check (room_lean_set is null or room_lean_set ~ '^[A-Za-z][A-Za-z+-]{0,31}$');

update public.tickets set lean_set = 'newest' where lean_newest and lean_set is null;
update public.matches set room_lean_set = 'newest' where room_lean_newest and room_lean_set is null;

comment on column public.tickets.lean_newest is
  $$R1372 until 0036: superseded by lean_set, which holds the same pick
  ('newest' for true). Kept unread, so a server from before 0036 still runs
  while a deploy replaces it.$$;

comment on column public.matches.room_lean_newest is
  $$R1372 until 0036: superseded by room_lean_set, which holds the same pick
  ('newest' for true). Kept unread, so a server from before 0036 still runs
  while a deploy replaces it.$$;

comment on column public.tickets.lean_set is
  $$R1372, R924000: an All Random ticket's "More cards from" pick for the deck
  dealt to its seat when it is paired: 'newest' (the newest set that ships
  then, R1371) or a shipped set's name. Null is no lean, as in the other two
  modes. 0027's `lean_newest = true` became 'newest'.$$;

comment on column public.matches.room_lean_set is
  $$R1372, R924000: on an `open` row, an All Random room's host's "More cards
  from" pick, read when the room is joined and the host's deck is dealt. Null
  is no lean; meaningless once the row is a match. 0027's
  `room_lean_newest = true` became 'newest'.$$;

grant select (lean_set) on public.tickets to authenticated;
