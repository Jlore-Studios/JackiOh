// Downloading a finished co-op replay as JSON (issue #1242, R1242000): the caller's seat's
// already-authorized `DuosView`s and the build's catalog version. Nothing else leaves: no seed, log,
// state, deck, match id, account id or name, and no ping. The file does carry what the team shares
// (§13.6), the teammate's duel and hand as the seat saw them, though nothing in it names the teammate.
// Its format is versioned apart from R1141000's 1v1 file; it is made in the page, never uploaded or kept.

import { CATALOG_VERSION } from "@jackioh/cards";

import { REPLAY_JSON_MAX_STEPS } from "../replay/config.ts";
import { NOT_A_REPLAY, TOO_BIG, checkReplayHeader } from "../replay/download.ts";
import { isRecord } from "../wire/json.ts";
import { ReplayFailure } from "../replay/source.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import type { DuosReplaySteps } from "./DuosReplay.tsx";
import { DUOS_SEATS } from "./seats.ts";

export const DUOS_REPLAY_JSON_FORMAT = "jackioh-duos-replay";
/** R2532000: 3 since the v0.5 retcon (#2532), its versions named by `VERSIONS_SCHEME`. */
export const DUOS_REPLAY_JSON_VERSION = 3;
/** R1371000: the version a file written before the version retcon carries; its catalog is an old name. */
export const DUOS_REPLAY_JSON_UNMARKED_VERSION = 1;

export type DuosReplayJsonStep = { step: number; round: number; view: DuosView };

export type DuosReplayJson = {
  format: typeof DUOS_REPLAY_JSON_FORMAT;
  version: typeof DUOS_REPLAY_JSON_VERSION;
  catalogVersion: string;
  seat: DuosSeat;
  steps: DuosReplayJsonStep[];
};

/** A view's fields named one by one, so a field the server's answer gains is never carried by accident. */
function viewOf(view: DuosView): DuosView {
  const { seat, own, teammate, team, enemy, round, roundSeats, concedeOfferedBy, result } = view;
  return { seat, own, teammate, team, enemy, round, roundSeats, concedeOfferedBy, result };
}

export function duosReplayJson(steps: DuosReplaySteps): DuosReplayJson {
  return {
    format: DUOS_REPLAY_JSON_FORMAT,
    version: DUOS_REPLAY_JSON_VERSION,
    catalogVersion: CATALOG_VERSION,
    seat: steps[0].seat,
    steps: steps.map((view, step) => ({ step, round: view.round, view: viewOf(view) })),
  };
}

export function duosReplayJsonFile(steps: DuosReplaySteps): File {
  return new File([JSON.stringify(duosReplayJson(steps))], "jackioh-duos-replay.json", { type: "application/json" });
}

/**
 * The views of a file `duosReplayJsonFile` wrote, under the 1v1 file's caps and sentences. A view is
 * checked only for its seat and round, never for rules (CLAUDE.md rule 7).
 */
export function parseDuosReplayJson(json: Record<string, unknown>): DuosReplaySteps {
  checkReplayHeader(json, DUOS_REPLAY_JSON_FORMAT, DUOS_REPLAY_JSON_UNMARKED_VERSION, DUOS_REPLAY_JSON_VERSION);
  const { seat, steps } = json;
  if (!DUOS_SEATS.some((each) => each === seat)) throw new ReplayFailure("error", NOT_A_REPLAY);
  if (!Array.isArray(steps) || steps.length === 0) throw new ReplayFailure("error", NOT_A_REPLAY);
  if (steps.length > REPLAY_JSON_MAX_STEPS) throw new ReplayFailure("error", TOO_BIG);
  const views: DuosView[] = [];
  let round = 0;
  for (const [index, step] of (steps as unknown[]).entries()) {
    if (!isRecord(step) || step.step !== index || !isRecord(step.view) || step.view.seat !== seat) throw new ReplayFailure("error", NOT_A_REPLAY);
    const view = step.view;
    if (typeof step.round !== "number" || !Number.isInteger(step.round) || step.round < round || view.round !== step.round) {
      throw new ReplayFailure("error", NOT_A_REPLAY);
    }
    round = step.round;
    views.push(view as DuosView);
  }
  const [first, ...rest] = views;
  if (first === undefined) throw new ReplayFailure("error", NOT_A_REPLAY);
  return [first, ...rest];
}
