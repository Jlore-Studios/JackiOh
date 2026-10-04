-- ============================================================================
-- Migration 0019: Player statistics on the account
-- ============================================================================
-- Serves SPEC.md §9.11 and SPEC §11 R641: each player's statistics tracked by
-- the client (R639, #125), synced to the account so players can view their own
-- stats across devices and participate in the public players tab.
--
-- TRUST BOUNDARY: no client role reads or writes this table directly. The server
-- (service_role, BYPASSRLS) reads and writes it through the Store port.
-- RLS default-deny with no policies ensures privacy.
-- ============================================================================

create table if not exists public.player_stats (
  profile_id uuid primary key references public.profiles(id) on delete cascade,
  stats      jsonb not null default '{}'::jsonb,
  is_private boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint player_stats_stats_object_check check (jsonb_typeof(stats) = 'object')
);

comment on table public.player_stats is
  $$SPEC §9.11 / R641: each profile's player statistics and privacy setting.
  Read and written solely by the server (service_role); no client access.$$;

comment on column public.player_stats.stats is
  $$The player's tally from R639: games, wins, losses, draws, cards.$$;

comment on column public.player_stats.is_private is
  $$Whether the player opted out of public player stats (false by default).$$;

alter table public.player_stats enable row level security;
revoke all on public.player_stats from public, anon, authenticated;
