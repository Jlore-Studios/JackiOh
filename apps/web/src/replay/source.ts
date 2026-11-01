// What the replay player reads (#511, R768): a count of steps and pages of them. The player never sees a
// log, a seed or a state (CLAUDE.md rule 7); a step is the view the seat held at that point, and its turn.

import type { ReplayRefusal, ReplayStep } from "@jackioh/engine";

import type { SeatLoadouts } from "../cards/loadouts.ts";
import { t } from "../i18n/i18n.ts";

/**
 * One replay, a page of steps at a time. `total` is the listing's `steps`: step 0 and one per accepted
 * action. A page holds 1 to `count` steps from `from`; when it holds fewer (a Glitch swap moved the
 * viewer, or the engine's page ended early) the next page starts at `from + length`. A failed page
 * rejects with a `ReplayFailure`. R1834003: an online match's source also answers both seats' frozen
 * loadouts (R1834002) as the pages fetched so far merged them, null before the first; others have none.
 */
export type ReplaySource = {
  readonly total: number;
  page(from: number, count: number): Promise<readonly ReplayStep[]>;
  loadouts?(): SeatLoadouts | null;
};

/** A source over steps already held in memory (#1160): a file opened from the player's own device. */
export function memoryReplaySource(steps: readonly ReplayStep[]): ReplaySource {
  const held = [...steps];
  return { total: held.length, page: (from, count) => Promise.resolve(held.slice(from, from + count)) };
}

/**
 * Why a replay cannot be shown: part 1's two refusals (R768), `expired` (its log was purged, or the
 * practice game is no longer kept) and `error`, anything else.
 */
export type ReplayFailureReason = ReplayRefusal | "expired" | "error";

/** A failed page, as `ApiRequestError` is a failed request: the reason, and a message for a debugger. */
export class ReplayFailure extends Error {
  readonly reason: ReplayFailureReason;

  constructor(reason: ReplayFailureReason, message: string = reason) {
    super(message);
    this.name = "ReplayFailure";
    this.reason = reason;
  }
}

export const REPLAY_FAILURE_TEXT: Record<Exclude<ReplayFailureReason, "error">, string> = {
  earlier_patch: "This game was played on an earlier patch.",
  rules_changed: "This game was played under older rules.",
  expired: "This replay has expired.",
};

export function replayFailureOf(cause: unknown): ReplayFailure {
  if (cause instanceof ReplayFailure) return cause;
  return new ReplayFailure("error", cause instanceof Error ? cause.message : String(cause));
}

export function replayFailureText(failure: ReplayFailure): string {
  return t(failure.reason === "error" ? failure.message : REPLAY_FAILURE_TEXT[failure.reason]);
}
