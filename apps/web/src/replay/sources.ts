// The replay player's two sources (#511): a practice game kept on this device, read through the
// practice worker's `replay` request (#509), and an online match, read through `GET /api/replays/:id`
// (#510). Each hands the player steps and nothing else (CLAUDE.md rule 7: no log, seed or state crosses).

import type { PracticeHost } from "../practice/host.ts";
import type { PracticeReplaySummary } from "../practice/protocol.ts";
import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";

import { createLoadoutsHolder, type LoadoutsHolder } from "../cards/loadouts.ts";
import { ApiRequestError, getReplaySteps } from "../net/api.ts";
import type { OnlineReplayListing } from "../net/api.ts";
import { REPLAY_OPEN_ATTEMPTS, REPLAY_OPEN_RETRY_MS } from "./config.ts";
import { ReplayFailure, replayFailureOf } from "./source.ts";
import type { ReplaySource } from "./source.ts";

/**
 * A practice worker's refusal as a failure. The worker's messages (`replayAnswer` in `practice/core.ts`):
 * "… cannot be replayed: <part 1's reason>" for a game that is unavailable and "no finished practice
 * game N is kept on this device" for one it dropped (expired). Any other message is an `error`.
 */
export function practiceFailure(message: string): ReplayFailure {
  const refused = /cannot be replayed: (earlier_patch|rules_changed)$/u.exec(message);
  if (refused !== null) return new ReplayFailure(refused[1] === "earlier_patch" ? "earlier_patch" : "rules_changed", message);
  if (/^no finished practice game \d+ is kept on this device$/u.test(message)) return new ReplayFailure("expired", message);
  return new ReplayFailure("error", message);
}

/**
 * A practice game kept on this device: `listing` is the row `replays` answered (or the finished game's
 * snapshot's `replay`), `host` the worker, or anything that sends it requests (the controller's queue).
 */
export function practiceReplaySource(
  host: Pick<PracticeHost, "request">,
  listing: Pick<PracticeReplaySummary, "game" | "steps">,
): ReplaySource {
  return {
    total: listing.steps,
    async page(from, count) {
      const response = await host.request({ type: "replay", game: listing.game, from, count });
      if (response.type === "replay") return response.steps;
      if (response.type === "failed") throw practiceFailure(response.message);
      throw new ReplayFailure("error", `the practice worker answered ${response.type} to a replay request`);
    },
  };
}

/**
 * A server's refusal as a failure: 410 `gone` (the log is purged) is expired, 422 `replay_unavailable`
 * carries part 1's reason in `details.reason`, and anything else is an `error` with its own message.
 */
export function onlineFailure(cause: unknown): ReplayFailure {
  if (cause instanceof ApiRequestError) {
    if (cause.code === "gone") return new ReplayFailure("expired", cause.message);
    if (cause.code === "replay_unavailable") {
      const reason = (cause.details as { reason?: unknown } | null | undefined)?.reason;
      if (reason === "earlier_patch" || reason === "rules_changed") return new ReplayFailure(reason, cause.message);
    }
  }
  return replayFailureOf(cause);
}

/** An online match the account played, by its listing: its pages are the server's, fetched with the account's token. */
export function onlineReplaySource(token: string, listing: Pick<OnlineReplayListing, "matchId" | "steps">): ReplaySource {
  return onlinePages(token, listing, createLoadoutsHolder());
}

/** `onlineReplaySource` merging each page's loadouts into `held` (R1834003). */
function onlinePages(token: string, listing: Pick<OnlineReplayListing, "matchId" | "steps">, held: LoadoutsHolder): ReplaySource {
  return {
    total: listing.steps,
    async page(from, count) {
      let answer: Awaited<ReturnType<typeof getReplaySteps>>;
      try {
        answer = await getReplaySteps(token, listing.matchId, from, count);
      } catch (cause) {
        throw onlineFailure(cause);
      }
      held.page(answer);
      return answer.steps;
    },
    loadouts: held.current,
  };
}

/**
 * R768000: the replay of `matchId`, just played, whose listing the account does not hold: its first page
 * is fetched to learn its length and kept as step 0's page. A 409 (its result is still being written) is
 * asked again, `REPLAY_OPEN_ATTEMPTS` times in all; otherwise it rejects with the first page's failure.
 */
export async function openOnlineMatchReplay(
  token: string,
  matchId: string,
  wait: (ms: number) => Promise<void> = (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
): Promise<ReplaySource> {
  let first: Awaited<ReturnType<typeof getReplaySteps>> | null = null;
  for (let attempt = 1; first === null; attempt += 1) {
    try {
      first = await getReplaySteps(token, matchId, 0, REPLAY_PAGE_STEPS);
    } catch (cause) {
      const finishing = cause instanceof ApiRequestError && cause.status === 409;
      if (!finishing || attempt >= REPLAY_OPEN_ATTEMPTS) throw onlineFailure(cause);
      await wait(REPLAY_OPEN_RETRY_MS);
    }
  }
  const held = createLoadoutsHolder();
  held.page(first);
  const later = onlinePages(token, { matchId, steps: first.total }, held);
  return {
    total: first.total,
    page: (from, count) => (from === 0 ? Promise.resolve(first.steps.slice(0, count)) : later.page(from, count)),
    loadouts: held.current,
  };
}
