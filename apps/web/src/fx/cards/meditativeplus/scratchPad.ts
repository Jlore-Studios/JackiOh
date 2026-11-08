// R3400001: Meditative+ #26 Scratch Pad (`meditativeplus-026`, bespoke "scratchPad", takeover "canvas"), a
// Mythic Spell's signature: its takeover empties the screen to blank paper over faint ink peaks, a cyan
// cursor blinking at its centre and ink rows laying down into a card's scaffold (takeoverArt.ts), and pixels
// spark as the hold ends. While the canvas is open (its prompts) nothing plays; the crafted card's
// scaffold folds into a card back that flies to the caster's hand pile, and as Scratch Pad exiles itself
// the paper tears away. It keys on its own play, never on the crafted card, which no cue names (R202).

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** The sub-look it codes in: Nerd's pixel, for screens. */
const PAD_PALETTE = "Nerd/pixel";

/** The pixels it sparks as the hold ends, at intensity "normal" and scale 1 (rule 9). */
const TUNING = { pixels: { count: 24, power: 1 } } as const;

/** As fractions of the entry: the scaffold folds until `foldEnd`, its back leaves at `ghostAt`, the paper tears until `tearEnd`. */
const TIMING = { foldEnd: 0.6, ghostAt: 0.4, tearEnd: 0.8 } as const;

export const scratchPad: CardRecipe = (event, p, play) => {
  const look = paletteLook(PAD_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const centre = tid(testid.board, FX_CENTER);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return { cues: [motifBurst(spec, centre, "area", p.entry.setPiece?.holdMs ?? 0, TUNING.pixels)], row: "replace" };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const leave = frac(TIMING.ghostAt, p.D);
    return {
      cues: [
        figureCue("scaffold", centre, figureTint(look), 0, frac(TIMING.foldEnd, p.D)),
        { kind: "ghost", from: centre, to: tid(animTestid.hand(sideOf(p.view, play.player))), delayMs: leave, durationMs: p.D - leave },
      ],
      row: "replace",
    };
  }
  if (event.type === "exiled" && event.instanceId === play.instanceId) {
    return { cues: [figureCue("tear", centre, figureTint(look), 0, frac(TIMING.tearEnd, p.D))], row: "replace" };
  }
  return null;
};
