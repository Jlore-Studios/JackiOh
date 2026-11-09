// R3395000: Stinkyscape #35 Drone Core IX (`stinkyscape-035`, bespoke "seedPod"), a Legendary Spell's
// signature. As it is cast a giant seed-pod core plunges onto the board's centre and splits into three
// hangar doors venting red steam, and three Drone Corvettes punch out of it in a wedge, each a rocket bolt
// streaking to the zone the next Corvette takes (the caster's first open unit zones, else the middle three
// lanes). Each Corvette's `summoned` lands in an exhaust ring, and the third throws the pod's shell plates
// away in one heavy shake. Every beat replaces its summon's own row, so the cast fits the Legendary Spell's
// ceiling (R3332000). It keys on its own play and how many summons it has seen, never on a card's
// definition (R202). Moved: the wedge flies inside the cast's entry, before the summons it heralds.

import type { PlayerView } from "@jackioh/shared";

import { sideOf, testid, type Side } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifBurst, motifScale, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { lookOf, replace } from "./signatureKit.ts";

/** The pod's red steam and the Corvettes' rockets. */
const STEAM = "Fire";
const ROCKET = "Tech/rocket";

/** The Corvettes the card summons (its declared `corvettes`), and the lanes the wedge takes on a full row. */
const CORVETTES = 3;
const WEDGE_LANES = [2, 3, 4] as const;

/** Every particle count the cast throws at scale 1 (rule 9). */
const TUNING = {
  steam: { count: 26, power: 1.1 },
} as const;

/** As shares of the entry: the pod splits at `split`, and each Corvette's bolt flies for `flight`, `step` after the last. */
const TIMING = { split: 0.3, flight: 0.3, step: 0.05 } as const;

/** The heavy shake as the third Corvette leaves the pod (a Spell's at most 1). */
const POD_TRAUMA = 0.7;

/** The zones the wedge flies to: the caster's open unit zones in lane order (R64's), the middle three on a full row. */
function wedgeZones(view: PlayerView, side: Side): string[] {
  const open = view[side].units.flatMap((unit, k) => (unit === null ? [k + 1] : []));
  const lanes = open.length > 0 ? open.slice(0, CORVETTES) : [...WEDGE_LANES];
  return lanes.map((lane) => testid.zone(side, "units", lane));
}

export const seedPod: CardRecipe = (event, p, play) => {
  const spec: CueSpec = { look: lookOf(ROCKET), scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" };
  const side = sideOf(p.view, play.player);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const centre = tid(testid.board, FX_CENTER);
    const split = Math.round(TIMING.split * p.D);
    const flight = Math.round(TIMING.flight * p.D);
    const wedge = wedgeZones(p.view, side).map((zone, n) => motifShot(spec, "bolt", centre, tid(zone), split + Math.round(n * TIMING.step * p.D), flight));
    return replace([
      figureCue("seedPod", centre, figureTint(lookOf(STEAM)), 0, p.D),
      motifBurst({ ...spec, look: lookOf(STEAM) }, centre, "area", split, TUNING.steam),
      ringCue(p.D, "fire", centre, split),
      ...wedge,
    ]);
  }
  if (event.type !== "summoned" || event.player !== play.player) return null;
  const n = play.seen.summoned ?? 1;
  const zone = tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane));
  const cues: FxCue[] = [{ kind: "sheen", at: zone, delayMs: 0, durationMs: p.D }];
  if (n < CORVETTES) cues.push(ringCue(p.D, "exhaust", zone, 0));
  if (n === CORVETTES) cues.push(...shakeCues(spec.scale, POD_TRAUMA, 0));
  return replace(cues);
};
