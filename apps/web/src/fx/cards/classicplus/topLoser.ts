// R3399000: Classic+ Top Loser (`classicplus-019-1`, entrance "shieldPlant"), his signature (#3399): he
// trudges into his zone alone (two heavy puffs of dust at his foot, one step after another), plants his
// tower shield with a thud (shards, a ring and a plated shield flash at the slam, the screen shaking once),
// a glint running over the steel, and turns his back on the rest of the board (a last dust at the zone's
// edge). His lane-wall text is the tile aura, drawn elsewhere. R202: the zone is the only anchor.

import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type Entrance } from "../recipe.ts";

/** Every particle count the plant throws, at intensity "normal" (rule 9). */
const TUNING = {
  step: { count: 6, power: 0.5 },
  plant: { count: 16, power: 1.1 },
  turn: { count: 6, power: 0.5 },
} as const;

const LOOK = paletteLook("Physical/steel") ?? FLAVOR_LOOKS.Physical;

/** The two steps before the slam, the glint after it, and the turn at the end. */
const STEP_AT = [0.2, 0.4] as const;
const GLINT_AT = 0.7;
const TURN_AT = 0.85;
const GLINT_MS = 400;
const SHIELD_MS = 500;
/** The back of the zone, where he turns away. */
const BACK: FxPoint = { x: 0.85, y: 1 };

export const shieldPlant: Entrance = ({ D, tgt, intensity: i }) => {
  const zone = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const steps = STEP_AT.map((at): FxCue => ({ ...tunedBurst(i, "dust", tid(tgt, FOOT), "point", frac(at, D), TUNING.step), colors: LOOK.colors }));
  return [
    ...steps,
    { ...tunedBurst(i, LOOK.particle, zone, "area", slam, TUNING.plant), colors: LOOK.colors },
    ringCue(D, LOOK.particle, zone, slam),
    { kind: "shield", size: "full", at: zone, delayMs: slam, durationMs: SHIELD_MS },
    { kind: "sheen", at: zone, delayMs: frac(GLINT_AT, D), durationMs: GLINT_MS },
    tunedBurst(i, "dust", tid(tgt, BACK), "point", frac(TURN_AT, D), TUNING.turn),
    ...shakeCues(i, ENTRANCE_TRAUMA.shieldPlant, slam),
  ];
};
