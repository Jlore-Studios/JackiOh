-- ============================================================================
-- Migration 0012: deleting an account
-- ============================================================================
-- Makes `delete from auth.users` work for every user. It cascades to
-- public.profiles (0001), and until this migration any profile that had tried
-- an invite code, minted one, been activated or played a match stopped the
-- delete with a foreign-key or append-only error. That delete is what the
-- Supabase dashboard runs, and what `DELETE /api/account` asks the auth admin
-- API for after the server has removed the profile itself
-- (apps/server/src/api/auth.ts, `ProfileStore.remove`).
--
-- What happens to each row that names the deleted profile:
--
--   * Rows that are only its own -- collection, collection_grants, decks,
--     trios, loadouts, tickets, tutorial_progress -- cascade, as their foreign
--     keys already said. collection_grants' append-only guard refused that
--     cascade; section 1 lets it through, and nothing else.
--   * code_attempts.profile_id: SET NULL, not cascade. The row keeps its
--     peppered IP hash, result and time, which the per-IP limit and the
--     circuit breaker count (SPEC §9.4 step 3, R106). Deleting them would let a
--     caller reset those counts by deleting an account. Migration 0013's purge
--     removes them once they are old.
--   * invite_codes.created_by: SET NULL. The code itself stays valid.
--   * Finished matches, their results, finished series and every logged
--     action: SET NULL on the deleted player's seat. The other player's
--     history, rating record and replay stay whole.
--   * A live match or an unfinished series: the delete is REFUSED, by the
--     check constraints in section 3. How a game ends belongs to the match
--     actor and the series rules; the API refuses first and says why.
--   * A room the profile opened that nobody joined, or a pairing that has not
--     started (a `matches` row with status 'open'): deleted with the profile,
--     by the trigger in section 4.
--
-- Constraint names are PostgreSQL's defaults for the single-column foreign
-- keys 0001, 0004 and 0009 declared inline. Each is dropped without
-- `if exists`, so a database where one is named differently fails this
-- migration (one transaction, rolled back) instead of keeping the old key.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The append-only guard admits one mutation: forgetting a deleted account.
-- ----------------------------------------------------------------------------
-- app.deny_row_mutation() (0001) now takes the name of the row's profile
-- column as its trigger argument. A foreign key's ON DELETE action runs after
-- the profile row is gone, so a DELETE of a row whose profile no longer exists
-- (a cascade), or an UPDATE that changes nothing but setting that column to
-- null (a set-null), is that action and passes. Every other update or delete
-- raises exactly as before, with the same message.
create or replace function app.deny_row_mutation()
returns trigger
language plpgsql
set search_path = ''
as $$
declare
  v_column  text := tg_argv[0];
  v_profile uuid;
begin
  if v_column is not null then
    v_profile := (to_jsonb(old) ->> v_column)::uuid;
    if v_profile is not null
       and not exists (select 1 from public.profiles p where p.id = v_profile) then
      if tg_op = 'DELETE' then
        return old;
      end if;
      if to_jsonb(new) = to_jsonb(old) || jsonb_build_object(v_column, null) then
        return new;
      end if;
    end if;
  end if;

  raise exception
    'append-only table %.% may not be updated or deleted (attempted %)',
    tg_table_schema, tg_table_name, tg_op;
end;
$$;

comment on function app.deny_row_mutation() is
  'SPEC-driven append-only guard: collection_grants and match_actions are '
  'audit ledgers that must never be edited or removed after the fact. '
  'Attached "before update or delete" by migrations 0002 and 0004, and since '
  '0012 given the row''s profile column as its argument: the foreign key''s own '
  'cascade or set-null for a profile that no longer exists is the one '
  'mutation it lets through.';

drop trigger if exists collection_grants_deny_mutation on public.collection_grants;
create trigger collection_grants_deny_mutation
  before update or delete on public.collection_grants
  for each row execute function app.deny_row_mutation('profile_id');

drop trigger if exists match_actions_deny_mutation on public.match_actions;
create trigger match_actions_deny_mutation
  before update or delete on public.match_actions
  for each row execute function app.deny_row_mutation('player_id');


-- ----------------------------------------------------------------------------
-- 2. The foreign keys that had no ON DELETE action.
-- ----------------------------------------------------------------------------
alter table public.code_attempts drop constraint code_attempts_profile_id_fkey;
alter table public.code_attempts
  add constraint code_attempts_profile_id_fkey
  foreign key (profile_id) references public.profiles(id) on delete set null;

alter table public.invite_codes drop constraint invite_codes_created_by_fkey;
alter table public.invite_codes
  add constraint invite_codes_created_by_fkey
  foreign key (created_by) references public.profiles(id) on delete set null;

