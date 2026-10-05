/**
 * The live half of the card statistics (SPEC §9.11, R376): the record a finished match leaves.
 *
 * `createRecordResult` (`results.ts`) calls `recordLiveGame` once a match's result has committed.
 * The record's game half — each seat's decklist, opening hand, draws and plays, who went first and
 * who won — is read off `(seed, decks, log)` by the engine's `summarizeGame`, through the engine
 * port the composition root binds into `deps.games`. It is filed under:
 *
 *  - the match's mode (R257), read off what made it: its series, its room or its queue tickets
 *    (`matches.modeOf`), so the match row and the path that starts a match are unchanged;
 *  - the patch the build's cards are (R388's newest patch, `deps.games.patch`);
 *  - two human pilots: a live match is two players, and a prompt the clock answered for one of them
 *    (R79's `timeout`) is still their game;
 *  - `source: "live"`, which is what keeps it apart from the AI's development runs (R378).
 *
 * It is telemetry, so it never costs a result: it runs after the result's transaction, and every
 * failure is logged and swallowed. It is idempotent: the record is keyed by the match id and the
 * store refuses a second one, so a result written twice (an actor healing itself after a restart)
 * files the game once. The reaper's ceiling draws (R112) leave none: the reaper reaches a match
 * whose actor stopped answering, and writes its result without a game to read.
 */

import type { GameRecord, Pilot, PlayerId } from "@jackioh/shared";
import type { ServerDeps } from "./ports";

/** R376: both seats of a live match are players. */
const LIVE_PILOTS: Readonly<Record<PlayerId, Pilot>> = { p1: "human", p2: "human" };

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Files one finished match for the card statistics. Resolves with the record it wrote, or null when
 * it wrote none: no recorder is bound, no series, room or ticket made the match, the log does not
 * reach a result, the record exists already, or something failed — each but the first and the
 * fourth logged. Until migration 0014 is applied the insert fails, which is logged and costs nothing
 * else: no other path reads or writes the table.
 */
export async function recordLiveGame(deps: ServerDeps, matchId: string): Promise<GameRecord | null> {
  const games = deps.games;
  if (games === undefined) return null;
  try {
    const match = await deps.store.matches.get(matchId);
    const mode = match === null ? null : await deps.store.matches.modeOf(matchId);
    if (match === null || mode === null) {
      deps.log.warn("game.record.skipped", {
        matchId,
        reason: match === null ? "no match row" : "no series, room or queue ticket made the match",
      });
      return null;
    }

    const log = await deps.store.matches.actions(matchId);
    const game = games.summarize({
      seed: match.seed,
      decks: match.decks,
      log: log.map((row) => row.action),
      // R417, R678: a match's frozen boards are setup, so the fold reads them as the live game did.
      ...(match.lastBoards === undefined ? {} : { lastBoards: match.lastBoards }),
      ...(match.glitchBoards === undefined ? {} : { glitchBoards: match.glitchBoards }),
    });
    if (game === null) {
      // The result came from this log's own last action, so a fold that does not reach it is a
      // determinism break (§9.3), and says so as loudly as the registry's fold errors do.
      deps.log.alert("game.record.unfinished", { matchId, actions: log.length });
      return null;
    }

    const record: GameRecord = {
      id: matchId,
      source: "live",
      mode,
      patch: games.patch,
      pilots: { ...LIVE_PILOTS },
      game,
    };
    if (!(await deps.store.gameRecords.insert(record))) return null;
    deps.log.info("game.recorded", { matchId, mode: record.mode, patch: record.patch });
    return record;
  } catch (error: unknown) {
    deps.log.alert("game.record.failed", { matchId, message: messageOf(error) });
    return null;
  }
}
