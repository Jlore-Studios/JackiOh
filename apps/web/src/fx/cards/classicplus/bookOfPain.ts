// R2795000: Classic+ #56 Book of Pain (`classicplus-056`, bespoke "inkTear") tears each card it makes the
// opponent discard out in ink: a splash of ink where the card leaves, the first with a closing ring and
// a shake, and a drop of ink where it lands in the graveyard. Each card's own row (the back flying to the
// graveyard) still plays, and it reads only what that row reads (R202).

import { animTestid, targetFor } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_INK_TEAR_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the tear throws at scale 1 (rule 9). */
const TUNING = {
  tear: { count: 18, power: 1.1 },
  drop: { count: 8, power: 0.6 },
} as const;

/** The archive's ink (#2792's sub-look), Nerd's own look if it were ever dropped. */
const INK = paletteLook("Nerd/ink") ?? FLAVOR_LOOKS.Nerd;

/** Classic+ #56 Book of Pain tears out every card its play discards; the play's first tear rings and shakes. */
export const inkTear: CardRecipe = (event, p, play) => {
  if (event.type !== "discarded") return null;
  const tgt = targetFor(event, p.view);
  if (tgt === null) return null;
  const t = timesOf(p.D);
  const at = tid(tgt);
  const scale = motifScale(play.defId, p.view, p.env.intensity);
  const spec: CueSpec = { look: INK, scale, row: "keep" };
  const graveyard = tid(animTestid.graveyard(sideOf(p.view, event.owner)));
  const cues: FxCue[] = [motifBurst(spec, at, "area", t.G, TUNING.tear), motifBurst(spec, graveyard, "point", t.L, TUNING.drop)];
  // The play's first discard: the memory already holds this entry's events when it plans.
  const own = p.entry.events.filter((candidate) => candidate.type === "discarded");
  if ((play.seen.discarded ?? 0) === own.length && own[0] === event) {
    cues.push(ringCue(p.D, "arcane", at, t.G), ...shakeCues(scale, FX_INK_TEAR_TRAUMA, t.G));
  }
  return { cues, row: "keep" };
};
