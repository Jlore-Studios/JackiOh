// #2799: #40 Goetzmann Game Design (`traininggrounds-040`, bespoke "gameDesign"), bespoke because it is the
// set's Mythic, its designer's own card, and `send`'s small flight undersells it. R3400001: its signature
// (takeover "napkin"): the takeover is a paper napkin under canteen lamplight, a pencil stub sketching a
// card and silhouettes of every set's cards spiralling up off it (takeoverArt.ts); as the hold ends, cyan
// and magenta pixels spark at the board's centre with a shake; the card it adds lifts off the napkin as a
// sketch, inks itself into a card back and is dealt into the caster's hand pile. It keys on its own play
// and the caster's own card added, never on that card, whose face no cue shows, and anchors on the board
// and the hand pile alone, the same on both seats (R202). Its beats stand in for the hand's row.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_CENTER, FX_GAME_DESIGN_TRAUMA } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** The sub-look its sparks wear: Tech's glitch, cyan and magenta pixels. */
const DESIGN_PALETTE = "Tech/glitch";

/** The pixels it sparks as the hold ends, at intensity "normal" and scale 1 (rule 9). */
const TUNING = { pixels: { count: 28, power: 1.2 } } as const;

/** As fractions of the entry: the sketch lifts until `sketchEnd`, its card back leaving at `ghostAt`. */
const TIMING = { sketchEnd: 0.6, ghostAt: 0.4 } as const;

export const gameDesign: CardRecipe = (event, p, play) => {
  const look = paletteLook(DESIGN_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const centre = tid(testid.board, FX_CENTER);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hold = p.entry.setPiece?.holdMs ?? 0;
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return { cues: [motifBurst(spec, centre, "area", hold, TUNING.pixels), ...shakeCues(p.env.intensity, FX_GAME_DESIGN_TRAUMA, hold)], row: "replace" };
  }
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  const leave = frac(TIMING.ghostAt, p.D);
  return {
    cues: [
      figureCue("sketch", centre, figureTint(look), 0, frac(TIMING.sketchEnd, p.D)),
      { kind: "ghost", from: centre, to: tid(animTestid.hand(sideOf(p.view, play.player))), delayMs: leave, durationMs: p.D - leave },
    ],
    row: "replace",
  };
};
