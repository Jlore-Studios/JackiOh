-- ============================================================================
-- Migration 0080: The dev dashboard: its audit, hand-set ratings and posts
-- (SPEC §14.10, §14.11, §14.13; R3074000, R3074001, R3074003; issue #3074)
-- ============================================================================
-- Serves R3074000. The dev dashboard (apps/admin) lets a dev account
-- (DEV_PROFILE_IDS, §14.10) do what /account's Dev tools do, and three things
-- that reach other players: hand-set its own rating on a ladder (R3074001),
-- read the whole ladder with each row marked person or bot (R3074002), and post
-- news that pays no reward (R3074003). Every write goes through a server route
-- that checks the list itself (src/api/admin.rs, src/api/dev.rs). This file:
--
--   1. Creates public.admin_actions: one row per dev or admin action, who did
--      it, which route, the request, the values before and after, and when.
--      Append-only (0012's guard): a row is never edited or removed. When its
--      account is deleted, the row stays and forgets who did it
--      (`on delete set null`, the one update the guard lets through), so the
--      record of what was done to the game outlives the account.
--   2. Creates public.hand_set_ratings: the ladders on which an account holds
--      a rating it set by hand, each with the rating its games had earned when
--      it was first set, which a reset puts back (R3074001). A current-state
--      table: a reset deletes the row. While a row stands on a strict server
--      (DEV_STRICT), the account is off the public ladder.
--   3. Creates public.rating_grants: the rating ledger, one append-only row per
--      hand-set rating or reset, its reason `dev` and marked `dev` (§14.11),
--      with the whole Glicko-2 triple before and after.
--   4. Creates public.news_posts: the posts a dev wrote from the dashboard,
--      read alongside crates/server/news.json by `GET /api/news` (§14.13). A
--      post is plain text and pays no reward (R3074003); taking one down sets
--      `taken_down_at` and keeps the row.
--
-- TRUST BOUNDARY: no client role reads or writes any of the four. The
-- hidden rating is server-only (R612), and so is everything a dev did; the
-- dashboard reads them through the server's dev-only routes. The server
-- (service_role, BYPASSRLS; src/admin/store.rs) is the only reader and writer.
--
-- Apply order: ... -> 0012 (deny_row_mutation's profile argument) -> ... ->
-- 0022 (profiles' rating) -> ... -> 0076 (two ladders) -> 0077 -> 0078 -> 0079
-- -> 0080 (this file). Safe to re-apply: create-if-not-exists, and every
-- trigger, check and index dropped if it exists and made again.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. public.admin_actions (R3074000).
-- ----------------------------------------------------------------------------
create table if not exists public.admin_actions (
  id           bigint generated always as identity primary key,
  profile_id   uuid references public.profiles(id) on delete set null,
  route        text not null,
  request      jsonb not null,
  before_value jsonb not null,
  after_value  jsonb not null,
  at           timestamptz not null
);

alter table public.admin_actions drop constraint if exists admin_actions_route_check;
alter table public.admin_actions
  add constraint admin_actions_route_check check (route ~ '^(GET|POST|PUT|PATCH|DELETE) /api/(dev|admin)/');

drop index if exists public.admin_actions_profile_idx;
create index admin_actions_profile_idx on public.admin_actions (profile_id);
drop index if exists public.admin_actions_at_idx;
create index admin_actions_at_idx on public.admin_actions (at desc, id desc);

alter table public.admin_actions enable row level security;

drop trigger if exists admin_actions_deny_mutation on public.admin_actions;
create trigger admin_actions_deny_mutation before update or delete on public.admin_actions
  for each row execute function app.deny_row_mutation('profile_id');

revoke all on public.admin_actions from public, anon, authenticated;


-- ----------------------------------------------------------------------------
-- 2. public.hand_set_ratings (R3074001).
-- ----------------------------------------------------------------------------
create table if not exists public.hand_set_ratings (
  profile_id        uuid not null references public.profiles(id) on delete cascade,
  format            text not null,
  earned_rating     double precision not null,
  earned_deviation  double precision not null,
  earned_volatility double precision not null,
  set_at            timestamptz not null,
  primary key (profile_id, format)
);

alter table public.hand_set_ratings drop constraint if exists hand_set_ratings_format_check;
alter table public.hand_set_ratings
  add constraint hand_set_ratings_format_check check (format in ('constructed', 'random'));

alter table public.hand_set_ratings enable row level security;

revoke all on public.hand_set_ratings from public, anon, authenticated;


-- ----------------------------------------------------------------------------
-- 3. public.rating_grants (R3074001, §14.11).
-- ----------------------------------------------------------------------------
create table if not exists public.rating_grants (
  id                bigint generated always as identity primary key,
  profile_id        uuid not null references public.profiles(id) on delete cascade,
  format            text not null,
  kind              text not null,
  rating_before     double precision not null,
  deviation_before  double precision not null,
  volatility_before double precision not null,
  rating_after      double precision not null,
  deviation_after   double precision not null,
  volatility_after  double precision not null,
  reason            text not null,
  dev               boolean not null,
  at                timestamptz not null
);

alter table public.rating_grants drop constraint if exists rating_grants_format_check;
alter table public.rating_grants
  add constraint rating_grants_format_check check (format in ('constructed', 'random'));
alter table public.rating_grants drop constraint if exists rating_grants_kind_check;
alter table public.rating_grants
  add constraint rating_grants_kind_check check (kind in ('set', 'reset'));
alter table public.rating_grants drop constraint if exists rating_grants_reason_check;
alter table public.rating_grants
  add constraint rating_grants_reason_check check (reason in ('dev'));
alter table public.rating_grants drop constraint if exists rating_grants_dev_check;
alter table public.rating_grants
  add constraint rating_grants_dev_check check (reason <> 'dev' or dev);

drop index if exists public.rating_grants_profile_idx;
create index rating_grants_profile_idx on public.rating_grants (profile_id, format, at);

alter table public.rating_grants enable row level security;

drop trigger if exists rating_grants_deny_mutation on public.rating_grants;
create trigger rating_grants_deny_mutation before update or delete on public.rating_grants
  for each row execute function app.deny_row_mutation('profile_id');

revoke all on public.rating_grants from public, anon, authenticated;


-- ----------------------------------------------------------------------------
-- 4. public.news_posts (R3074003).
-- ----------------------------------------------------------------------------
create table if not exists public.news_posts (
  id            text primary key,
  title         text not null,
  summary       text not null,
  posted_at     timestamptz not null,
  posted_by     uuid references public.profiles(id) on delete set null,
  taken_down_at timestamptz
);

-- A post's id is a slug of its own prefix, so it never names a news.json entry and a claim of it
-- fits news_claims_news_check (0077).
alter table public.news_posts drop constraint if exists news_posts_id_check;
alter table public.news_posts
  add constraint news_posts_id_check check (id ~ '^post-[a-z0-9]+(-[a-z0-9]+)*$');
alter table public.news_posts drop constraint if exists news_posts_text_check;
alter table public.news_posts
  add constraint news_posts_text_check check (btrim(title) <> '' and btrim(summary) <> '');

drop index if exists public.news_posts_posted_by_idx;
create index news_posts_posted_by_idx on public.news_posts (posted_by);
drop index if exists public.news_posts_posted_at_idx;
create index news_posts_posted_at_idx on public.news_posts (posted_at desc, id);

alter table public.news_posts enable row level security;

revoke all on public.news_posts from public, anon, authenticated;


comment on table public.admin_actions is
  $$R3074000: one row per dev or admin action (§14.10): who, which route, the
  request, the values before and after, and when. Append-only; the server alone
  writes and reads it, and the dev dashboard reads it through the server. An
  account's deletion forgets who, and keeps the row.$$;
comment on table public.hand_set_ratings is
  $$R3074001: the ladders on which an account holds a hand-set rating, each with
  the rating its games had earned when it was first set, which a reset puts
  back. Server-only, as the rating is (R612).$$;
comment on table public.rating_grants is
  $$R3074001, §14.11: the rating ledger, one row per hand-set rating or reset,
  reason 'dev' and marked dev, with the Glicko-2 triple before and after.
  Append-only; server-only (R612).$$;
comment on table public.news_posts is
  $$R3074003: the news posts a dev wrote from the dev dashboard, plain text with
  no reward, read alongside crates/server/news.json (§14.13). Taking one down
  sets taken_down_at. Server-only: players read them through GET /api/news.$$;
