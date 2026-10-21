// Downloading a finished co-op replay as JSON (issue #1242, R1242000): the caller's seat's
// already-authorized `DuosView`s, each step's index and round, the caller's seat and the build's
// catalog version, in one file a player attaches to a bug report. Nothing else leaves (CLAUDE.md
// rule 7): no seed, log, state, deck, match id, account id or name, and no ping. The file does
// carry what the team shares (§13.6): the teammate's duel and hand as the seat saw them, so
// sharing the file shares that with whoever reads it, though nothing in it names the teammate.
// Its format is versioned apart from R1141000's 1v1 file, and like that file it is made in the
// page and handed to the browser's downloads, never uploaded or kept.

import { CATALOG_VERSION, currentVersionName } from "@jackioh/cards";

import { REPLAY_JSON_MAX_STEPS } from "../replay/config.ts";
import { NOT_A_REPLAY, OTHER_PATCH, TOO_BIG, UNSUPPORTED, isRecord } from "../replay/download.ts";
import { ReplayFailure } from "../replay/source.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import type { DuosReplaySteps } from "./DuosReplay.tsx";
import { DUOS_SEATS } from "./seats.ts";

/** What a reader of the file checks first: what it is, and which shape of it. */
export const DUOS_REPLAY_JSON_FORMAT = "jackioh-duos-replay";
export const DUOS_REPLAY_JSON_VERSION = 2;
/** R1371000: the version a file written before the version retcon carries; its catalog is an old name. */
export const DUOS_REPLAY_JSON_UNMARKED_VERSION = 1;

/** One step of the file: its index, its round and the seat's view after it. */
export type DuosReplayJsonStep = { step: number; round: number; view: DuosView };

/** The file's shape: the format and version, the catalog version, the caller's seat and its views. */
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

/** The file for `steps`: the seat's views, each with its index and round, and nothing of the match. */
export function duosReplayJson(steps: DuosReplaySteps): DuosReplayJson {
  return {
    format: DUOS_REPLAY_JSON_FORMAT,
    version: DUOS_REPLAY_JSON_VERSION,
    catalogVersion: CATALOG_VERSION,
    seat: steps[0].seat,
    steps: steps.map((view, step) => ({ step, round: view.round, view: viewOf(view) })),
  };
}

/** The file for `steps`, made in the page and handed to the browser's downloads. */
export function duosReplayJsonFile(steps: DuosReplaySteps): File {
  return new File([JSON.stringify(duosReplayJson(steps))], "jackioh-duos-replay.json", { type: "application/json" });
}

/**
 * The views of a file `duosReplayJsonFile` wrote, or a `ReplayFailure` saying why it is refused:
 * another format or version, another catalog version, no Duos seat, or steps missing, out of
 * order, out of round or of another seat, under the 1v1 file's caps and sentences. A view is
 * checked only for its seat and round, never for rules (CLAUDE.md rule 7).
 */
export function parseDuosReplayJson(json: Record<string, unknown>): DuosReplaySteps {
  const unmarked = json.version === DUOS_REPLAY_JSON_UNMARKED_VERSION;
  if (json.format !== DUOS_REPLAY_JSON_FORMAT || (json.version !== DUOS_REPLAY_JSON_VERSION && !unmarked)) throw new ReplayFailure("error", UNSUPPORTED);
  const played = unmarked && typeof json.catalogVersion === "string" ? currentVersionName(json.catalogVersion) : json.catalogVersion;
  if (played !== CATALOG_VERSION) throw new ReplayFailure("error", OTHER_PATCH);
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
