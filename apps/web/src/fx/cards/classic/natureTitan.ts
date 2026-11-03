// R670: Nature Titan (`classic-045`, bespoke "titanBloom"): a column of light and a bloom where it lands.

import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCE_TRAUMA, FOOT, type Entrance } from "../recipe.ts";

/** Every particle count the bloom throws, at intensity "normal" (rule 9). */
const TUNING = {
  bloomSparkle: { count: 34, power: 1.1 },
  bloomGold: { count: 30, power: 1.2 },
  bloomDust: { count: 24, power: 1 },
} as const;

export const titanBloom: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    raysCue(D, "holy", at, 0),
    ringCue(D, "dust", at, slam),
    tunedBurst(i, "dust", tid(tgt, FOOT), "ring", slam, TUNING.bloomDust),
    tunedBurst(i, "sparkle", at, "area", slam, TUNING.bloomSparkle),
    tunedBurst(i, "gold", at, "ring", slam, TUNING.bloomGold),
    ...shakeCues(i, ENTRANCE_TRAUMA.titanBloom, slam),
  ];
};
