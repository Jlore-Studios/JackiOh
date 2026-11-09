// R3395000: Stinkyscape #73 Paragon Mk2 (`stinkyscape-073`, entrance "paragonStreak", a cyan shimmer on its
// tile), a Legendary Unit's signature: it has no text but its keywords, so its signature is its entrance.
// An arrowhead fighter streaks in from off the board's edge with a sonic crack, banks hard over its zone and
// lands, its engines still lit for Charge, as a hexagonal shield bubble snaps on around it for its Divine
// Shield. It keys on its own landing, never on a card's definition (R202). Moved: the hex shield shattering
// at the first damage it takes is the shared Divine Shield loss's (R2791000), since no frame keys a later hit
// to its card; the hex shimmer and engine flicker on its tile are cyan motes.

import { testid } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import { motifShot } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type Entrance } from "../recipe.ts";
import { entranceFigure, landingSide, leadOf, lookOf, slamAt, slamShake } from "./signatureKit.ts";

/** Its cyan hull and shield. */
const HULL = "Tech/hologram";

/** Where it streaks in from: off the board's left edge, at its seat's height. */
const FROM = { you: { x: -0.05, y: 0.75 }, opponent: { x: -0.05, y: 0.25 } } as const;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  engines: { count: 18, power: 1 },
} as const;

export const paragonStreak: Entrance = (plan) => {
  const look = lookOf(HULL);
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  const cues: FxCue[] = [
    motifShot({ look, scale: plan.intensity, row: "replace" }, "bolt", tid(testid.board, FROM[landingSide(plan) ?? "you"]), at, leadOf(plan), slam - leadOf(plan)),
    entranceFigure("fighter", HULL, plan, true),
    { kind: "shield", size: "small", at, delayMs: slam, durationMs: plan.D - slam },
    { ...tunedBurst(plan.intensity, "exhaust", tid(plan.tgt, FOOT), "area", slam, TUNING.engines), colors: look.colors },
    ringCue(plan.D, "frost", at, slam),
    ...slamShake(plan, "paragonStreak"),
  ];
  return cues;
};
