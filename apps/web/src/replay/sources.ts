// The replay player's two sources (#511): a practice game kept on this device, read through the
// practice worker's `replay` request (#509), and an online match, read through `GET /api/replays/:id`
// (#510). Each hands the player steps and nothing else, and turns what went wrong into a
// `ReplayFailure` the player words (CLAUDE.md rule 7: no log, seed or state crosses).

import type { PracticeHost } from "../practice/host.ts";
import type { PracticeReplaySummary } from "../practice/protocol.ts";
import { ApiRequestError, getReplaySteps } from "../net/api.ts";
import type { OnlineReplayListing } from "../net/api.ts";
import { ReplayFailure, replayFailureOf } from "./source.ts";
import type { ReplaySource } from "./source.ts";

/**
 * A practice worker's refusal as a failure. The worker answers a refusal as a message
 * (`replayAnswer` in `practice/core.ts`): "… cannot be replayed: <part 1's reason>" for a game that
 * is unavailable and "no finished practice game N is kept on this device" for one it dropped (a
 * game no longer kept reads as expired). Any other message, a game in progress among them, is an
 * `error` that says what the worker said.
 */
export function practiceFailure(message: string): ReplayFailure {
  const refused = /cannot be replayed: (earlier_patch|rules_changed)$/u.exec(message);
  if (refused !== null) return new ReplayFailure(refused[1] === "earlier_patch" ? "earlier_patch" : "rules_changed", message);
  if (/^no finished practice game \d+ is kept on this device$/u.test(message)) return new ReplayFailure("expired", message);
  return new ReplayFailure("error", message);
}

/** A practice game kept on this device: `listing` is the row `replays` answered, `host` the worker that holds the game. */
export function practiceReplaySource(host: PracticeHost, listing: Pick<PracticeReplaySummary, "game" | "steps">): ReplaySource {
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
 * carries part 1's reason in `details.reason`, and anything else (the network, the rate limit, a
 * match not over) is an `error` with its own message.
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
  return {
    total: listing.steps,
    async page(from, count) {
      try {
        return (await getReplaySteps(token, listing.matchId, from, count)).steps;
      } catch (cause) {
        throw onlineFailure(cause);
      }
    },
  };
}
