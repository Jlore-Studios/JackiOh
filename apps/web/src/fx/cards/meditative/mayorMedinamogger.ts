// R3405001: Meditative #86 Mayor Medinamogger (`meditative-086`, entrance "coinFlip", a cash orbit on his
// tile), a Legendary Unit's signature: his entrance alone, no recipe. Bunting snaps across his zone as he
// strides on; he flips a silver coin high, a compass needle spinning beneath it, and catches it at the slam,
// a glint off the coin and confetti bursting over his zone for the crowd that cheers him unseen. His aura
// (every target chosen at random) is catalog data that no event names, so its spinning needle plays here,
// under his coin. It names no card (R202).

import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, type Entrance } from "../recipe.ts";

/** The look his entrance wears: Hustle's cash sub-look, his aura's. */
const MAYOR_PALETTE = "Hustle/cash";

/** Every particle count his entrance throws, at intensity "normal" (rule 9). */
const TUNING = { glint: { count: 8, power: 0.5 }, cheer: { count: 24, power: 1.1 } } as const;

/** As fractions of the motion after the lead: the coin leaves his hand at `flipAt`, caught at the slam; the bunting and the coin last the entry and `tailMs` more. */
const TIMING = { flipAt: 0.15, tailMs: 300 } as const;

/** Where the coin is caught, high in his zone, and where the crowd's confetti bursts, over its top. */
const CATCH_AT: FxPoint = { x: 0.5, y: 0.3 };
const CHEER_AT: FxPoint = { x: 0.5, y: 0.05 };

export const coinFlip: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(MAYOR_PALETTE) ?? FLAVOR_LOOKS.Hustle;
  const tint = figureTint(look);
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const flip = lead + frac(TIMING.flipAt, D - lead);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [
    figureCue("bunting", at, tint, lead, D - lead + TIMING.tailMs, side),
    figureCue("silverCoin", at, tint, flip, D - flip + TIMING.tailMs, side),
    { ...tunedBurst(i, "sparkle", tid(tgt, CATCH_AT), "point", slam, TUNING.glint), colors: look.colors },
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, tid(tgt, CHEER_AT), "area", slam, TUNING.cheer), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.coinFlip, slam),
  ];
  return cues;
};