alter table public.match_actions drop constraint match_actions_player_id_fkey;
alter table public.match_actions
  add constraint match_actions_player_id_fkey
  foreign key (player_id) references public.profiles(id) on delete set null;

alter table public.matches alter column p1_profile_id drop not null;
alter table public.matches drop constraint matches_p1_profile_id_fkey;
alter table public.matches
  add constraint matches_p1_profile_id_fkey
  foreign key (p1_profile_id) references public.profiles(id) on delete set null;
alter table public.matches drop constraint matches_p2_profile_id_fkey;
alter table public.matches
  add constraint matches_p2_profile_id_fkey
  foreign key (p2_profile_id) references public.profiles(id) on delete set null;

alter table public.results alter column p1_profile_id drop not null;
alter table public.results alter column p2_profile_id drop not null;
alter table public.results drop constraint results_p1_profile_id_fkey;
alter table public.results
  add constraint results_p1_profile_id_fkey
  foreign key (p1_profile_id) references public.profiles(id) on delete set null;
alter table public.results drop constraint results_p2_profile_id_fkey;
alter table public.results
  add constraint results_p2_profile_id_fkey
  foreign key (p2_profile_id) references public.profiles(id) on delete set null;
alter table public.results drop constraint results_winner_profile_id_fkey;
alter table public.results
  add constraint results_winner_profile_id_fkey
  foreign key (winner_profile_id) references public.profiles(id) on delete set null;

alter table public.series alter column p1_profile_id drop not null;
alter table public.series alter column p2_profile_id drop not null;
alter table public.series drop constraint series_p1_profile_id_fkey;
alter table public.series
  add constraint series_p1_profile_id_fkey
  foreign key (p1_profile_id) references public.profiles(id) on delete set null;
alter table public.series drop constraint series_p2_profile_id_fkey;
alter table public.series
  add constraint series_p2_profile_id_fkey
  foreign key (p2_profile_id) references public.profiles(id) on delete set null;


-- ----------------------------------------------------------------------------
-- 3. Which seats may be empty: only those of a game that is over.
-- ----------------------------------------------------------------------------
-- Replaces 0004's `matches_p2_required_when_not_open_check` (status = 'open'
-- or p2 set) and its not-null p1. An open room needs its host, a live match
-- both players; only an 'over' match may have lost a seat to a deleted
-- account. So a delete that would empty a seat of a live match raises here.
alter table public.matches drop constraint matches_p2_required_when_not_open_check;
alter table public.matches
  add constraint matches_seats_filled_unless_over_check check (
    status = 'over'
    or (p1_profile_id is not null and (status = 'open' or p2_profile_id is not null))
  );

-- 0004's `p1 is distinct from p2` is false for two empty seats, which a match
-- whose players both deleted their accounts has. `<>` is null there, and a
-- check passes on null; for two filled seats the two tests agree.
alter table public.matches drop constraint matches_players_differ_check;
alter table public.matches
  add constraint matches_players_differ_check check (p1_profile_id <> p2_profile_id);

-- 0009's `series_players_differ_check` is already `<>`. A series that is not
-- over keeps both players, as a live match does.
alter table public.series
  add constraint series_players_seated_unless_over_check check (
    status = 'over' or (p1_profile_id is not null and p2_profile_id is not null)
  );


-- ----------------------------------------------------------------------------
-- 4. A room nobody joined goes with its host.
-- ----------------------------------------------------------------------------
-- An 'open' matches row is a room waiting for its second player (0004), or the
-- reservation a queue pairing writes before the match starts (src/db/store.ts,
-- `tickets.claimPair`). Nothing in it survives the profile: it has no actions
-- and no result. Rows that expired unjoined stay 'open' for good, so without
-- this every player who ever opened an unjoined room would hit the check above.
-- The one open row it leaves is the profile's own current match: a pairing
-- that is starting right now. Deleting that row would set this profile's
-- current_match_id to null while the profile is being deleted, which
-- PostgreSQL refuses; the delete fails on the check above instead, as for a
-- live match.
-- SECURITY INVOKER: it runs as whoever deletes the profile (the server's
-- service_role, or the table owner inside auth.users' cascade).
create or replace function app.release_open_matches()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  delete from public.matches
   where status = 'open'
     and (p1_profile_id = old.id or p2_profile_id = old.id)
     and id is distinct from old.current_match_id;
  return old;
end;
$$;

comment on function app.release_open_matches() is
  'Migration 0012: deletes the open rooms and unstarted pairings of a profile '
  'being deleted, before its foreign keys run.';

revoke execute on function app.release_open_matches() from public;

drop trigger if exists profiles_release_open_matches on public.profiles;
create trigger profiles_release_open_matches
  before delete on public.profiles
  for each row execute function app.release_open_matches();
