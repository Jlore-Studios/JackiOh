// R2784000 #2796: what Meditative's and Meditative+'s Flourishes (jadeCrack, cnSweep, harmonyWave) draw
// with: the motifs' cue helpers (fx/motifs/build.ts) in the card's look at the card's scale, so a
// Flourish is built from motif primitives and adds no rendering.

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { flavorLookOf } from "../../flavorSource.ts";
import { motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxPlay } from "../../types.ts";
import type { CardFxPlan } from "../recipe.ts";

/**
 * A Flourish's look and scale: `palette`'s look, else the play's own flavor's (a Chaos card's pick per
 * instance), at the card's motif scale times the intensity; it plays over the event's own row.
 */
export function flourishSpec(p: CardFxPlan, play: FxPlay, palette?: FxPalette): CueSpec {
  const look = (palette !== undefined ? paletteLook(palette) : undefined) ?? flavorLookOf(play.defId, play.instanceId, p.view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  return { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
}

/** Whether a Flourish draws anything: never with the effects off, under reduced motion (an entry of no duration) or at no scale. */
export function flourishes(p: CardFxPlan): boolean {
  return p.env.intensity > 0 && p.D > 0;
}
