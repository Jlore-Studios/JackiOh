// R3395000: Stinkyscape #59 Repeater v5 (`stinkyscape-059`, entrance "barrelSpin", bespoke "repeaterBurst",
// barrel glow on its tile), a Legendary Unit's signature. A rotary cluster of barrels drops into its zone and
// spins up once, spent casings rattling across the tile. Each Activate fires a tight burst of three bolts
// through its target, its barrels glowing hot as they fire. It keys on its hook and its own public id, never
// on a card's definition (R202). Moved: the barrels growing redder across the turn's four uses is one glow
// per burst, since a use cannot read the ones before it; a Repeater equipped into another Unit activates as
// that Unit; the cooling glow on its tile is rising embers.

import { ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxShotCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Its barrels' steel and its bolts' laser. */
const STEEL = "Physical/steel";
const LASER = "Tech/laser";

/** Its barrels' glow as they fire. */
const HOT = { rim: "#fb923c", core: "#ffedd5", glow: "#ea580c" } as const;

/** The bolts of one burst. */
const BOLTS = 3;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  casings: { count: 18, power: 1 },
} as const;

/** As shares of the entry: each bolt leaves `step` after the last and flies for `flight`. */
const TIMING = { step: 0.12, flight: 0.25 } as const;

export const barrelSpin: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("barrels", STEEL, plan),
    { ...tunedBurst(plan.intensity, "shard", tid(plan.tgt, FOOT), "area", slam, TUNING.casings), colors: lookOf(STEEL).colors },
    ringCue(plan.D, "spark", at, slam),
    ...slamShake(plan, "barrelSpin"),
  ];
};

export const repeaterBurst: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate" || event.type !== "damage") return null;
  const target = placeOf(p.view, event.targetId);
  if (target === null) return null;
  const from = cardAt(p.view, play);
  const flight = Math.round(TIMING.flight * p.D);
  const bolts = Array.from({ length: BOLTS }, (_, n): FxShotCue => ({
    kind: "shot",
    shot: { motion: "bolt", trail: "spark" },
    colors: lookOf(LASER).colors,
    preset: "spark",
    from,
    to: tid(target),
    delayMs: Math.round(n * TIMING.step * p.D),
    flightMs: flight,
    density: p.env.intensity,
  }));
  return replace([...bolts, { kind: "brand", at: from, tint: HOT, delayMs: 0, durationMs: p.D }]);
};
