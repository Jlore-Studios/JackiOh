-- ============================================================================
-- Migration 0090: Private lobbies, a friend game's pre-game stage (SPEC §9.5,
-- §13.9; R3730000, R3730001, R3730003, R3730004; issue #3730)
-- ============================================================================
-- Serves R3730000. A private lobby is where a 1v1 game or a co-op game with a
-- friend waits before it starts: its host, its one guest seat, the host's
-- settings (`{"format":"duel","mode":…}` or `{"format":"coop","difficulty":…}`),
-- each member's frozen pick (R253, R3730001) and Ready, and once the host's
-- Start has landed (R3730002) the match or series it started. `rev` counts
-- every write, so each change is a compare-and-set against the revision it
-- read (src/lobby/store.rs): a Start never seats a guest who left.
--
-- A member holds no seat of the seat ledger until Start (R1523000, R3730000),
-- and nothing here is a match: the 1v1 Start opens and claims a room under the
-- lobby's code (public.matches, 0004, 0008) and the co-op Start writes its
-- public.duos_matches row (0039) as a room's join does.
--
-- ACCESS: the server's alone (service_role, BYPASSRLS; src/lobby/store.rs):
-- RLS on, no policy, every client privilege revoked, as public.matches. A
-- lobby's picks hold whole decklists, which no client read may carry (§9.8).
-- A profile's deletion takes the lobbies it hosts or sits in (on delete
-- cascade). No data is changed or lost: this file only ADDS a table.
--
-- Apply order: ... -> 0039 (duos_rooms) -> ... -> 0089 -> 0090 (this file).
-- Safe to re-apply: create-if-not-exists throughout.
-- ============================================================================

create table if not exists public.lobbies (
  code            text primary key,
  host            uuid not null references public.profiles(id) on delete cascade,
  guest           uuid references public.profiles(id) on delete cascade,
  settings        jsonb not null,
  host_pick       jsonb,
  guest_pick      jsonb,
  host_ready      boolean not null default false,
  guest_ready     boolean not null default false,
  rev             bigint not null default 0,
  catalog_version text not null,
  created_at      timestamptz not null,
  expires_at      timestamptz not null,
  match_id        uuid,
  series_id       uuid,
  started_at      timestamptz,
  constraint lobbies_guest_not_host_check check (guest is distinct from host)
);

comment on table public.lobbies is
  'R3730000: a private lobby, a friend game''s pre-game stage (SPEC §9.5, §13.9). The server''s alone.';

create index if not exists lobbies_host_idx on public.lobbies (host);
create index if not exists lobbies_guest_idx on public.lobbies (guest);
create index if not exists lobbies_expires_at_idx on public.lobbies (expires_at);

alter table public.lobbies enable row level security;
revoke all on public.lobbies from public, anon, authenticated;
