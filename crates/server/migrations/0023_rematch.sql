-- ============================================================================
-- Migration 0023: rematch mode and stakes (SPEC §9.5, R672)
-- ============================================================================
-- Serves the rematch a finished non-series match offers: the new match is
-- made by no ticket, room or series, so nothing `matches.modeOf` reads names
-- its mode. The rematch therefore states both on its own row:
--
--   - `mode`: the mode the rematch replays ('bo1' or 'random'; a series game
--     never offers one). Null on every older row, whose mode keeps coming
--     from its tickets, room or series exactly as before.
--   - `stake`: 2 on a double-or-nothing rematch, null (a normal game) anywhere
--     else. Only a ranked rematch is ever dealt 2 (`double_requires_ranked`).
--
-- Both are nullable with no default, so existing rows are untouched and no
-- backfill runs. `store.ts` reads a null mode as "derive it" and a null stake
-- as 1; `modeOf` prefers the row's own mode before the tickets, room and
-- series fallbacks. Safe to re-apply: add-column-if-not-exists, and the
-- constraints are dropped before they are added.
-- ============================================================================

alter table public.matches add column if not exists mode text;
alter table public.matches add column if not exists stake smallint;

alter table public.matches drop constraint if exists matches_mode_check;
alter table public.matches add constraint matches_mode_check
  check (mode is null or mode in ('bo1', 'bo3', 'random'));

alter table public.matches drop constraint if exists matches_stake_check;
alter table public.matches add constraint matches_stake_check
  check (stake is null or stake in (1, 2));

comment on column public.matches.mode is
  'R672: the mode a rematch was made in. Only a rematch writes it (no ticket, room or series made it); older rows keep deriving theirs from what did.';
comment on column public.matches.stake is
  'R672: a double-or-nothing rematch''s stakes (2); null is a normal game. Only ever 2, and only on a ranked rematch.';
