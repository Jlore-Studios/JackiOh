// R2797000: Stinkyscape #36 Drone Core X (`stinkyscape-036`, bespoke "battleshipWarp"), the Mythic top
// of the Drone Core line that summons the Drone Battleship: stars gather where it will stand, and it
// warps in from the board's top edge (FX_WARP_FROM) as a streak of starlight, landing in a blast of
// exhaust that shakes the board. R3336000: the arrival is one beat per layer it summons, in stack order
// (R1842000), each on its own `summoned`: the warp drops the reactor keel first, the hangar decks lock
// onto it with two bolts from above, then the superstructure comes down and its dome rises in a flare. The
// summon's own row and entrance still play (`keep`).

import { targetFor } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_SHIP_ARRIVAL_BEATS, FX_SHIP_DOME_TRAUMA, FX_SHIP_LOCK_TRAUMA, FX_WARP_FROM, FX_WARP_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { MOTIF_TUNING, motifBurst, motifRing, motifScale, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import { shieldCues } from "../../shield.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the warp throws at scale 1 (rule 9). */
const TUNING = {
  stars: { count: 24, power: 0.7 },
  exhaust: { count: 34, power: 1.4 },
  lock: { count: 20, power: 1.1 },
} as const;

/** R3336000: where the hangar's two bolts start, above the zone's top corners, and the superstructure's, above its middle (fractions of the zone). */
const LOCK_FROM = [
  { x: 0.1, y: -0.4 },
  { x: 0.9, y: -0.4 },
] as const;
const DOME_FROM = { x: 0.5, y: -0.8 } as const;

/**
 * R2797000, R3336000: Drone Core X's warp: the layer each of the play's first `FX_SHIP_ARRIVAL_BEATS` summons is
 * (the reactor keel, the hangar decks, the superstructure) arrives as its own beat; any later summon of the play is its own.
 */
export const battleshipWarp: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned") return null;
  const own = p.entry.events.filter((candidate) => candidate.type === "summoned");
  // Which of the play's summons this is: `seen` counts this entry's own, the last of them being its tail.
  const beat = (play.seen.summoned ?? 0) - own.length + own.indexOf(event) + 1;
  if (beat < 1 || beat > FX_SHIP_ARRIVAL_BEATS) return null;
  const zone = targetFor(event, p.view);
  if (zone === null) return null;
  const look = paletteLook("Tech/starfield") ?? FLAVOR_LOOKS.Tech;
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const t = timesOf(p.D);
  const at = tid(zone);
  if (beat === 1) {
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
  }
  if (beat === 2) {
    // The hangar decks lock onto the keel: a bolt from above each corner, meeting on it.
    const cues: FxCue[] = LOCK_FROM.map((from) => motifShot(spec, "bolt", tid(zone, from), at, t.T, t.L - t.T));
    cues.push(
      motifRing(spec, t.D, at, t.L),
      motifBurst(spec, at, "ring", t.L, TUNING.lock, "spark"),
      ...shakeCues(spec.scale, FX_SHIP_LOCK_TRAUMA, t.L),
    );
    return { cues, row: "keep" };
  }
  // The superstructure comes down on the hangar and its dome flares up over the whole ship.
  return {
    cues: [
      motifShot(spec, "bolt", tid(zone, DOME_FROM), at, t.T, t.L - t.T),
      ...shieldCues("full", at, t.D, t.L, spec.scale),
      ...shakeCues(spec.scale, FX_SHIP_DOME_TRAUMA, t.L),
    ],
    row: "keep",
  };
};
