// #2799: #40 Goetzmann Game Design (`traininggrounds-040`, bespoke "gameDesign"), bespoke because it is the
// set's Mythic, its designer's own card, and `send`'s small flight undersells it: a card is drafted at the
// board's centre in Mythic rays, in Nerd's pixel sub-look (games and screens), then dealt into the caster's
// hand pile, landing in a prismatic burst and a sparkling afterglow. It keys on the caster's own card added,
// and anchors on the board and the hand pile alone, the same on both seats (R202); the hand's row still plays.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { raysCue, shakeCues, tid } from "../../build.ts";
import { FX_CENTER, FX_GAME_DESIGN_TRAUMA } from "../../constants.ts";
import { paletteLook } from "../../flavors.ts";
import { MOTIF_TUNING, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import { flourishSpec } from "../core/flourish.ts";
import type { CardRecipe } from "../recipe.ts";

/** The sub-look it drafts in (#2792): Nerd's pixel, for games and screens. */
const DESIGN_PALETTE = "Nerd/pixel";

export const gameDesign: CardRecipe = (event, p, play) => {
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  const own = flourishSpec(p, play);
  const spec = { ...own, look: paletteLook(DESIGN_PALETTE) ?? own.look };
  const t = timesOf(p.D);
  const centre = tid(testid.board, FX_CENTER);
  const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
  return {
    cues: [
      motifBurst(spec, centre, "area", t.G, MOTIF_TUNING.gather),
      raysCue(p.D, "mythic", centre, t.G),
      motifRing(spec, p.D, centre, t.T),
      { kind: "ghost", from: centre, to: hand, delayMs: t.T, durationMs: t.L - t.T },
      motifBurst(spec, hand, "point", t.L, MOTIF_TUNING.land, "prismatic"),
      motifBurst(spec, hand, "ring", t.Li, MOTIF_TUNING.linger, "sparkle"),
      ...shakeCues(spec.scale, FX_GAME_DESIGN_TRAUMA, t.L),
    ],
    row: "keep",
  };
};
