-- ============================================================================
-- Migration 0023: Glitch (issue #170) — the boards outcome and the void outcome
-- ============================================================================
-- Serves SPEC §7 (the hidden token Glitch) and SPEC §11 R678, R679.
--
--   * public.matches.p1_glitch_board / p2_glitch_board -- R678: the boards of
--     two OTHER players' last server games, sampled from public.last_boards
--     when the match is created, that a Glitch's boards outcome puts on the
--     field. Like 0017's p1_last_board / p2_last_board they are an input of the
--     engine's createGame beside the decks, frozen so a restarted server folds
--     the same game (SPEC §9.3, §9.5). '[]' for every older row, for an open
--     room, and for a seat with no other board to sample: the empty board such
--     a match was always played with.
--   * app.forget_voided_match(p_match_id) -- R679: a Glitch's void outcome
--     ends the game with no winner, and the server keeps no trace of it but a
--     log line. The match row goes, and with it its action log, which is
--     append-only (0004, app.deny_row_mutation). Like app.purge_expired_rows
--     (0013) this function sets `jackioh.retention_purge = 'on'` for its own
--     deletes only and clears it before it returns, so the guard's one
--     exception covers it and no other statement inherits it. It deletes only a
--     'live' match with no result: a finished game is never erased.
--     profiles.current_match_id and tickets.match_id are `on delete set null`
--     (0004), so both players are let go with the row; series.next_match_id has
--     no foreign key (0009), so a voided series game is simply started again.
--
-- TRUST BOUNDARY: unchanged. public.matches stays unreachable for client roles
-- (0004), and the function is executable by postgres and service_role only.
--
-- Apply order: ... -> 0017 (`matches.p*_last_board`) -> ... -> 0022 -> 0023.
-- Safe to re-apply: add-column-if-not-exists, drop-constraint-if-exists-then-
-- add, create-or-replace.
-- ============================================================================

alter table public.matches add column if not exists p1_glitch_board jsonb not null default '[]'::jsonb;
alter table public.matches add column if not exists p2_glitch_board jsonb not null default '[]'::jsonb;

alter table public.matches drop constraint if exists matches_glitch_boards_check;
alter table public.matches add constraint matches_glitch_boards_check check (
  jsonb_typeof(p1_glitch_board) = 'array'
  and jsonb_typeof(p2_glitch_board) = 'array'
  and not jsonb_path_exists(p1_glitch_board, '$[*] ? (!(@.defId.type() == "string" && @.radiant.type() == "boolean"))')
  and not jsonb_path_exists(p2_glitch_board, '$[*] ? (!(@.defId.type() == "string" && @.radiant.type() == "boolean"))')
);

comment on column public.matches.p1_glitch_board is
  $$R678, SPEC §9.3: the board of another player's last server game that a
  Glitch puts on p1's side, sampled as this match started and frozen so a
  rebuilt actor folds the same game. p2_glitch_board is p2's.$$;


-- ----------------------------------------------------------------------------
-- app.forget_voided_match -- R679's "as if it never existed".
-- ----------------------------------------------------------------------------
-- SECURITY INVOKER: the server calls it as service_role, which may delete from
-- both tables already. Returns whether a match went.
create or replace function app.forget_voided_match(p_match_id uuid)
returns boolean
language plpgsql
set search_path = ''
as $$
declare
  v_gone boolean;
begin
  if not exists (
    select 1 from public.matches m
     where m.id = p_match_id
       and m.status = 'live'
       and not exists (select 1 from public.results r where r.match_id = p_match_id)
  ) then
    return false;
  end if;

  -- Transaction-local, and cleared below; a failed delete aborts the transaction.
  perform set_config('jackioh.retention_purge', 'on', true);

  delete from public.match_actions a where a.match_id = p_match_id;
  delete from public.matches m where m.id = p_match_id and m.status = 'live';
  v_gone := found;

  perform set_config('jackioh.retention_purge', 'off', true);
  return v_gone;
end;
$$;

comment on function app.forget_voided_match(uuid) is
  'Migration 0023 (R679): deletes a live match a Glitch voided, with its action '
  'log; its players are let go by the foreign keys. Never a finished match or '
  'one with a result. Called by the server (src/db/store.ts matches.forgetVoided).';

revoke execute on function app.forget_voided_match(uuid) from public;
grant execute on function app.forget_voided_match(uuid) to postgres, service_role;

-- anon, authenticated: nothing; matches stays unreachable (0004).
