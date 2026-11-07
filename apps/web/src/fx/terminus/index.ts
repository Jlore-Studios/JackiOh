// R3338000: the Terminus cards' set pieces, #3338's end-of-the-world tier. Each Terminus card's entry in
// its set's effects table names its own (`{ setPiece: "<key>" }`); the runner plays its signature moment
// as one entry (game/runs.ts `signatureRunAt`, or its first cast), the queue claims it as the action's one
// set piece and holds it (`signatureClaim`, R2785000), and its planner here draws it inside the entry. Big
// moments "short" holds it shorter and "off" not at all (it plays unheld, its beats inside the entry), and
// Reduce motion shows one still frame of it (`planTerminusStill`). Ceaseless Void's fold and Farewell's
// lamp are board clears, drawn with the other wipes (setPieces.ts). Two follow-ups play unheld when the
// turn a card owes opens (Event Horizon's snap back, Last Word's Charge and countdown), and The Big
// Crunch's point stands over each entry of its play (`planTerminusFollow`). Every cue starts inside its
// entry and ends within FX_MAX_TAIL_MS of it (R200); no cue names a card, and a hidden card is drawn as
// a card back, never as which one it is (R202).

import type { PlayerView } from "@jackioh/shared";

import { targetFor, type AnimationEntry } from "../../game/animations.ts";
import { sideOf, testid } from "../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../build.ts";
import { FX_TERMINUS_COUNT_AT, FX_TERMINUS_SNAP_TRAUMA, FX_TERMINUS_STILL_MS, FX_TERMINUS_TAIL_MS } from "../constants.ts";
import { FLAVOR_LOOKS } from "../flavors.ts";
import { calloutCue } from "../motifs/callout.ts";
import { SET_PIECES, setPieceKeyOf, tintOf, type SetPieceKey, type SetPiecePlan } from "../setPieces.ts";
import type { FxAnchor, FxCue, FxPlanEnv } from "../types.ts";
import { BOARD, TERMINUS_TUNING, momentOf, pieceCue, type TerminusPlanner } from "./build.ts";
import { crunch } from "./crunch.ts";
import { destination } from "./destination.ts";
import { doom } from "./doom.ts";
import { exam } from "./exam.ts";
import { horizon } from "./horizon.ts";
import { BIG_CRUNCH, EVENT_HORIZON, LAST_WORD } from "./ids.ts";
import { lastWord } from "./lastWord.ts";
import { rift } from "./rift.ts";

export { TERMINUS_TUNING } from "./build.ts";

/** The keys a signature plays as (the board clears, voidFold and farewell, are wipes: setPieces.ts). */
export type TerminusKey = Exclude<SetPieceKey, "nether" | "voidFold" | "vortex" | "farewell">;

/** R3338000: one planner per Terminus signature; a missing one fails `typecheck`. */
export const TERMINUS_PLANNERS: { readonly [K in TerminusKey]: TerminusPlanner } = {
  terminusRift: rift,
  lastExam: exam,
  eventHorizon: horizon,
  lastWord,
  finalDestination: destination,
  bigCrunch: crunch,
  diceOfDoom: doom,
};

function isTerminusKey(key: SetPieceKey | undefined): key is TerminusKey {
  return key !== undefined && Object.prototype.hasOwnProperty.call(TERMINUS_PLANNERS, key);
}

/**
 * R3338000: the set piece of a Terminus card's signature entry, or null: no signature, a card with none,
 * the effects off or an entry of no duration (Reduce motion: `planTerminusStill`). Held (`entry.setPiece`),
 * its beats run over the hold and its landing follows it; unheld, they run up to SET_PIECE_SWALLOW_AT.
 */
export function planTerminus(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): SetPiecePlan | null {
  const key = entry.signature === undefined ? undefined : setPieceKeyOf(entry.signature.by);
  if (!isTerminusKey(key)) return null;
  const moment = momentOf(key, entry, view, env);
  return moment === null ? null : TERMINUS_PLANNERS[key](moment);
}

/** The key of a Terminus set piece an entry is: its signature's, or a Terminus board clear's (the Void's fold, Farewell's lamp). */
function terminusKeyOf(entry: AnimationEntry): SetPieceKey | undefined {
  if (entry.signature !== undefined) {
    const key = setPieceKeyOf(entry.signature.by);
    return isTerminusKey(key) ? key : undefined;
  }
  const key = entry.wipe === undefined ? undefined : setPieceKeyOf(entry.wipe.by);
  return key === "voidFold" || key === "farewell" ? key : undefined;
}

