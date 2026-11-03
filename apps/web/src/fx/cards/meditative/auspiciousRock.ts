// R2784000 #2796: Meditative #39.1 Auspicious Rock (`meditative-039-1`, bespoke "jadeCrack"): the jade
// line's gamble, which five cards deal and the census plays most, cracks the caster's hero as it pays
// its life and flies a jade glint into their hand. It never reads the card it rolled (R202): a Dud, a
// Jade and a Red Jade land alike, and the other seat's hand takes the same glint.

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_JADE_CRACK_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifShot, timesOf } from "../../motifs/build.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";
import { flourishSpec, flourishes } from "./flourish.ts";

/** The jade line's sub-look (#2792). */
const JADE = "Hustle/jade";

/** Meditative #39.1 Auspicious Rock: a crack on the caster's hero at its life loss, then the rolled card's flight into their hand. */
export const jadeCrack: CardRecipe = (event, p, play) => {
  if (!flourishes(p)) return null;
  if (event.type !== "healthLost" && event.type !== "addedToHand") return null;
  if (event.player !== play.player) return null;
  const t = timesOf(p.D);
  const spec = flourishSpec(p, play, JADE);
  const hero = heroOf(p.view, play);
  if (event.type === "healthLost") {
    return {
      cues: [
        motifBurst(spec, hero, "point", t.G, MOTIF_TUNING.gather, "crumb"),
        { kind: "crack", at: hero, delayMs: t.T, durationMs: p.D - t.T + FX_CRACK_TAIL_MS },
        motifBurst(spec, hero, "area", t.L, MOTIF_TUNING.land),
        ...shakeCues(spec.scale, FX_JADE_CRACK_TRAUMA, t.L),
      ],
      row: "keep",
    };
  }
  const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
  return {
    cues: [motifShot(spec, "bolt", hero, hand, t.T, t.L - t.T), motifBurst(spec, hand, "point", t.L, MOTIF_TUNING.small, "sparkle")],
    row: "keep",
  };
};
