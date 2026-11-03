// R670: #100 Ceaseless Void (`core-100`, bespoke "voidCollapse"): its own entrance in place of a Mythic's
// rays and prismatic burst; its takeover of the board is takeover.ts's (R2722001).

import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCE_TRAUMA, type Entrance } from "../recipe.ts";

/** Every particle count the collapse throws, at intensity "normal" (rule 9). */
const TUNING = {
  voidPull: { count: 36, power: 0.8 },
  voidBurst: { count: 56, power: 1.5 },
  voidShard: { count: 18, power: 1.2 },
} as const;

export const voidCollapse: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    ringCue(D, "void", at, 0),
    tunedBurst(i, "void", at, "ring", 0, TUNING.voidPull),
    raysCue(D, "mythic", at, 0),
    tunedBurst(i, "void", at, "area", slam, TUNING.voidBurst),
    tunedBurst(i, "shard", at, "ring", slam, TUNING.voidShard),
    { kind: "crack", at, delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.voidCollapse, slam),
  ];
};
