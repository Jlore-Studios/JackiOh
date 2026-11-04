// R2795000: Classic+ #41 KY's Constant (`classicplus-041`, bespoke "constantStamp") chalks its new number
// onto the card it changed: a ring of chalk dust closes on the card, a sheen wipes across it and the
// number lands in a puff of chalk. It reads the card the viewer's own view shows and nothing else: a
// change hidden from this seat, or a card its view does not draw, stamps none (R202). Its row (the
// number's glint) still plays.

import { HIDDEN_ID, targetFor } from "../../../game/animations.ts";
import { ringCue, tid } from "../../build.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the stamp throws at scale 1 (rule 9). */
const TUNING = {
  chalk: { count: 16, power: 0.8 },
} as const;

/** The classroom's chalk (#2792's sub-look), Nerd's own look if it were ever dropped. */
const CHALK = paletteLook("Nerd/chalk") ?? FLAVOR_LOOKS.Nerd;

/** Classic+ #41 KY's Constant stamps each `numberChanged` of its play on the card this seat reads. */
export const constantStamp: CardRecipe = (event, p, play) => {
  if (event.type !== "numberChanged" || event.instanceId === HIDDEN_ID || event.hiddenFrom?.includes(p.view.viewer) === true) return null;
  const tgt = targetFor(event, p.view);
  if (tgt === null) return null;
  const t = timesOf(p.D);
  const at = tid(tgt);
  const spec: CueSpec = { look: CHALK, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  return {
    cues: [
      ringCue(p.D, "dust", at, t.G),
      { kind: "sheen", at, delayMs: t.T, durationMs: t.L - t.T },
      motifBurst(spec, at, "area", t.L, TUNING.chalk),
    ],
    row: "keep",
  };
};
