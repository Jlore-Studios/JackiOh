-- ============================================================================
-- Migration 0027: All Random's "More cards from the newest set" (SPEC §9.5,
-- R258, R1372)
-- ============================================================================
-- A player who queues for All Random, or makes or joins an All Random room,
-- may ask that their own dealt deck lean on the newest set that ships: at
-- least half of it from that set (R1370, R1371). It is the player's intent
-- for their own seat, and the server deals the deck when the match is made,
-- so the intent has to wait where the choice waits:
--
--   * public.tickets.lean_newest         -- an All Random ticket's ask,
--                                           written at enqueue and read when
--                                           the sweeper pairs it;
--   * public.matches.room_lean_newest    -- an All Random room's host's ask,
--                                           on the `open` row beside
--                                           room_mode, room_trio and
--                                           p1_deck. The joiner's ask comes
--                                           with the join and is never
--                                           stored.
--
-- Both are `not null default false`: every older ticket and room, and every
-- one in another mode, reads as no lean, and nothing is backfilled. The set
-- itself is not stored: the server resolves "the newest set" when it deals,
-- and the match row keeps the decks it dealt, so `(seed, decks, log)` still
-- replays (R258). A rematch's ask comes with its offer, which lives in the
-- server's memory like the offer itself (R672), so nothing here holds it.
--
-- TRUST BOUNDARY: unchanged. `tickets` keeps 0022's column whitelist for
-- `authenticated` (own rows only, RLS), which grants every column a client may
-- read and no rating column (R612); a player's own lean is theirs to read, so
-- it joins the whitelist as `portrait` did. `matches` stays unreachable to
-- client roles (0004).
--
-- Apply order: ... -> 0025 -> 0026 -> 0027.
-- Safe to re-apply: add-column-if-not-exists, comment-on and grant are
-- idempotent.
-- ============================================================================

alter table public.tickets add column if not exists lean_newest boolean not null default false;

comment on column public.tickets.lean_newest is
  $$R1372: an All Random ticket's "More cards from the newest set": the deck
  dealt to this ticket's seat leans on the newest set that ships (R1370,
  R1371). False in the other two modes and on every ticket from before this
  migration.$$;

alter table public.matches add column if not exists room_lean_newest boolean not null default false;

comment on column public.matches.room_lean_newest is
  $$R1372: on an `open` row, an All Random room's host's "More cards from the
  newest set", read when the room is joined and the host's deck is dealt.
  False on every other row, and meaningless once the row is a match.$$;

grant select (lean_newest) on public.tickets to authenticated;
