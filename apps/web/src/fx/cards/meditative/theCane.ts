// R3405001: Meditative #71.1 The Cane (`meditative-071-1`, a token printed Mythic, entrance "caneRise",
// takeover "caneRack", a storm aura on its zone), a Mythic Field Spell's signature: its entrance and its
// takeover alone, no recipe. After the takeover's hold, which it gets whether it is played or put onto the
// field face up by Pareto Optimality (R3405000), a red-corded bamboo cane floats up off its backrow zone,
// crackling white with storm light, and settles at the slam in a white ring. Its Animated on its owner's
// turn runs no hook, so the event rows play that move. Its entrance keys on its own readable `summoned`,
// and no cue names a card (R202).

import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type Entrance } from "../recipe.ts";

/** The look its storm light wears: Physical's steel, white and silver. */
const CANE_PALETTE = "Physical/steel";

/** Every particle count it throws, at intensity "normal" (rule 9): the crackle as it lifts, at its height, and as it settles. */
const TUNING = {
  lift: { count: 12, power: 0.8 },
  crackle: { count: 10, power: 1.3 },
  settle: { count: 16, power: 1 },
} as const;

/** The cane floats from the lead to `caneTailMs` past its entry, crackling again at `crackleAt` of its motion. */
const TIMING = { caneTailMs: 350, crackleAt: 0.3 } as const;

/** Where it crackles at its height: above its zone's middle. */
const CRACKLE_AT = { x: 0.5, y: 0.2 } as const;

export const caneRise: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(CANE_PALETTE) ?? FLAVOR_LOOKS.Physical;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const cues: FxCue[] = [
    figureCue("bambooCane", at, figureTint(look), lead, D - lead + TIMING.caneTailMs, view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) }),
    { ...tunedBurst(i, "spark", tid(tgt, FOOT), "area", lead, TUNING.lift), colors: look.colors },
    { ...tunedBurst(i, "spark", tid(tgt, CRACKLE_AT), "point", lead + frac(TIMING.crackleAt, D - lead), TUNING.crackle), colors: look.colors },
    ringCue(D, "spark", at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.settle), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.caneRise, slam),
  ];
  return cues;
};
