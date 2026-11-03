// #2799: #16 Celestial Caller (`traininggrounds-016`, bespoke "celestialCall"), bespoke because its Cry
// calls a Sun and a Moon at once, in two looks one motif's palette cannot hold: each summon is a beam
// from the Caller into the new card's zone, landing in the light of its own sky (Holy's gold sunburst
// for the Sun, Spirit's shaft of moonlight for the Moon), a ring and an afterglow. It anchors on the
// Caller, the summoned zones and the hero alone, the same on both seats (R202); the summon rows still play.

import { locateInstance, targetFor } from "../../../game/animations.ts";
import { raysCue, shakeCues, tid } from "../../build.ts";
import { FX_CELESTIAL_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS, type FlavorLook } from "../../flavors.ts";
import { flies, MOTIF_TUNING, motifBurst, motifRing, motifShot, timesOf } from "../../motifs/build.ts";
import type { FxCue, FxPreset, FxRayTone } from "../../types.ts";
import { flourishSpec } from "../core/flourish.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";

type Sky = { look: FlavorLook; rays: FxRayTone; glow: FxPreset };

/** #6 Chosen by the Sun's and #7 Chosen by the Moon's skies: each one's look, rays and afterglow. */
const SKIES: ReadonlyMap<string, Sky> = new Map<string, Sky>([
  ["traininggrounds-006", { look: FLAVOR_LOOKS.Holy, rays: "radiant", glow: "gold" }],
  ["traininggrounds-007", { look: FLAVOR_LOOKS.Spirit, rays: "holy", glow: "star" }],
]);

export const celestialCall: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned" || event.player !== play.player) return null;
  const sky = SKIES.get(event.defId);
  const target = targetFor(event, p.view);
  if (sky === undefined || target === null) return null;
  const spec = { ...flourishSpec(p, play), look: sky.look };
  const t = timesOf(p.D);
  const at = tid(target);
  const caller = locateInstance(p.view, play.instanceId);
  const from = caller === null ? heroOf(p.view, play) : tid(caller);
  const cues: FxCue[] = [motifBurst(spec, from, "point", t.G, MOTIF_TUNING.gather)];
  if (flies(from, at)) cues.push(motifShot(spec, "beam", from, at, t.T, t.L - t.T));
  cues.push(
    raysCue(p.D, sky.rays, at, t.L),
    motifBurst(spec, at, "area", t.L, MOTIF_TUNING.land),
    motifRing(spec, p.D, at, t.L),
    motifBurst(spec, at, "point", t.Li, MOTIF_TUNING.linger, sky.glow),
    ...shakeCues(spec.scale, FX_CELESTIAL_TRAUMA, t.L),
  );
  return { cues, row: "keep" };
};
