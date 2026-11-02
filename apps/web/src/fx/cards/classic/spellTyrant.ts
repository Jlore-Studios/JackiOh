// R670: Spell Tyrant (`classic-056`, bespoke "tyrantSigil"): an arcane sigil that flares as it lands.

import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCE_TRAUMA, type Entrance } from "../recipe.ts";

/** Every particle count the sigil throws, at intensity "normal" (rule 9). */
const TUNING = {
  sigilSparkle: { count: 26, power: 0.9 },
  sigilArcane: { count: 48, power: 1.4 },
} as const;

export const tyrantSigil: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    ringCue(D, "arcane", at, 0),
    tunedBurst(i, "sparkle", at, "ring", 0, TUNING.sigilSparkle),
    raysCue(D, "mythic", at, slam),
    tunedBurst(i, "arcane", at, "area", slam, TUNING.sigilArcane),
    ...shakeCues(i, ENTRANCE_TRAUMA.tyrantSigil, slam),
  ];
};
