// #2793: what Core's bespoke flourishes (transmogulate.ts, zephrys.ts, craftACard.ts) share, and
// Stinkyscape's (#2798, ../stinkyscape/) and Training Grounds' (#2799, ../traininggrounds/): the caster's
// look and scale, as a motif reads them (motifs/build.ts), so each is built from the motif primitives at
// the scale its cost and rarity give it.

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flavorLookOf } from "../../flavorSource.ts";
import { motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxPlay } from "../../types.ts";
import type { CardFxPlan } from "../recipe.ts";

/** The resolving play's own look (its flavor's, a Chaos card's pick) and its motif scale, the intensity in it. */
export function flourishSpec(p: CardFxPlan, play: FxPlay): CueSpec {
  return {
    look: flavorLookOf(play.defId, play.instanceId, p.view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK],
    scale: motifScale(play.defId, p.view, p.env.intensity),
    row: "keep",
  };
}
