// R3395000: Stinkyscape #80 Galaxy (`stinkyscape-080`, entrance "flagshipDescent", bespoke "bellyBay", drill
// sparks falling on its tile), a Legendary Unit's signature. A vast shadow sweeps across the board as the
// flagship descends into its zone, settling on its exhaust. Its Cry Equips Radiant Miners into it: the first
// Miner bolts up into its belly with a drill spin as its `fused` lands, in a ring of sparks, and each later
// one rings and sheens there. It keys on its play and the fused card's public id, never on a Miner's
// definition; the Discover's offer stays its caster's, a prompt no beat shows (R202). Moved: its two
// tributes' deaths are paid before its play opens, so no frame keys them to its card; the chart bars on its
// bridge are dropped and the drill sparks on its tile fall as motes.

import { ringCue, tid, tunedBurst } from "../../build.ts";
import { motifShot } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Its hull's rocket steel. */
const HULL = "Tech/rocket";

/** Where a Miner bolts up from: below its belly. */
const BELOW = { x: 0.5, y: 1.6 } as const;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  exhaust: { count: 24, power: 1 },
} as const;

/** As a share of the entry, how long a Miner flies up into the belly. */
const TIMING = { flight: 0.3 } as const;

export const flagshipDescent: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("flagship", HULL, plan),
    { ...tunedBurst(plan.intensity, "exhaust", tid(plan.tgt, FOOT), "area", slam, TUNING.exhaust), colors: lookOf(HULL).colors },
    ringCue(plan.D, "smoke", at, slam),
    ...slamShake(plan, "flagshipDescent"),
  ];
};

export const bellyBay: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || event.type !== "fused") return null;
  const host = placeOf(p.view, event.resultInstanceId);
  const belly = host === null ? cardAt(p.view, play) : tid(host);
  const flight = Math.round(TIMING.flight * p.D);
  const cues: FxCue[] = [ringCue(p.D, "spark", belly, flight)];
  if ((play.seen.fused ?? 1) <= 1 && belly.kind === "testid") {
    cues.unshift(motifShot({ look: lookOf(HULL), scale: p.env.intensity, row: "replace" }, "bolt", tid(belly.testid, BELOW), belly, 0, flight));
  } else {
    cues.push({ kind: "sheen", at: belly, delayMs: 0, durationMs: p.D });
  }
  return replace(cues);
};
