// R2797000: Stinkyscape #36 Drone Core X (`stinkyscape-036`, bespoke "battleshipWarp"), the Mythic top
// of the Drone Core line that summons the Drone Battleship: stars gather where it will stand, and it
// warps in from the board's top edge (FX_WARP_FROM) as a streak of starlight, landing in a blast of
// exhaust that shakes the board. It draws at the play's first `summoned`, once a play; the summon's own
// row and entrance still play (`keep`).

import { targetFor } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_WARP_FROM, FX_WARP_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { MOTIF_TUNING, motifBurst, motifRing, motifScale, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the warp throws at scale 1 (rule 9). */
const TUNING = {
  stars: { count: 24, power: 0.7 },
  exhaust: { count: 34, power: 1.4 },
} as const;

/** R2797000: Drone Core X's warp, at the play's first `summoned`. */
export const battleshipWarp: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned") return null;
  const own = p.entry.events.filter((candidate) => candidate.type === "summoned");
  if (own[0] !== event || play.seen.summoned !== own.length) return null;
  const zone = targetFor(event, p.view);
  if (zone === null) return null;
  const look = paletteLook("Tech/starfield") ?? FLAVOR_LOOKS.Tech;
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const t = timesOf(p.D);
  const at = tid(zone);
  return {
    cues: [
      motifBurst(spec, at, "area", t.G, TUNING.stars, "star"),
      ringCue(t.D, "star", at, t.T),
      motifShot(spec, "bolt", tid(testid.board, FX_WARP_FROM), at, t.T, t.L - t.T),
      motifBurst(spec, at, "area", t.L, TUNING.exhaust, "exhaust"),
      motifRing(spec, t.D, at, t.L),
      motifBurst(spec, at, "ring", t.Li, MOTIF_TUNING.linger, "star"),
      ...shakeCues(spec.scale, FX_WARP_TRAUMA, t.L),
    ],
    row: "keep",
  };
};
