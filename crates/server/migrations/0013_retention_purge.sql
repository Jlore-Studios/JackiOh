-- ============================================================================
-- Migration 0013: the retention purge
-- ============================================================================
-- Rows nothing reads any more are deleted once they are old, so the privacy
-- policy can state a retention period instead of "forever":
--
--   * public.code_attempts older than CODE_ATTEMPT_RETENTION_DAYS (30). The
--     limits that read the table look back an hour at most (SPEC §9.4 steps 2
--     and 3, R106's 600 s window).
--   * public.match_actions of every match that ended more than
--     MATCH_ACTION_RETENTION_DAYS (90) ago. Only a live match is ever replayed
--     from its log. public.results, and so every rating, is kept.
--
-- The numbers live in apps/server/src/config.ts. The server passes the two
-- cutoffs in: `purgeExpired` in src/api/retention.ts runs at boot and then
-- every RETENTION_PURGE_INTERVAL_SECONDS, next to the match reaper. No pg_cron.
--
-- match_actions is append-only (0004, app.deny_row_mutation). The purge is the
-- one path that deletes from it: app.purge_expired_rows sets
-- `jackioh.retention_purge = 'on'` for its own deletes, and the guard lets a
-- DELETE through only while that is set. The function clears it again before it
-- returns, so no other statement in the same transaction inherits it.
--
-- It is set with set_config() in the body, never with a `set` clause on the
-- function: since Postgres 15 only a superuser may attach a parameter Postgres
-- does not know to a function, and the role db:migrate runs as on Supabase is not
-- one ("permission denied to set parameter"). That clause was this file's first
-- version, which src/db/migrate.ts still accepts as applied (REWRITTEN).
-- ============================================================================


-- ----------------------------------------------------------------------------
-- 1. The append-only guard, with the purge's one exception added to 0012's.
-- ----------------------------------------------------------------------------
create or replace function app.deny_row_mutation()
returns trigger
language plpgsql
set search_path = ''
as $$
declare
  v_column  text := tg_argv[0];
  v_profile uuid;
begin
  -- 0012: the foreign key's own cascade or set-null for a deleted profile.
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

  -- 0013: a delete made by app.purge_expired_rows. Deletes only; nothing edits a row.
  if tg_op = 'DELETE' and current_setting('jackioh.retention_purge', true) = 'on' then
    return old;
  end if;

  raise exception
    'append-only table %.% may not be updated or deleted (attempted %)',
    tg_table_schema, tg_table_name, tg_op;
end;
$$;


-- ----------------------------------------------------------------------------
-- 2. The purge itself.
-- ----------------------------------------------------------------------------
-- SECURITY INVOKER: the server calls it as service_role, which may delete from
-- both tables already. Returns how many rows of each went, for the log line.
create or replace function app.purge_expired_rows(
  p_code_attempts_before timestamptz,
  p_match_actions_before timestamptz
) returns table (code_attempts bigint, match_actions bigint)
language plpgsql
set search_path = ''
as $$
declare
  v_attempts bigint;
  v_actions  bigint;
begin
  -- Transaction-local, and cleared below; a failed delete aborts the transaction.
  perform set_config('jackioh.retention_purge', 'on', true);

  delete from public.code_attempts a
   where a.at < p_code_attempts_before;
  get diagnostics v_attempts = row_count;

  delete from public.match_actions a
   using public.matches m
   where a.match_id = m.id
     and m.status = 'over'
     and m.ended_at < p_match_actions_before;
  get diagnostics v_actions = row_count;

  perform set_config('jackioh.retention_purge', 'off', true);
  return query select v_attempts, v_actions;
end;
$$;

comment on function app.purge_expired_rows(timestamptz, timestamptz) is
  'Migration 0013: deletes code_attempts made before the first cutoff and the '
  'action log of every match that ended before the second. Results are kept. '
  'Called by the server (src/api/retention.ts) with cutoffs from src/config.ts.';

revoke execute on function app.purge_expired_rows(timestamptz, timestamptz) from public;
grant execute on function app.purge_expired_rows(timestamptz, timestamptz) to postgres, service_role;
