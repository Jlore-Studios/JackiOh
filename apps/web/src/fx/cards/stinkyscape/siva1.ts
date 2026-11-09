// R3395000: Stinkyscape #87 Siva 1 (`stinkyscape-087`, entrance "redCarpet", bespoke "glowLeash", flash
// sparkles on her tile), a Legendary Unit's signature. A lilac carpet rolls into her zone and she struts
// down it through camera flashes, tail high, then sits. Her Cry flicks a glowing leash at a random enemy
// Acclaimed Unit, the loop closing in a ring as its Allure mark takes; on the Radiant face a Unit she
// summons for the opponent lands Frozen in a frost ring first; where her Allure tugs a Unit across, it lands
// in a ring. It keys on her play and the Units' public ids and marks, never on a card's definition (R202).
// Moved: the steal comes at her controller's next turn, under no hook of hers; the flashes and the leash's
// end on her tile are lilac sparkles.

import { ALLURE_MARK } from "../../../cards/marks.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxShotCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake, summonZone } from "./signatureKit.ts";

/** Her lilac. */
const LILAC = "Beast/felinor";

/** Every particle count she throws, at intensity "normal" (rule 9). */
const TUNING = {
  flashes: { count: 22, power: 1 },
} as const;

/** As a share of the entry, how long the leash flies. */
const TIMING = { leash: 0.35 } as const;

export const redCarpet: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("carpet", LILAC, plan, true),
    { ...tunedBurst(plan.intensity, "sparkle", tid(plan.tgt, FOOT), "area", slam, TUNING.flashes), colors: lookOf(LILAC).colors },
    ringCue(plan.D, "sparkle", at, slam),
    ...slamShake(plan, "redCarpet"),
  ];
};

export const glowLeash: CardRecipe = (event, p, play) => {
  if (event.type === "marked" && event.mark === ALLURE_MARK && event.added && play.hook === undefined) {
    const unit = placeOf(p.view, event.instanceId);
    if (unit === null) return null;
    const flight = Math.round(TIMING.leash * p.D);
    const leash: FxShotCue = { kind: "shot", shot: { motion: "beam" }, colors: lookOf(LILAC).colors, preset: "sparkle", from: cardAt(p.view, play), to: tid(unit), delayMs: 0, flightMs: flight, density: p.env.intensity };
    return replace([leash, ringCue(p.D, "sparkle", tid(unit), flight)]);
  }
  if (event.type === "summoned" && event.player !== play.player && play.hook === undefined) {
    return replace([ringCue(p.D, "frost", tid(summonZone(p.view, event)), 0)]);
  }
  if (event.type === "controlChanged" && event.controller === play.player) {
    return replace([ringCue(p.D, "sparkle", tid(testid.zone(sideOf(p.view, event.controller), event.row, event.lane)), 0)]);
  }
  return null;
};
