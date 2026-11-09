// R3395000: Stinkyscape #16 Rod of Ages (`stinkyscape-016`, entrance "rodRise", bespoke "rodBands",
// takeover "rod", silver motes rising in its zone), a Mythic Field Spell's signature. Its takeover is a
// starless gulf with the colossal rod rising past the camera, its twelve bands catching light one by one
// (fx/takeoverArt.ts); then the rod rises into its zone, its bands lighting bottom to top, in a silver ring.
// Each `keywordGranted` its End of turn gives it engraves a new band, a silver ring and a sheen on its zone,
// and at its twelfth keyword `radiantSet` floods it with Radiant rays. It keys on its hook and its own
// public id, never on a card's definition (R202). Moved: the first Unit melting up its shaft as its `fused`
// lands happens in that Unit's play, which keys nothing of the Rod's (and a fused Rod's id is the Unit's);
// the bands orbiting its shaft on its tile are pale silver motes rising.

import { raysCue, ringCue, tid, tunedBurst } from "../../build.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, entranceFigure, lookOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** The silver of its bands. */
const SILVER = "Physical/blade";

/** Every particle count the Rod throws, at intensity "normal" (rule 9). */
const TUNING = {
  rise: { count: 26, power: 1.1 },
  motes: { count: 14, power: 0.7 },
} as const;

export const rodRise: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  const i = plan.intensity;
  return [
    entranceFigure("rod", SILVER, plan),
    { ...tunedBurst(i, "sparkle", at, "area", slam, TUNING.rise), colors: lookOf(SILVER).colors },
    tunedBurst(i, "holy", tid(plan.tgt, FOOT), "ring", slam, TUNING.motes),
    ringCue(plan.D, "holy", at, slam),
    ...slamShake(plan, "rodRise"),
  ];
};

export const rodBands: CardRecipe = (event, p, play) => {
  if (event.type === "keywordGranted" && event.instanceId === play.instanceId) {
    // A band is engraved: a silver ring and a sheen on the rod.
    const rod = cardAt(p.view, play);
    return replace([ringCue(p.D, "frost", rod, 0), { kind: "sheen", at: rod, delayMs: 0, durationMs: p.D }]);
  }
  if (event.type === "radiantSet" && event.instanceId === play.instanceId) {
    // The twelfth band: Radiant rays flood the rod.
    return replace([raysCue(p.D, "radiant", cardAt(p.view, play), 0)]);
  }
  return null;
};
