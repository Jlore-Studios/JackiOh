// R3399000: Classic+ Jungle Loser (`classicplus-019-2`, entrance "vineNap"), his signature (#3399): vines
// drop into the zone one after another (three leaf bursts from its top, left to right), and he slides down
// one (a falling arrow, in step with the slam: leaves, a ring and the one shake), yawning, then curls up for
// a nap among the leaves (a slow ring of leaves at the end). R202: the zone is the only anchor.

import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, type Entrance } from "../recipe.ts";

/** Every particle count the vines throw, at intensity "normal" (rule 9). */
const TUNING = {
  vine: { count: 6, power: 0.6 },
  slide: { count: 14, power: 0.9 },
  nap: { count: 8, power: 0.4 },
} as const;

const LOOK = paletteLook("Nature") ?? FLAVOR_LOOKS.Nature;

/** Each vine's drop: where along the zone's top it hangs, and its fraction of the entry. */
const VINES: readonly { at: FxPoint; when: number }[] = [
  { at: { x: 0.2, y: 0 }, when: 0.1 },
  { at: { x: 0.5, y: 0 }, when: 0.25 },
  { at: { x: 0.8, y: 0 }, when: 0.4 },
];
const SLIDE_AT = 0.35;
const NAP_AT = 0.85;
const SLIDE_MS = 500;

export const vineNap: Entrance = ({ D, tgt, intensity: i }) => {
  const zone = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const drops = VINES.map((vine): FxCue => ({ ...tunedBurst(i, "leaf", tid(tgt, vine.at), "point", frac(vine.when, D), TUNING.vine), colors: LOOK.colors }));
  return [
    ...drops,
    { kind: "arrows", direction: "down", at: zone, delayMs: frac(SLIDE_AT, D), durationMs: SLIDE_MS },
    { ...tunedBurst(i, "leaf", zone, "area", slam, TUNING.slide), colors: LOOK.colors },
    ringCue(D, "leaf", zone, slam),
    { ...tunedBurst(i, "leaf", zone, "ring", frac(NAP_AT, D), TUNING.nap), colors: LOOK.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.vineNap, slam),
  ];
};
