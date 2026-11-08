// R3400001: Meditative+ #1 Bob's Plan (`meditativeplus-001`, bespoke "bobsPlan", takeover "summit"), a
// Mythic Spell's signature: its takeover is Bob's ink-wash range with a red brushstroke climbing to the
// summit flag (takeoverArt.ts, the same whatever it picks), and as the hold ends ink splashes at the
// board's centre. Each card it adds rolls up as a scroll there and flies to the caster's hand pile as a
// card back, the plan's scroll smaller. It keys on its own play and how many cards that play added
// (`seen`), never on an added card's definition, and anchors on the board and the hand pile alone, the
// same on both seats (R202). A Discover's offer is a prompt, so no beat shows it.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_BOBS_PLAN_TRAUMA, FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** The sub-look it plans in: Nerd's ink, a brush on paper. */
const BOB_PALETTE = "Nerd/ink";

/** The ink it splashes as the hold ends, at intensity "normal" and scale 1 (rule 9). */
const TUNING = { ink: { count: 30, power: 1.2 } } as const;

/** When in its entry an added card's back leaves the scroll, as a fraction of the entry. */
const TIMING = { ghostAt: 0.2 } as const;

export const bobsPlan: CardRecipe = (event, p, play) => {
  const look = paletteLook(BOB_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const centre = tid(testid.board, FX_CENTER);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hold = p.entry.setPiece?.holdMs ?? 0;
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return { cues: [motifBurst(spec, centre, "area", hold, TUNING.ink), ...shakeCues(p.env.intensity, FX_BOBS_PLAN_TRAUMA, hold)], row: "replace" };
  }
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  // The pick first, its plan after it, smaller.
  const nth = play.seen.addedToHand ?? 1;
  const leave = frac(TIMING.ghostAt, p.D);
  return {
    cues: [
      figureCue("scroll", centre, figureTint(look), 0, p.D, nth > 1 ? { size: "small" } : {}),
      { kind: "ghost", from: centre, to: tid(animTestid.hand(sideOf(p.view, play.player))), delayMs: leave, durationMs: p.D - leave },
    ],
    row: "replace",
  };
};
