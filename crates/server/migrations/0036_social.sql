-- ============================================================================
-- Migration 0036: Play together (issue #936) -- friends, blocks, presence,
-- challenges, chat, shared replays and the spectator setting
-- ============================================================================
-- Serves the fullsend SPEC's S7 (B43-B53; issues #937, #938, #941).
--
--   * public.friend_requests -- B43, B44: a pending request from one account to
--     another, one per direction and pair, never to oneself. Accepting,
--     declining or cancelling deletes it (B45).
--   * public.friendships -- B45: one row per pair of friends, the pair in
--     order (a < b), so a pair has exactly one row whichever side asked.
--   * public.blocks -- B46: who blocked whom. A block removes the friendship
--     and every request either way, and stops new requests, challenges, chat
--     and shares either way (the server's rule; the rows are its record).
--   * public.presence -- B47: each account's last heartbeat.
--   * public.challenges -- B48, B49: a challenge to a friend and the room code
--     it opened, in the mode the room plays, open until it is accepted (the
--     room joined), declined, cancelled or expired.
--   * public.chat_messages -- B53: a message between two friends, 1 to 500
--     characters (CHAT_MAX_CHARS), and when its recipient read it. Purged
--     CHAT_RETENTION_DAYS after it was sent, its reports with it.
--   * public.chat_reports -- B53: a report of a message, one per reporter.
--   * public.replay_shares -- B52: a finished match's replay a seat holder
--     shared with a friend. match_id names a match the server checks; it has
--     no foreign key, so a share outlives nothing it should not (the server
--     answers a share only while the friendship stands).
--   * public.profiles.allow_spectators -- B50: whether friends may watch the
--     account's live unranked games. On by default.
--
-- TRUST BOUNDARY: every write goes through the server (service_role,
-- BYPASSRLS). RLS is on for every table; a client role may read only the rows
-- naming its own profile (a request or a challenge it sent or received, a
-- friendship it is in, a block it made, a message it sent or received, a
-- report it made, a share it made or received) and writes nothing. presence
-- has no policy and no client grant: who is online is read through the API,
-- for friends only. Every row cascades with either profile it names, so
-- deleting an account removes them (0012).
--
-- Apply order: ... -> 0034 -> 0035 -> 0036 (this file).
-- Safe to re-apply: create-if-not-exists, add-column-if-not-exists,
-- drop-and-create policies, comment-on, revoke and grant.
-- ============================================================================

create table if not exists public.friend_requests (
  id           uuid primary key default gen_random_uuid(),
  from_profile uuid not null references public.profiles(id) on delete cascade,
  to_profile   uuid not null references public.profiles(id) on delete cascade,
  created_at   timestamptz not null default now(),
  unique (from_profile, to_profile),
  check (from_profile <> to_profile)
);

create table if not exists public.friendships (
  a     uuid not null references public.profiles(id) on delete cascade,
  b     uuid not null references public.profiles(id) on delete cascade,
  since timestamptz not null default now(),
  primary key (a, b),
  check (a < b)
);

create table if not exists public.blocks (
  blocker    uuid not null references public.profiles(id) on delete cascade,
  blocked    uuid not null references public.profiles(id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (blocker, blocked),
  check (blocker <> blocked)
);

create table if not exists public.presence (
  profile_id uuid primary key references public.profiles(id) on delete cascade,
  last_seen  timestamptz not null
);

create table if not exists public.challenges (
  id           uuid primary key default gen_random_uuid(),
  from_profile uuid not null references public.profiles(id) on delete cascade,
  to_profile   uuid not null references public.profiles(id) on delete cascade,
  room_code    text not null,
  mode         text not null,
  created_at   timestamptz not null default now(),
  expires_at   timestamptz not null,
  state        text not null default 'open'
               check (state in ('open', 'accepted', 'declined', 'cancelled', 'expired'))
);

create table if not exists public.chat_messages (
  id           uuid primary key default gen_random_uuid(),
  from_profile uuid not null references public.profiles(id) on delete cascade,
  to_profile   uuid not null references public.profiles(id) on delete cascade,
  body         text not null check (char_length(body) between 1 and 500),
  created_at   timestamptz not null default now(),
  read_at      timestamptz
);

create table if not exists public.chat_reports (
  id         uuid primary key default gen_random_uuid(),
  message_id uuid not null references public.chat_messages(id) on delete cascade,
  reporter   uuid not null references public.profiles(id) on delete cascade,
  reason     text not null,
  created_at timestamptz not null default now(),
  unique (message_id, reporter)
);

create table if not exists public.replay_shares (
  match_id   uuid not null,
  sharer     uuid not null references public.profiles(id) on delete cascade,
  friend     uuid not null references public.profiles(id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (match_id, sharer, friend)
);

alter table public.profiles add column if not exists allow_spectators boolean not null default true;

-- ----------------------------------------------------------------------------
-- Indexes: each read the server makes, by the profile it is about.
-- ----------------------------------------------------------------------------

-- The incoming half of an account's requests (the outgoing half is the unique key's prefix).
create index if not exists friend_requests_to_idx on public.friend_requests (to_profile);
-- The friendships an account is the `b` of (the `a` half is the primary key's prefix).
create index if not exists friendships_b_idx on public.friendships (b);
-- Whether an account is blocked by another, read before every request, challenge and message.
create index if not exists blocks_blocked_idx on public.blocks (blocked);
-- B48: one open challenge per challenger and friend, so two racing challenges cannot both land.
-- The server marks a lapsed one expired before it adds the next.
create unique index if not exists challenges_open_pair_key
  on public.challenges (from_profile, to_profile) where state = 'open';
create index if not exists challenges_to_idx on public.challenges (to_profile, state);
-- B49: the open challenge a room was opened for, read as the room is joined.
create index if not exists challenges_room_code_idx on public.challenges (room_code) where state = 'open';
-- B53: a conversation, newest first, and an account's unread messages.
create index if not exists chat_messages_pair_idx
  on public.chat_messages (from_profile, to_profile, created_at desc);
create index if not exists chat_messages_unread_idx
  on public.chat_messages (to_profile) where read_at is null;
-- B53: the retention purge's cutoff.
create index if not exists chat_messages_created_at_idx on public.chat_messages (created_at);
create index if not exists chat_reports_reporter_idx on public.chat_reports (reporter);
-- B52: what was shared with an account.
create index if not exists replay_shares_friend_idx on public.replay_shares (friend);

-- ----------------------------------------------------------------------------
-- Comments
-- ----------------------------------------------------------------------------

comment on table public.friend_requests is
  $$B43, B44 (#936): a pending friend request, one per direction and pair, never
  to oneself. Accepted, declined or cancelled, it is deleted (B45).$$;

comment on table public.friendships is
  $$B45 (#936): two friends, the pair in order (a < b), so a pair has one row.$$;

comment on table public.blocks is
  $$B46 (#936): who blocked whom. A block ends the friendship and every request
  either way, and stops requests, challenges, chat and shares either way.$$;

comment on table public.presence is
  $$B47 (#936): each account's last heartbeat; online while it is at most
  PRESENCE_TTL_MS old. Server-only (service_role).$$;

comment on table public.challenges is
  $$B48, B49 (#936): a challenge to a friend and the room it opened. Accepted
  when the room is joined; declined, cancelled or expired, the room is freed.$$;

comment on column public.challenges.mode is
  $$B48: the mode of the room the challenge opened: bo1, bo3 or random.$$;

comment on table public.chat_messages is
  $$B53 (#936): a message between two friends, filtered by the username word
  filter before it is stored. Purged after CHAT_RETENTION_DAYS.$$;

comment on table public.chat_reports is
  $$B53 (#936): a report of a chat message, one per reporter.$$;

comment on table public.replay_shares is
  $$B52 (#936): a finished match's replay shared by a seat holder with a
  friend, answered only while they stay friends and neither blocks the other.$$;

comment on column public.profiles.allow_spectators is
  $$B50 (#936): whether friends may watch this account's live unranked games.$$;

-- ----------------------------------------------------------------------------
-- Row-level security: a client reads the rows naming its own profile, and
-- writes none (RLS denies by default; no write policy exists).
-- ----------------------------------------------------------------------------

alter table public.friend_requests enable row level security;
alter table public.friendships     enable row level security;
alter table public.blocks          enable row level security;
alter table public.presence        enable row level security;
alter table public.challenges      enable row level security;
alter table public.chat_messages   enable row level security;
alter table public.chat_reports    enable row level security;
alter table public.replay_shares   enable row level security;

drop policy if exists friend_requests_select_own on public.friend_requests;
create policy friend_requests_select_own
  on public.friend_requests
  for select
  to authenticated
  using (from_profile = app.current_profile_id() or to_profile = app.current_profile_id());

drop policy if exists friendships_select_own on public.friendships;
create policy friendships_select_own
  on public.friendships
  for select
  to authenticated
  using (a = app.current_profile_id() or b = app.current_profile_id());

drop policy if exists blocks_select_own on public.blocks;
create policy blocks_select_own
  on public.blocks
  for select
  to authenticated
  using (blocker = app.current_profile_id());

-- presence: no policy. Who is online is the server's to say, to friends only.

drop policy if exists challenges_select_own on public.challenges;
create policy challenges_select_own
  on public.challenges
  for select
  to authenticated
  using (from_profile = app.current_profile_id() or to_profile = app.current_profile_id());

drop policy if exists chat_messages_select_own on public.chat_messages;
create policy chat_messages_select_own
  on public.chat_messages
  for select
  to authenticated
  using (from_profile = app.current_profile_id() or to_profile = app.current_profile_id());

drop policy if exists chat_reports_select_own on public.chat_reports;
create policy chat_reports_select_own
  on public.chat_reports
  for select
  to authenticated
  using (reporter = app.current_profile_id());

drop policy if exists replay_shares_select_own on public.replay_shares;
create policy replay_shares_select_own
  on public.replay_shares
  for select
  to authenticated
  using (sharer = app.current_profile_id() or friend = app.current_profile_id());

-- ----------------------------------------------------------------------------
-- Privileges. service_role: BYPASSRLS covers reads/writes to these tables. A
-- client role may read the seven tables above through their policies, and
-- nothing of presence.
-- ----------------------------------------------------------------------------

revoke all on public.friend_requests from public, anon, authenticated;
revoke all on public.friendships     from public, anon, authenticated;
revoke all on public.blocks          from public, anon, authenticated;
revoke all on public.presence        from public, anon, authenticated;
revoke all on public.challenges      from public, anon, authenticated;
revoke all on public.chat_messages   from public, anon, authenticated;
revoke all on public.chat_reports    from public, anon, authenticated;
revoke all on public.replay_shares   from public, anon, authenticated;

grant select on public.friend_requests to authenticated;
grant select on public.friendships     to authenticated;
grant select on public.blocks          to authenticated;
grant select on public.challenges      to authenticated;
grant select on public.chat_messages   to authenticated;
grant select on public.chat_reports    to authenticated;
grant select on public.replay_shares   to authenticated;

-- 0022 holds profiles to a column whitelist for clients; the new setting is the
-- account's own to read, like the rest of its row but the hidden rating.
grant select (allow_spectators) on public.profiles to authenticated;
