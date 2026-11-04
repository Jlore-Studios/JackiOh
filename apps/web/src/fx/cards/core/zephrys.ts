// #2793: #97 Zephrys (`core-097`, bespoke "perfectWish"), bespoke because its "perfect" card blows in
// from the far side of the board, which no hand motif says: a star gathers at the other hero, a bolt
// crosses to the caster's hand pile and lands in Mythic rays, a ring and a gold afterglow. It anchors on
// the heroes and the hand pile alone, the same on both seats (R202); the hand's own row still plays.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { raysCue, shakeCues, tid } from "../../build.ts";
import { FX_WISH_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifRing, motifShot, timesOf } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec } from "./flourish.ts";

export const perfectWish: CardRecipe = (event, p, play) => {
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  const far = tid(testid.hero(side === "you" ? "opponent" : "you"));
  const hand = tid(animTestid.hand(side));
  return {
    cues: [
      motifBurst(spec, far, "point", t.G, MOTIF_TUNING.gather, "star"),
      motifShot(spec, "bolt", far, hand, t.T, t.L - t.T),
      raysCue(p.D, "mythic", hand, t.L),
      motifBurst(spec, hand, "area", t.L, MOTIF_TUNING.land, "star"),
      motifRing(spec, p.D, hand, t.L),
      motifBurst(spec, hand, "ring", t.Li, MOTIF_TUNING.linger, "gold"),
      ...shakeCues(spec.scale, FX_WISH_TRAUMA, t.L),
    ],
    row: "keep",
  };
};
