// R3395000: Stinkyscape #37 Kwzka (`stinkyscape-037`, entrance "spotlightKiss", bespoke "allureRibbon",
// pink petals falling on her tile), a Legendary Unit's signature. A pink spotlight drops onto her zone and
// she rises on a stage platform in a burst of confetti, blowing a kiss that flies as hearts to the enemy
// zone across her lane. When her Aura Allures a cheaper enemy Unit, a pink ribbon runs from her to it as
// its Allure mark blooms (a beam and a ring on it), and it lands in a ring where its control changes. At
// her Death every ribbon snaps: a burst of sparks on each Unit losing its Allure. It keys on her hooks and
// the Units' public ids and marks, never on a card's definition (R202). Moved: the spotlight cone on her
// tile is a fall of pink petals (an aura draws motes and an edge).

import { ALLURE_MARK } from "../../../cards/marks.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxCue, FxShotCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, leadOf, lookOf, placeOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Her pink: Clout's hearts and confetti. */
const PINK = "Clout";

/** Every particle count her signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  stage: { count: 22, power: 1 },
  snap: { count: 12, power: 0.9 },
} as const;

/** As shares of the entry: the kiss flies from her slam for `kiss`; a ribbon unrolls for `ribbon`. */
const TIMING = { kiss: 0.3, ribbon: 0.3 } as const;

export const spotlightKiss: Entrance = (plan) => {
  const look = lookOf(PINK);
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  const cues: FxCue[] = [
    entranceFigure("spotlight", PINK, plan, true),
    { ...tunedBurst(plan.intensity, "confetti", tid(plan.tgt, FOOT), "area", slam, TUNING.stage), colors: look.colors },
    ringCue(plan.D, "confetti", at, slam),
  ];
  if (plan.view !== undefined && plan.landing !== undefined) {
    // The kiss: hearts across her lane to the enemy's zone.
    const enemy = testid.zone(otherSide(sideOf(plan.view, plan.landing.player)), "units", plan.landing.lane);
    const kiss: FxShotCue = {
      kind: "shot",
      shot: { motion: "arc", sprites: ["heart"], turn: "float", count: 3 },
      colors: look.colors,
      preset: "confetti",
      from: at,
      to: tid(enemy),
      delayMs: slam,
      flightMs: Math.round(TIMING.kiss * (plan.D - leadOf(plan))),
      density: plan.intensity,
    };
    cues.push(kiss);
  }
  cues.push(...slamShake(plan, "spotlightKiss"));
  return cues;
};

export const allureRibbon: CardRecipe = (event, p, play) => {
  const look = lookOf(PINK);
  if (event.type === "marked" && event.mark === ALLURE_MARK) {
    const unit = placeOf(p.view, event.instanceId);
    if (unit === null) return replace([]);
    if (!event.added) {
      // Her Death: the ribbon snaps.
      return replace([{ ...tunedBurst(p.env.intensity, "spark", tid(unit), "point", 0, TUNING.snap), colors: look.colors }]);
    }
    const flight = Math.round(TIMING.ribbon * p.D);
    const ribbon: FxShotCue = { kind: "shot", shot: { motion: "beam" }, colors: look.colors, preset: "confetti", from: cardAt(p.view, play), to: tid(unit), delayMs: 0, flightMs: flight, density: p.env.intensity };
    return replace([ribbon, ringCue(p.D, "confetti", tid(unit), flight)]);
  }
  if (event.type === "controlChanged" && event.controller === play.player) {
    return replace([ringCue(p.D, "confetti", tid(testid.zone(sideOf(p.view, event.controller), event.row, event.lane)), 0)]);
  }
  return null;
};
