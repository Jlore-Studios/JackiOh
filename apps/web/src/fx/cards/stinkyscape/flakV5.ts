// R3395000: Stinkyscape #61 Flak v5 (`stinkyscape-061`, entrance "turretScan", bespoke "flakWaves", dust in
// a searchlight on its tile), a Legendary Unit's signature. A squat turret bristling with short barrels
// rises into its zone and swivels a searchlight across the enemy side. At its Activate the first wave of
// shrapnel blooms over every enemy Unit it hits, a sky of bursts above each; the later waves' hits draw a
// puff of smoke alone, and then the sky clears. It keys on its hook, its own hits and the board's public
// Units, never on a card's definition (R202). Moved: the second and third waves are smoke puffs, inside the
// Legendary's ceiling; a Flak equipped into another Unit fires as that Unit; the searchlight on its tile is
// a drift of dust.

import { sideOf } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import { otherSide } from "../../motifs/build.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { entranceFigure, lookOf, placeOf, replace, shownUnitZones, slamAt, slamShake } from "./signatureKit.ts";

/** Its shrapnel's rocket fire. */
const FLAK = "Tech/rocket";

/** Where a wave blooms: above the Unit it hits. */
const ABOVE = { x: 0.5, y: -0.15 } as const;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  dust: { count: 18, power: 0.9 },
  bloom: { count: 16, power: 1.1 },
} as const;

export const turretScan: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("turret", FLAK, plan),
    tunedBurst(plan.intensity, "dust", tid(plan.tgt, FOOT), "area", slam, TUNING.dust),
    ringCue(plan.D, "smoke", at, slam),
    ...slamShake(plan, "turretScan"),
  ];
};

export const flakWaves: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate" || event.type !== "damage") return null;
  const target = placeOf(p.view, event.targetId);
  if (target === null) return null;
  // The first wave hits each enemy Unit the board shows once; every hit after them is a later wave's.
  const enemies = shownUnitZones(p.view, otherSide(sideOf(p.view, play.player))).length;
  const first = (play.seen.damage ?? 1) <= Math.max(1, enemies);
  if (!first) return replace([tunedBurst(p.env.intensity, "smoke", tid(target, ABOVE), "point", 0, TUNING.dust)]);
  return replace([{ ...tunedBurst(p.env.intensity, "spark", tid(target, ABOVE), "area", 0, TUNING.bloom), colors: lookOf(FLAK).colors }]);
};
