// R2795000: Classic+ #35 Rollback (`classicplus-035`, bespoke "rollbackSweep") rewinds the board lane by
// lane: at its `rolledBack` an arcane ring runs back from lane 5 to lane 1 through the unit zones of
// every side it rolled back, in the card's sepia, and each of those heroes glints as the board lands
// where it was. Its row (`rewind`, the board's sheen and shake) still plays.

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, tid } from "../../build.ts";
import { FX_ROLLBACK_STEP } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flavorLookOf } from "../../flavorSource.ts";
import { motifBurst, motifScale, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the rewind throws at scale 1 (rule 9). */
const TUNING = {
  laneArcane: { count: 8, power: 0.7 },
  heroSparkle: { count: 14, power: 0.9 },
} as const;

/** Classic+ #35 Rollback plays at its play's first `rolledBack`, on the sides that event names. */
export const rollbackSweep: CardRecipe = (event, p, play) => {
  if (event.type !== "rolledBack" || play.seen.rolledBack !== 1) return null;
  const t = timesOf(p.D);
  const look = flavorLookOf(play.defId, play.instanceId, p.view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const sides = [...new Set(event.sides.map((player) => sideOf(p.view, player)))];
  const cues: FxCue[] = [];
  // Lane 5 first, back to lane 1, FX_ROLLBACK_STEP of D apart, every ring inside Travel to Land (R200).
  [...LANES].reverse().forEach((lane, k) => {
    const at = Math.min(t.L, t.T + frac(k * FX_ROLLBACK_STEP, p.D));
    for (const side of sides) {
      const zone = tid(testid.zone(side, "units", lane));
      cues.push(ringCue(p.D, "arcane", zone, at), motifBurst(spec, zone, "point", at, TUNING.laneArcane, "arcane"));
    }
  });
  for (const side of sides) cues.push(motifBurst(spec, tid(testid.hero(side)), "area", t.L, TUNING.heroSparkle, "sparkle"));
  return { cues, row: "keep" };
};
