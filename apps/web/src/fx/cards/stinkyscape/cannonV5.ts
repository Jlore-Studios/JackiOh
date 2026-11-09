// R3395000: Stinkyscape #60 Cannon v5 (`stinkyscape-060`, entrance "breechLock", bespoke "cannonShell", a
// smoke curl on its tile), a Legendary Unit's signature. A single colossal barrel swings down into its zone
// and locks with a clang as the breech slides a shell home. At its Activate the recoil jolts the board, and
// one huge glowing shell arcs across to burst on its target in a crater ring and a heavy shake. It keys on
// its hook and its own public id, never on a card's definition (R202). Moved: a Cannon equipped into
// another Unit fires as that Unit; the smoke curl and breech light on its tile are slow rising smoke.

import { ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FOOT, projectile, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Its gunmetal. */
const GUNMETAL = "Tech/rocket";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  clang: { count: 20, power: 1 },
} as const;

/** As a share of the entry, how long the shell flies. */
const TIMING = { flight: 0.5 } as const;

/** The recoil as it fires, and the shell's impact (a Unit's at most 0.8). */
const RECOIL_TRAUMA = 0.3;
const IMPACT_TRAUMA = 0.6;

export const breechLock: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("cannon", GUNMETAL, plan),
    { ...tunedBurst(plan.intensity, "spark", at, "area", slam, TUNING.clang), colors: lookOf(GUNMETAL).colors },
    tunedBurst(plan.intensity, "smoke", tid(plan.tgt, FOOT), "point", slam, TUNING.clang),
    ringCue(plan.D, "smoke", at, slam),
    ...slamShake(plan, "breechLock"),
  ];
};

export const cannonShell: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate" || event.type !== "damage") return null;
  const target = placeOf(p.view, event.targetId);
  if (target === null) return null;
  const land = Math.round(TIMING.flight * p.D);
  const i = p.env.intensity;
  return replace([
    ...shakeCues(i, RECOIL_TRAUMA, 0),
    projectile(i, "fire", cardAt(p.view, play), tid(target), 0, land),
    ringCue(p.D, "fire", tid(target), land),
    ...shakeCues(i, IMPACT_TRAUMA, land),
  ]);
};
