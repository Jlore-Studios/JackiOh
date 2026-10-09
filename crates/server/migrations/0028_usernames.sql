-- ============================================================================
-- Migration 0028: usernames (SPEC §9.1, §9.4, R1432-R1436; issue #579)
-- ============================================================================
-- Every profile gets a username: a base name plus, when the base is taken, a
-- numeric tag (`Max`, `Max#1`, `Max#2`). A username is only a display label:
-- every table, request and log keeps naming a player by profile id, and no
-- table but this one stores a name (R1436). The columns are:
--
--   * username_base        -- the base name in its stored, NFKC form (R1432);
--   * username_key         -- the key two names clash on, the NFKC case fold
--                             of the base, computed by the server in Rust
--                             (`username::username_key`), so no collation here
--                             ever decides (R1434);
--   * username_tag         -- the tag, 1 or more; null for a bare name;
--   * username_changed_at  -- the last change, which starts the 24-hour
--                             cooldown; null while the profile still holds
--                             the default it was given (R1435);
--   * username_prompted    -- whether the prompt after activation has been
--                             answered, by a pick or a skip (R1435).
--
-- A unique index on (username_key, coalesce(username_tag, 0)) holds R1434: one
-- bare holder of a base and one holder of each tag. A new account starts as
-- the lowest free `Player#n`, always tagged: `app.assign_default_username`, a
-- BEFORE INSERT trigger, names every row that arrives without a name (both
-- `app.handle_new_user`'s and the server's own `profiles.create`), under a
-- transaction-level advisory lock on the key `player`, the same lock the
-- server takes to claim `Player` (`USERNAME_LOCK_SQL`, src/db/pg.rs), so two
-- sign-ups at the same moment, or a sign-up and a claim of `Player`, never
-- share a tag. A tag is freed when its holder renames or deletes their
-- account, and the lowest free one goes to the next taker.
--
-- BACKFILL: every existing profile gets a `Player#n` default in order of
-- sign-up (created_at, then id), so the earliest accounts hold the lowest
-- numbers, and stays unprompted, so its player meets the prompt once on their
-- next sign-in (R1434, R1435). The backfill bumps each row's updated_at, as
-- any update of a profile does. `display_name` (nullable, unchecked, written
-- only by tests and seeding) is replaced by these columns: no server from this
-- one on reads or writes it. It is not dropped here. A deploy migrates before
-- it serves, and the deploy before it keeps serving until the new one passes
-- its health check (docs/architecture.md), so for that while, and for good if
-- the new one never does, the previous server runs on this schema and selects
-- `display_name` with every profile it reads. A later migration drops it, once
-- no deployed server reads it. Everything else here only adds, and the
-- trigger names the rows the previous server inserts.
--
-- TRUST BOUNDARY: a username is written only through the API (`PUT
-- /api/username`), never by a client. `authenticated` keeps 0022's read
-- whitelist on its own row (RLS) and gains the five new columns in it; no
-- write grant exists for any column of profiles (0001, 0022).
--
-- Apply order: ... -> 0026 -> 0027 -> 0028.
-- Safe to re-apply: add-column-if-not-exists, a backfill that touches only
-- rows still without a name (numbered after the highest `Player` tag already
-- held), set-not-null, drop-and-add of the check, create-index-if-not-exists,
-- create-or-replace of the function, drop-and-create of the trigger,
-- drop-column-if-exists, comment-on and grant are all idempotent.
-- ============================================================================

alter table public.profiles add column if not exists username_base text;
alter table public.profiles add column if not exists username_key text;
alter table public.profiles add column if not exists username_tag int;
alter table public.profiles add column if not exists username_changed_at timestamptz;
alter table public.profiles add column if not exists username_prompted boolean not null default false;

with held as (
  select coalesce(max(username_tag), 0) as top
    from public.profiles
   where username_key = 'player'
), unnamed as (
  select id, row_number() over (order by created_at, id) as n
    from public.profiles
   where username_key is null
)
update public.profiles p
   set username_base = 'Player',
       username_key = 'player',
       username_tag = (held.top + unnamed.n)::int,
       username_changed_at = null,
       username_prompted = false
  from unnamed, held
 where p.id = unnamed.id;

alter table public.profiles alter column username_base set not null;
alter table public.profiles alter column username_key set not null;

alter table public.profiles drop constraint if exists profiles_username_tag_positive;
alter table public.profiles add constraint profiles_username_tag_positive
  check (username_tag is null or username_tag >= 1);

create unique index if not exists profiles_username_unique
  on public.profiles (username_key, coalesce(username_tag, 0));

comment on column public.profiles.username_base is
  $$R1432: the username's base name in its stored (NFKC) form, `Max` of
  `Max#3`. Written only by the server; every read that shows a player joins it
  in from here (R1436).$$;

comment on column public.profiles.username_key is
  $$R1434: the key two base names clash on, the NFKC case fold of the base,
  computed by the server in Rust (`Max`, `max` and `MAX` are `max`).$$;

comment on column public.profiles.username_tag is
  $$R1434: the tag, 1 or more, carried when the base was already taken; null
  for a bare name. A new account's default `Player#n` always has one.$$;

comment on column public.profiles.username_changed_at is
  $$R1435: the last change of username, which starts the 24-hour cooldown;
  null while the profile still holds the default it was given.$$;

comment on column public.profiles.username_prompted is
  $$R1435: whether the username prompt after activation has been answered, by
  a pick or a skip. It shows at each sign-in until then.$$;

-- -----------------------------------------------------------------------------
-- app.assign_default_username(): R1434's default, the lowest free `Player#n`
-- from 1, for every row inserted without a name. The lock is the server's lock
-- on the key `player` (src/db/pg.rs, `USERNAME_LOCK_SQL`), so the tag it reads
-- as free stays free until this transaction commits. SECURITY DEFINER for the
-- reason 0001 gives app.handle_new_user: the role inserting during signup does
-- not own public.profiles.
-- -----------------------------------------------------------------------------

create or replace function app.assign_default_username()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  if new.username_key is null then
    perform pg_advisory_xact_lock(hashtextextended('jackioh.username:player', 0));
    new.username_base := 'Player';
    new.username_key := 'player';
    new.username_tag := (
      select min(n)::int
        from generate_series(
               1,
               (select count(*) from public.profiles where username_key = 'player') + 1
             ) as n
       where not exists (
         select 1 from public.profiles
          where username_key = 'player' and coalesce(username_tag, 0) = n
       )
    );
  end if;
  return new;
end;
$$;

comment on function app.assign_default_username() is
  'R1434: names every new public.profiles row the lowest free Player#n, under '
  'the advisory lock the server takes on the key player.';

revoke execute on function app.assign_default_username() from public;

drop trigger if exists profiles_assign_default_username on public.profiles;
create trigger profiles_assign_default_username
  before insert on public.profiles
  for each row
  execute function app.assign_default_username();

comment on column public.profiles.display_name is
  'Unused since migration 0028 (R1434): the username columns replace it. Kept '
  'only for the server deployed before 0028, which reads it; a later migration '
  'drops it.';

grant select (username_base, username_key, username_tag, username_changed_at, username_prompted)
  on public.profiles to authenticated;
