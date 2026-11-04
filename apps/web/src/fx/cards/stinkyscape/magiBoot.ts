// R2797000: Stinkyscape #18 Imitated Instrumentality (`stinkyscape-018`, bespoke "magiBoot"), the
// Legendary that summons MELCHIOR, BALTHASAR and CASPER: each comes online as a hologram beam from its
// caster's hero into its zone, booting with a ring and a burst there. The runner gives each summon its
// own entry, so the three boot one after another; an entry holding several staggers their beams
// FX_MAGI_STAGGER of D apart (at most FX_MAGI_BOOTS of them), never landing past Linger. Each summon's
// own row still plays (`keep`).

import { targetFor } from "../../../game/animations.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_MAGI_BOOTS, FX_MAGI_STAGGER, FX_MAGI_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifRing, motifScale, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";

/** Every particle count the boot throws at scale 1 (rule 9). */
const TUNING = {
  boot: { count: 20, power: 1 },
} as const;

/** R2797000: Imitated Instrumentality's boots, one at each `summoned` of its play. */
export const magiBoot: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned") return null;
  const zone = targetFor(event, p.view);
  if (zone === null) return null;
  const look = paletteLook("Tech/hologram") ?? FLAVOR_LOOKS.Tech;
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const t = timesOf(p.D);
  const k = Math.min(p.entry.events.filter((candidate) => candidate.type === "summoned").indexOf(event), FX_MAGI_BOOTS - 1);
  const s = frac(FX_MAGI_STAGGER, t.D);
  const leave = t.T + k * s;
  const land = Math.min(t.Li, t.L + k * s);
  const at = tid(zone);
  const cues: FxCue[] = [
    motifShot(spec, "beam", heroOf(p.view, play), at, leave, land - leave),
    motifRing(spec, t.D, at, land),
    motifBurst(spec, at, "area", land, TUNING.boot),
  ];
  if (k === 0) cues.push(...shakeCues(spec.scale, FX_MAGI_TRAUMA, t.L));
  return { cues, row: "keep" };
};
