// R670: BOOM! Big Max (`classic-080`, bespoke "bigBoom"): a fuse, then a blast where it lands.

import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCE_FUSE_AT, ENTRANCE_TRAUMA, FOOT, type Entrance } from "../recipe.ts";

/** Every particle count the blast throws, at intensity "normal" (rule 9). */
const TUNING = {
  fuseSpark: { count: 16, power: 0.7 },
  boomFire: { count: 60, power: 1.6 },
  boomEmber: { count: 30, power: 1.2 },
  boomSmoke: { count: 24, power: 0.9 },
} as const;

export const bigBoom: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const fuse = frac(ENTRANCE_FUSE_AT, D);
  const slam = frac(FX_SLAM_AT, D);
  return [
    tunedBurst(i, "spark", at, "point", fuse, TUNING.fuseSpark),
    raysCue(D, "legendary", at, 0),
    ringCue(D, "fire", at, slam),
    tunedBurst(i, "fire", at, "area", slam, TUNING.boomFire),
    tunedBurst(i, "ember", at, "ring", slam, TUNING.boomEmber),
    tunedBurst(i, "smoke", tid(tgt, FOOT), "area", slam, TUNING.boomSmoke),
    { kind: "crack", at, delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.bigBoom, slam),
  ];
};
