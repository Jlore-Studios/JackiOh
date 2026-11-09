// R3399000: Classic+ Classic Golem (`classicplus-073-1`, entrance "templeStack"), its signature (#3399): his entrance
// alone (his attack trigger is unmarked in the event stream).
// Temple stones tumble into the zone, three crumb bursts dropping from above, one after another; they stack into a
// golem (a rising column of dust bursts in the zone); at the landing both fists slam down, a burst under each fist
// and a ring, and the zone's floor cracks (a `crack` cue) under the Legendary's shake.
// R202: nothing here names a card.

import { frac, ringCue, shakeCues, tid } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { motifBurst, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, type Entrance } from "../recipe.ts";

/** Every particle count the temple throws, at intensity "normal" (rule 9). */
const TUNING = {
  stone: { count: 8, power: 0.8 },
  stack: { count: 8, power: 0.6 },
  fist: { count: 16, power: 1.2 },
} as const;

/** The stones tumble in at these fractions of D, each dropping from above the zone. */
const TUMBLE_AT = [0.05, 0.17, 0.29] as const;
/** The stack rises at these, low to high. */
const STACK_AT = [0.4, 0.46, 0.52] as const;
const STACK_Y = [0.85, 0.55, 0.25] as const;
const STONE_FROM = { x: 0.5, y: 0 } as const;
/** Each fist's x in the zone, left and right, and the floor they hit. */
const FIST_X = [0.25, 0.75] as const;
const FLOOR_Y = 1;
const CRACK_MS = FX_CRACK_TAIL_MS;

const look = FLAVOR_LOOKS.Brute;

export const templeStack: Entrance = ({ D, tgt, intensity: i }) => {
  const spec: CueSpec = { look, scale: i, row: "keep" };
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const cues: FxCue[] = [
    ...TUMBLE_AT.map((f) => motifBurst(spec, tid(tgt, STONE_FROM), "area", frac(f, D), TUNING.stone, "crumb")),
    ...STACK_AT.map((f, k) => motifBurst(spec, tid(tgt, { x: 0.5, y: STACK_Y[k] ?? 0.5 }), "point", frac(f, D), TUNING.stack, "dust")),
    ...FIST_X.map((x) => motifBurst(spec, tid(tgt, { x, y: FLOOR_Y }), "area", slam, TUNING.fist, "dust")),
    ringCue(D, "dust", at, slam),
    { kind: "crack", at, delayMs: slam, durationMs: CRACK_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.templeStack, slam),
  ];
  return cues;
};