/**
 * R3338000: under Reduce motion (an entry of no duration), a Terminus set piece's one still frame: its
 * shape at rest over the board for FX_TERMINUS_STILL_MS, holding nothing; nothing with the effects off.
 */
export function planTerminusStill(entry: AnimationEntry, _view: PlayerView, env: Pick<FxPlanEnv, "intensity">): FxCue[] {
  const key = terminusKeyOf(entry);
  if (key === undefined || !(env.intensity > 0)) return [];
  const spec = SET_PIECES[key];
  const at = entry.wipe?.anchor === undefined || entry.wipe.anchor === null ? BOARD : tid(entry.wipe.anchor);
  return [{ ...pieceCue(spec.shape, FLAVOR_LOOKS[spec.palette ?? "Dark"], at, 0, [], FX_TERMINUS_STILL_MS), still: true }];
}

/**
 * R3338000: what a Terminus card plays after its own set piece, unheld, inside the entry it falls on:
 * the opening of the turn Event Horizon owes (the board snaps back: a ring, a burst and a shake) and of
 * the one Last Word owes (each of the player's Units lit for Charge, and a 3, 2, 1 on their hero that
 * burns away), as the memory says the entry opened it (`opening`); and, while The Big Crunch's play
 * resolves, its bright point over every entry but its claimed first cast, each of its casts flying out of it.
 */
export function planTerminusFollow(entry: AnimationEntry, view: PlayerView, env: FxPlanEnv): FxCue[] {
  const D = entry.durationMs;
  const i = env.intensity;
  if (!(i > 0) || !(D > 0)) return [];
  const look = FLAVOR_LOOKS.Dark;
  const cues: FxCue[] = [];
  const opening = env.memory.opening();
  if (opening?.defId === EVENT_HORIZON) {
    cues.push(
      ringCue(D, "prismatic", BOARD, 0),
      { ...tunedBurst(i, "prismatic", BOARD, "area", 0, TERMINUS_TUNING.snap), colors: look.colors },
      ...shakeCues(i, FX_TERMINUS_SNAP_TRAUMA, 0),
    );
  }
  if (opening?.defId === LAST_WORD) {
    const side = sideOf(view, opening.player);
    for (const unit of view[side].units) {
      if (unit !== null) cues.push(calloutCue("Charge", tid(testid.card(unit.instanceId)), 0, false));
    }
    const hero = tid(testid.hero(side));
    FX_TERMINUS_COUNT_AT.forEach((at, k) => {
      const from = frac(at, D);
      const until = k + 1 < FX_TERMINUS_COUNT_AT.length ? frac(FX_TERMINUS_COUNT_AT[k + 1] ?? 1, D) : D;
      cues.push({ kind: "zone", at: hero, tint: tintOf(look), text: String(FX_TERMINUS_COUNT_AT.length - k), direction: "none", delayMs: from, durationMs: Math.max(1, until - from) });
    });
    cues.push({ ...tunedBurst(i, "ember", hero, "area", D, TERMINUS_TUNING.countEmber), colors: look.colors });
  }
  const crunching = env.memory.openPlay(BIG_CRUNCH);
  // From its first cast on (its own play's entry comes before the collapse).
  if (crunching !== undefined && (crunching.seen.cardPlayed ?? 0) > 0) {
    const own = entry.signature?.by === BIG_CRUNCH;
    const H = own && entry.setPiece?.key === "bigCrunch" ? entry.setPiece.holdMs : 0;
    // Its first cast draws the collapse itself (planTerminus); every later entry keeps the point.
    if (!own) cues.push(pieceCue("crunch", look, BOARD, 0, [], D + FX_TERMINUS_TAIL_MS));
    if (entry.cast?.by === BIG_CRUNCH) {
      const played = entry.events.find((event) => event.type === "cardPlayed");
      const at = played === undefined ? null : targetFor(played, view);
      const to: FxAnchor = at === null ? tid(testid.hero(sideOf(view, crunching.player))) : tid(at);
      cues.push({ kind: "ghost", from: BOARD, to, delayMs: H, durationMs: Math.max(1, frac(TERMINUS_TUNING.rainFall, D - H)) });
    }
  }
  return cues;
}
