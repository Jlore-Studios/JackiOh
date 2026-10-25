-- ============================================================================
-- Migration 0056: Every foreign key a delete or a lookup walks has an index
-- (issue #1800)
-- ============================================================================
-- A delete of a referenced row looks up the rows that point at it by the key's
-- columns, to cascade, set null or refuse, and so does a read by such a link;
-- with no index leading with those columns, each such lookup reads the whole
-- table. Sixteen keys outside the catalog had none that serves them:
--
--   * into public.matches, walked by every delete of a match:
--     tickets.match_id, which matches_mode_of (src/db/pg.rs) reads at every
--     match start and rebuild, and profiles.current_match_id;
--   * into public.profiles, walked by every account deletion (0012):
--     match_actions.player_id, results.winner_profile_id,
--     invite_codes.created_by, duos_rooms.host and guest, replay_shares.sharer,
--     and five whose only leading index is partial on another predicate, so it
--     cannot serve the delete: tickets.profile_id (tickets_profile_queued_key),
--     challenges.from_profile (challenges_open_pair_key),
--     chat_messages.to_profile (chat_messages_unread_idx) and duos_matches.a1
--     and a2 (duos_matches_live_a1_idx, duos_matches_live_a2_idx);
--   * into public.decks, walked by every deck delete: trios' three slot keys,
--     (profile_id, deckN_id). 0007 left them to trios_profile_created_idx,
--     which a profile's few trios keep cheap; they get their own so that
--     tests/sql/01_schema_invariants.sql CHECK 19 holds every key alike.
--
-- An index on a nullable key is partial on `is not null`: a lookup by the key
-- never matches a null, so the rows it leaves out are never looked for. The
-- keys into the catalog (public.cards, public.cosmetics) stay as they are: the
-- server never deletes a catalog row.
--
-- Only indexes, no data. Each is built in this file's transaction, so without
-- `concurrently`, holding writes to its table while it builds.
--
-- Apply order: 0001 (profiles, invite_codes) -> 0004 (matches, tickets,
-- match_actions, results) -> 0007 (trios) -> ... -> 0038 (challenges,
-- chat_messages, replay_shares) -> 0039 (duos_rooms, duos_matches) -> ... ->
-- 0055 -> 0056 (this file).
-- Safe to re-apply: every index is `create index if not exists`.
-- ============================================================================

-- Into public.matches.
create index if not exists tickets_match_id_idx
  on public.tickets (match_id) where match_id is not null;
create index if not exists profiles_current_match_id_idx
  on public.profiles (current_match_id) where current_match_id is not null;

-- Into public.profiles.
create index if not exists match_actions_player_id_idx
  on public.match_actions (player_id) where player_id is not null;
create index if not exists results_winner_profile_id_idx
  on public.results (winner_profile_id) where winner_profile_id is not null;
create index if not exists invite_codes_created_by_idx
  on public.invite_codes (created_by) where created_by is not null;
create index if not exists duos_rooms_host_idx on public.duos_rooms (host);
create index if not exists duos_rooms_guest_idx
  on public.duos_rooms (guest) where guest is not null;
create index if not exists replay_shares_sharer_idx on public.replay_shares (sharer);
create index if not exists tickets_profile_id_idx on public.tickets (profile_id);
create index if not exists challenges_from_idx on public.challenges (from_profile);
create index if not exists chat_messages_to_idx on public.chat_messages (to_profile);
create index if not exists duos_matches_a1_idx
  on public.duos_matches (a1) where a1 is not null;
create index if not exists duos_matches_a2_idx
  on public.duos_matches (a2) where a2 is not null;

-- Into public.decks.
create index if not exists trios_profile_deck1_idx
  on public.trios (profile_id, deck1_id) where deck1_id is not null;
create index if not exists trios_profile_deck2_idx
  on public.trios (profile_id, deck2_id) where deck2_id is not null;
create index if not exists trios_profile_deck3_idx
  on public.trios (profile_id, deck3_id) where deck3_id is not null;
