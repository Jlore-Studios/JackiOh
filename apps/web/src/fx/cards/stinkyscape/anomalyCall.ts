// R3337003: Stinkyscape's Anomaly Found! (`stinkyscape-t-anomaly-found`, bespoke "anomalyCall"). Its cast
// calls the Anomaly: as its `boardReplaced` opens the rift (fx/anomaly.ts, whose set piece claims that
// entry, whichever card laid it), a void ring and a burst of stars gather over the middle of its
// caster's side, where the rift tears. The event's own row still plays (`keep`), which the set piece
// draws in place of. The play is public, and so is the board it lays (R3337000, R202).

import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, tid } from "../../build.ts";
import { FX_MOTIF_CENTER_LANE } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the call throws at scale 1 (rule 9). */
const TUNING = {
  call: { count: 22, power: 1 },
} as const;

/** R3337003: Anomaly Found!'s call, at its play's `boardReplaced`. */
export const anomalyCall: CardRecipe = (event, p, play) => {
  if (event.type !== "boardReplaced" || event.player !== play.player) return null;
  const spec: CueSpec = { look: paletteLook("Tech/starfield") ?? FLAVOR_LOOKS.Tech, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const t = timesOf(p.D);
  const centre = tid(testid.zone(sideOf(p.view, event.player), "units", FX_MOTIF_CENTER_LANE));
  return { cues: [ringCue(t.D, "void", centre, t.G), motifBurst(spec, centre, "area", t.G, TUNING.call, "star")], row: "keep" };
};
