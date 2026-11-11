// R3395000: Stinkyscape #70 Z'Zoth (`stinkyscape-070`, entrance "wraithGather", bespoke "ghostHulls", a
// shadow's drift on its tile), a Legendary Unit's signature. Debris swirls into a hooded wraith in its zone,
// a pale lantern eye opens and the light drains in a ring of void. Its Cry raises the fallen: a ghost hull
// (a card back) drifts from the caster's graveyard pile to each zone and solidifies in a ring of smoke as its
// `summoned` lands; on the Radiant face each takes Reborn's pale halo (a holy ring) as it is granted. It keys
// on its play, never on a card's definition (R202). Moved: the board's light draining to bone-white is the
// void ring; the debris orbiting its hood on its tile is a drift of shadow.

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardBack, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake, summonZone } from "./signatureKit.ts";

/** Its shadow. */
const SHADOW = "Dark/shadow";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  debris: { count: 24, power: 0.9 },
} as const;

/** As a share of the entry, when a raised hull solidifies. */
const TIMING = { solid: 0.7 } as const;

export const wraithGather: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("wraith", SHADOW, plan),
    { ...tunedBurst(plan.intensity, "smoke", tid(plan.tgt, FOOT), "area", slam, TUNING.debris), colors: lookOf(SHADOW).colors },
    ringCue(plan.D, "void", at, slam),
    ...slamShake(plan, "wraithGather"),
  ];
};

export const ghostHulls: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  if (event.type === "summoned" && event.player === play.player && event.instanceId !== play.instanceId) {
    const zone = tid(summonZone(p.view, event));
    const solid = Math.round(TIMING.solid * p.D);
    return replace([cardBack(tid(animTestid.graveyard(sideOf(p.view, event.player))), zone, 0, solid), ringCue(p.D, "smoke", zone, solid)]);
  }
  if (event.type === "keywordGranted" && event.instanceId !== play.instanceId) {
    const unit = placeOf(p.view, event.instanceId);
    return unit === null ? null : replace([ringCue(p.D, "holy", tid(unit), 0)]);
  }
  return null;
};
