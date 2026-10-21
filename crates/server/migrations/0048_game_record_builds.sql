-- ============================================================================
-- Migration 0048: game records carry their build (R1367000; issue #1367)
-- ============================================================================
-- Serves R1367000 (amends R654): a game record names the build that played it,
-- `record.build`: the git commit the server (`RENDER_GIT_COMMIT`) or tool was
-- built from, and, when the AI played a seat, the AI generation that build
-- holds (`crates/ai/generation.json`). An AI change can then be split out of a
-- patch's figures without splitting the patch (#1360).
--
-- The record is stored whole in public.game_records.record (0014), so nothing
-- new is written: this migration only holds the field's shape. A record
-- written before it has no `build` and stays valid; the server and the
-- engine's reader (`parse_game_record`) read it as before.
--
-- `game_records_build_check`: `build` is absent, or an object whose `commit`
-- is a non-empty string and whose `aiGeneration`, when present, is a
-- non-negative whole number. The fake store refuses the same records
-- (crates/server/src/db/fake.rs). A record whose detail the purge emptied
-- (R1044001, `record` null) passes as one without a build.
--
-- Apply order: 0001 -> ... -> 0014 (`public.game_records`) -> ... -> 0041
-- (the totals) -> ... -> 0046 -> 0047 -> 0048 (this file).
-- Safe to re-apply: drop-constraint-if-exists before the add, and comment-on.
-- ============================================================================

alter table public.game_records drop constraint if exists game_records_build_check;
alter table public.game_records add constraint game_records_build_check check (
  record is null
  or not (record ? 'build')
  -- A missing key reads as null, and a null check passes: coalesce makes it a refusal.
  or coalesce(
    jsonb_typeof(record -> 'build') = 'object'
    and jsonb_typeof(record -> 'build' -> 'commit') = 'string'
    and length(record -> 'build' ->> 'commit') > 0
    and (
      not ((record -> 'build') ? 'aiGeneration')
      -- `case`, so a value that is not a number is never cast.
      or case
        when jsonb_typeof(record -> 'build' -> 'aiGeneration') = 'number' then
          (record -> 'build' ->> 'aiGeneration')::numeric >= 0
          and (record -> 'build' ->> 'aiGeneration')::numeric
              = trunc((record -> 'build' ->> 'aiGeneration')::numeric)
        else false
      end
    ),
    false
  )
) not valid;
alter table public.game_records validate constraint game_records_build_check;

comment on column public.game_records.record is
  $$R376: the GameRecord itself (crates/engine/src/wire/stats.rs). R1367000:
  its `build`, when present, names the commit that played the game and the
  AI generation of a seat the AI played (game_records_build_check).$$;
