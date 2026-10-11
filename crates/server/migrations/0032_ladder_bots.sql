-- ============================================================================
-- Migration 0032: the ladder bots (SPEC §9.5, §9.12, R1447-R1449; issue #638)
-- ============================================================================
-- A ranked Best of 1 or All Random ticket that waits past its fallback time
-- with no person to pair with plays a ladder bot (R1448), which the server
-- seats and plays itself (R1447) out of a pool it keeps (R1449). A bot is a
-- profile like a person's, so every table that names a seat by profile id
-- (matches, results, game_records, rated_games' sides) takes it as it is:
--
--   * public.profiles.kind -- 'human' or 'bot'. SERVER-ONLY: no column grant
--     to authenticated (0022 and 0028 grant a whitelist of columns, and this
--     one is not on it), so no client read tells a bot's profile from a
--     person's.
--   * public.bots -- one row per bot profile: its strength (an index into the
--     server's BOT_STRENGTHS), its think times' speed, its persona (R645), its
--     schedule, its Best-of-1 decks and the catalog version they were dealt
--     for, and when it was retired. Its rating is its R610 row in
--     public.bot_ratings, keyed by the profile id.
--
-- A bot has no auth.users row: Supabase Auth owns that table, and a bot never
-- signs in. So the foreign key 0001 put on profiles.id gives way to two
-- triggers that keep what it held for people:
--
--   * on_auth_user_deleted (auth.users, after delete) deletes the profile, as
--     the key's `on delete cascade` did, so account deletion (0012) runs as
--     before.
--   * profiles_need_auth_user (profiles, before insert) refuses a profile of
--     kind 'human' with no auth.users row, as the key did.
--
-- A bot owns no collection (R1449): 0002's app.grant_launch_collection_all,
-- which 0016 runs on every catalog stamp, grants people only from here on.
--
-- TRUST BOUNDARY: public.bots is server-only, as public.bot_ratings is (0022):
-- RLS on, NO policy, no grant to anon or authenticated. The server
-- (service_role, BYPASSRLS) is the only reader and writer.
--
-- Apply order: ... -> 0001 (`public.profiles`) -> ... -> 0022 (column grants,
-- `public.bot_ratings`) -> ... -> 0029 -> 0030 -> 0031 -> 0032.
-- Safe to re-apply: add-column-if-not-exists, drop-constraint-if-exists,
-- create-or-replace of the three functions, drop-and-create of both triggers,
-- create-if-not-exists, comment-on and revoke are all idempotent.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. public.profiles.kind (R1447).
-- ----------------------------------------------------------------------------
alter table public.profiles
  add column if not exists kind text not null default 'human';

alter table public.profiles drop constraint if exists profiles_kind_check;
alter table public.profiles
  add constraint profiles_kind_check check (kind in ('human', 'bot'));

comment on column public.profiles.kind is
  $$R1447: whether a person or a ladder bot holds the profile. Server-only: no
  client role is granted it, so nothing a client reads tells a bot's seat from
  a person's.$$;


-- ----------------------------------------------------------------------------
-- 2. The foreign key to auth.users gives way to two triggers.
-- ----------------------------------------------------------------------------
alter table public.profiles drop constraint if exists profiles_id_fkey;

-- -----------------------------------------------------------------------------
-- app.forget_auth_user(): deletes the profile of a deleted auth.users row, as
-- 0001's `references auth.users(id) on delete cascade` did. SECURITY DEFINER
-- for the reason 0001 gives app.handle_new_user: the role that deletes from
-- auth.users (Supabase Auth's own service role) does not own public.profiles.
-- -----------------------------------------------------------------------------
create or replace function app.forget_auth_user()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  delete from public.profiles where id = old.id;
  return old;
end;
$$;

comment on function app.forget_auth_user() is
  'R1447: deletes the public.profiles row of a deleted auth.users identity, '
  'the cascade the foreign key 0001 put on profiles.id gave before 0032 '
  'dropped it so a ladder bot can hold a profile with no auth user.';

revoke execute on function app.forget_auth_user() from public;

drop trigger if exists on_auth_user_deleted on auth.users;
create trigger on_auth_user_deleted
  after delete on auth.users
  for each row
  execute function app.forget_auth_user();

-- -----------------------------------------------------------------------------
-- app.profile_needs_auth_user(): refuses a person's profile with no
-- auth.users row, as the foreign key did. A bot's profile has none.
-- SECURITY DEFINER so that any role allowed to insert a profile can see
-- auth.users.
-- -----------------------------------------------------------------------------
create or replace function app.profile_needs_auth_user()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if new.kind = 'human' and not exists (select 1 from auth.users where id = new.id) then
    raise exception 'profile % has no auth user', new.id
      using errcode = '23503';
  end if;
  return new;
end;
$$;

comment on function app.profile_needs_auth_user() is
  'R1447: refuses a public.profiles row of kind human whose id is no '
  'auth.users identity, as the foreign key 0032 dropped did. A ladder bot''s '
  'profile (kind bot) has none.';

revoke execute on function app.profile_needs_auth_user() from public;

drop trigger if exists profiles_need_auth_user on public.profiles;
create trigger profiles_need_auth_user
  before insert on public.profiles
  for each row
  execute function app.profile_needs_auth_user();


-- ----------------------------------------------------------------------------
-- 3. public.bots (R1447-R1449).
-- ----------------------------------------------------------------------------
create table if not exists public.bots (
  profile_id            uuid primary key references public.profiles(id) on delete cascade,
  strength              smallint not null,
  speed                 double precision not null,
  persona               text not null,
  active_from_hour      smallint not null,
  active_hours          smallint not null,
  decks                 jsonb not null default '[]'::jsonb,
  decks_catalog_version text,
  retired_at            timestamptz,
  constraint bots_strength_check check (strength >= 0),
  constraint bots_speed_check check (speed > 0),
  constraint bots_persona_check check (persona in ('balanced', 'polite', 'bm', 'silent')),
  constraint bots_active_from_hour_check check (active_from_hour between 0 and 23),
  constraint bots_active_hours_check check (active_hours between 1 and 24)
);

comment on table public.bots is
  'R1447-R1449: one row per ladder bot profile (profiles.kind = bot): its '
  'strength (an index into the server''s BOT_STRENGTHS), think-time speed, '
  'persona (R645), schedule (UTC hours), Best-of-1 decks and the catalog '
  'version they were dealt for, and when it was retired. Its rating is its '
  'public.bot_ratings row (R610). Server-only.';

alter table public.bots enable row level security;

-- anon, authenticated: nothing. service_role: BYPASSRLS covers it (0002's
-- convention).
revoke all on public.bots from public, anon, authenticated;


-- ----------------------------------------------------------------------------
-- 4. A bot owns no collection (R1449).
-- ----------------------------------------------------------------------------
-- 0002's backfill, as it was, but over people's active profiles only: a bot's
-- profile is active from the start (it never redeems a code, so 0002's
-- activation trigger never grants it either), and 0016 runs this on every
-- catalog stamp.
create or replace function app.grant_launch_collection_all()
returns int
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_count    int := 0;
  v_profile  record;
begin
  -- SPEC §9.6 build order step 3: "Collection ledger; grant everything to
  -- everyone." Backfill for every currently active person's profile; safe to
  -- run repeatedly because app.grant_launch_collection is itself idempotent.
  -- R1449: a ladder bot (kind 'bot') owns nothing.
  for v_profile in
    select id from public.profiles where status = 'active' and kind = 'human'
  loop
    perform app.grant_launch_collection(v_profile.id);
    v_count := v_count + 1;
  end loop;

  return v_count;
end;
$$;

comment on function app.grant_launch_collection_all() is
  $$Backfill: calls app.grant_launch_collection for every active person's
  profile (kind human; a ladder bot owns no collection, R1449). Returns the
  number of profiles touched (visited), not the number that received new
  grants -- a repeat run touches the same count safely.$$;
