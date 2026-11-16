-- ============================================================================
-- Migration 0093: the in-game notification feed, kept until read (SPEC §9.4,
-- §9.13; R4039000; issue #4039)
-- ============================================================================
-- Serves R4039000. JackiOh tells a player what happened, and keeps telling
-- them until they have seen it, without email: one row per event for one
-- account, written by the server where it already handles the event (a daily
-- game waiting on you, ending, a daily challenge and its answer, a friend
-- request and its accept, a lobby invite, a shared replay), read in the
-- inbox's Recent section and pushed over /ws/social as it is written.
--
--   * public.notifications -- `profile_id` is whom it tells, `kind` what
--     happened, `actor_id` who did it (null when nobody did), `subject` what
--     it is about (a daily match id, a challenge or friend request id, a lobby
--     code, a replay's match id), `payload` the few words it needs (a result, a
--     format), `created_at` and `read_at` epoch ms, as 0089's times are. A new
--     row replaces its account's unread row of the same kind, actor and
--     subject, so a daily game's newer move never stacks (src/notifications/).
--
-- KEPT SMALL: the server keeps each account's newest NOTIFICATIONS_KEEP rows
-- as it writes one, and the daily sweep deletes rows older than
-- NOTIFICATIONS_KEEP_DAYS (crates/server/src/config.rs).
--
-- TRUST BOUNDARY: one account's history of who did what to it. Like
-- public.practice_claim_counts (0078) and public.daily_email_days (0092) it
-- has RLS on, no policy and no grant: only service_role (BYPASSRLS) reads or
-- writes it, and a client reads its own rows through GET /api/notifications.
-- A profile's deletion takes the rows naming it either way (on delete cascade).
-- No data is changed or lost: this file only ADDS a table.
--
-- Apply order: ... -> 0089 (daily games) -> 0090 (lobbies) -> 0091 (2v2) ->
-- 0092 (daily email days) -> 0093 (this file).
-- Safe to re-apply: create-if-not-exists, comment-on, revoke.
-- ============================================================================

create table if not exists public.notifications (
  id         bigint generated always as identity primary key,
  profile_id uuid not null references public.profiles(id) on delete cascade,
  kind       text not null constraint notifications_kind_check check (kind in (
               'dailyMove', 'dailyEnded', 'dailyChallenge', 'dailyAnswered',
               'friendRequest', 'friendAccepted', 'lobbyInvite', 'replayShared')),
  actor_id   uuid references public.profiles(id) on delete cascade,
  subject    text,
  payload    jsonb not null default '{}'::jsonb,
  created_at bigint not null,
  read_at    bigint
);

comment on table public.notifications is
  $$R4039000 (SPEC §9.4, §9.13, #4039): the in-game notification feed, one row per
  event for one account, kept until read, at most NOTIFICATIONS_KEEP an account and
  NOTIFICATIONS_KEEP_DAYS old. The server's alone: RLS on, no policy, no grant.$$;

-- The page (newest first, by id), the replace and the cap read an account's rows by id.
create index if not exists notifications_profile_idx on public.notifications (profile_id, id desc);
-- CHECK 19: every foreign key has an index that leads with it (a deleted actor's cascade).
create index if not exists notifications_actor_idx on public.notifications (actor_id);
-- The sweep's purge.
create index if not exists notifications_created_at_idx on public.notifications (created_at);

alter table public.notifications enable row level security;

revoke all on public.notifications from public, anon, authenticated;

-- ----------------------------------------------------------------------------
-- Closing summary of what this schema lets each role do.
-- ----------------------------------------------------------------------------
-- anon:          nothing.
-- authenticated: nothing on public.notifications -- no SELECT, INSERT, UPDATE
--                or DELETE, and no policy.
-- service_role:  BYPASSRLS covers the server's writes, pages, reads and purge.
-- ============================================================================
