/**
 * The retention purge: rows nothing reads any more are deleted once they are old, so the privacy
 * policy can state how long each is kept. Not in SPEC, and no R-row.
 *
 *  - `code_attempts` (profile id, peppered IP hash, time) after `CODE_ATTEMPT_RETENTION_DAYS`. The
 *    limits that read them look back `CODE_ATTEMPT_WINDOW_SECONDS` at most.
 *  - a finished match's action log `MATCH_ACTION_RETENTION_DAYS` after the match ended. Only a live
 *    match is replayed from its log; the result row, and so the rating history, is kept.
 *
 * `src/index.ts` runs it at boot and then every `RETENTION_PURGE_INTERVAL_SECONDS`, next to the
 * match reaper. In Postgres it is one call to `app.purge_expired_rows` (migration 0013).
 */

import { CODE_ATTEMPT_RETENTION_DAYS, MATCH_ACTION_RETENTION_DAYS } from "../config";
import type { RetentionPurgeResult, ServerDeps } from "./ports";

/** Unit conversion, not configuration. */
const MS_PER_DAY = 86_400_000;

export async function purgeExpired(deps: Pick<ServerDeps, "store" | "timers">): Promise<RetentionPurgeResult> {
  const now = deps.timers.now();
  return deps.store.purgeExpired({
    codeAttemptsBefore: now - CODE_ATTEMPT_RETENTION_DAYS * MS_PER_DAY,
    matchActionsEndedBefore: now - MATCH_ACTION_RETENTION_DAYS * MS_PER_DAY,
  });
}
