// #2799: #34 Dino-size (`traininggrounds-034`, bespoke "dinoStomp"), bespoke because its one effect, a Unit
// set to 10/10, reports a `numberChanged` no motif reads, and no other Training Grounds card shows one: the
// unit swells under rising arrows and lands with a stomp, a ring of dust at its foot, a burst and a ring of
// the look and a shake past nova's, the dust settling after. It keys on the set's `attack` number
// alone, once a unit, on a unit the board shows and a number the viewer may read (R202); the rows still play.

import { testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_ARROWS_TAIL_MS, FX_DINO_STOMP_TRAUMA } from "../../constants.ts";
import { MOTIF_FOOT, MOTIF_TUNING, motifBurst, motifRing, placeOf, timesOf } from "../../motifs/build.ts";
import { flourishSpec } from "../core/flourish.ts";
import type { CardRecipe } from "../recipe.ts";

/** The number of the two a set reports (`attack`, then `health`) the stomp keys on, so it lands once a unit. */
const STOMP_KEY = "attack";

export const dinoStomp: CardRecipe = (event, p, play) => {
  if (event.type !== "numberChanged" || event.key !== STOMP_KEY) return null;
  if (event.hiddenFrom?.includes(p.view.viewer) === true || placeOf(p.view, event.instanceId) === null) return null;
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  const card = testid.card(event.instanceId);
  const at = tid(card);
  const foot = tid(card, MOTIF_FOOT);
  return {
    cues: [
      { kind: "arrows", direction: "up", at, delayMs: t.G, durationMs: t.D - t.G + FX_ARROWS_TAIL_MS },
      motifBurst(spec, foot, "ring", t.L, MOTIF_TUNING.nova, "dust"),
      motifBurst(spec, at, "area", t.L, MOTIF_TUNING.land),
      motifRing(spec, p.D, at, t.L),
      motifBurst(spec, foot, "area", t.Li, MOTIF_TUNING.linger, "dust"),
      ...shakeCues(spec.scale, FX_DINO_STOMP_TRAUMA, t.L),
    ],
    row: "keep",
  };
};
