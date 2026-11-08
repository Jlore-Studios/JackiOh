// R3395000: Stinkyscape #66 Miner v5 (`stinkyscape-066`, entrance "intakeMaw", bespoke "asteroidCrunch",
// drill dust on its tile), a Legendary Unit's signature. A huge intake maw grinds open in its zone, drills
// spinning up, and a gust of grit blows across the tile. At each Activate an asteroid is crunched in its maw
// (a burst of ore) and the Resource it gains pops out the back, flying to its owner's hand pile as a card
// back, a glint on that pile for its owner alone, whose card it is to read. It keys on its hook and its own
// public id, never on the Resource's definition (R202). Moved: the asteroid drawn into the maw is the crunch
// itself; a Miner equipped into another Unit mines as that Unit; the ore glinting on its tile is drill dust
// trickling down.

import { ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, cardBack, entranceFigure, handOf, lookOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Its ore. */
const ORE = "Hustle/ore";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  grit: { count: 24, power: 1.1 },
  crunch: { count: 16, power: 1 },
} as const;

/** As a share of the entry, when the Resource pops out of the maw. */
const TIMING = { popAt: 0.3 } as const;

export const intakeMaw: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("maw", ORE, plan),
    { ...tunedBurst(plan.intensity, "dust", tid(plan.tgt, FOOT), "area", slam, TUNING.grit), colors: lookOf(ORE).colors },
    ringCue(plan.D, "crumb", at, slam),
    ...slamShake(plan, "intakeMaw"),
  ];
};

export const asteroidCrunch: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate" || event.type !== "addedToHand" || event.player !== play.player) return null;
  const maw = cardAt(p.view, play);
  const hand = handOf(p.view, event.player);
  const cues: FxCue[] = [{ ...tunedBurst(p.env.intensity, "crumb", maw, "area", 0, TUNING.crunch), colors: lookOf(ORE).colors }, cardBack(maw, hand, Math.round(TIMING.popAt * p.D), p.D)];
  // Its kind is its owner's to read: the glint plays on their seat alone.
  if (p.view.viewer === event.player) cues.push({ kind: "sheen", at: hand, delayMs: 0, durationMs: p.D });
  return replace(cues);
};
