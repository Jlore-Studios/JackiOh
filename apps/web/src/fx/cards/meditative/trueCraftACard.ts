// R3405001: Meditative #17 True Craft a Card (`meditative-017`, bespoke "trueCraft", takeover "workshop"), a
// Mythic Spell's signature in Nerd's pixel look: its takeover is the workshop (takeoverArt.ts), and as the
// hold ends pixel threads stitch a blank card together at the board's centre in a spray of pixels. While
// its prompts are open (the cost, then the craft) the caster has the editor, and the other seat sees only
// the blank card humming there as each opens; the minted card cools into a card back that flies to the
// caster's hand pile, its face's sheen the caster's alone, and as the Spell exiles itself it crumbles into
// pixels. It keys on its own play, never on the crafted card, which no cue names (R202).

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe, RecipeFixture } from "../recipe.ts";

/** The sub-look it crafts in: Nerd's pixel, for screens. */
const CRAFT_PALETTE = "Nerd/pixel";

/** The pixels it sprays as the hold ends, at scale 1 (rule 9). */
const TUNING = { pixels: { count: 24, power: 1 } } as const;

/** The threads stitch, and hum at a prompt, `tailMs` past their entry; the back leaves at `ghostAt` of its entry; the Spell crumbles until `crumbleEnd` of its exile's. */
const TIMING = { tailMs: 400, ghostAt: 0.3, crumbleEnd: 0.8 } as const;

export const trueCraft: CardRecipe = (event, p, play) => {
  const look = paletteLook(CRAFT_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const tint = figureTint(look);
  const centre = tid(testid.board, FX_CENTER);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hold = p.entry.setPiece?.holdMs ?? 0;
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return { cues: [figureCue("pixelThreads", centre, tint, hold, p.D - hold + TIMING.tailMs), motifBurst(spec, centre, "area", hold, TUNING.pixels)], row: "replace" };
  }
  // The caster's prompts open the editor on its seat; the other seat sees only the blank card hum.
  if (event.type === "promptOpened" && event.player === play.player) {
    return p.view.viewer === play.player ? null : { cues: [figureCue("pixelThreads", centre, tint, 0, p.D + TIMING.tailMs)], row: "replace" };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
    const leave = frac(TIMING.ghostAt, p.D);
    const cues: FxCue[] = [{ kind: "ghost", from: centre, to: hand, delayMs: leave, durationMs: p.D - leave }];
    if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: hand, delayMs: leave, durationMs: p.D - leave });
    return { cues, row: "replace" };
  }
  if (event.type === "exiled" && event.instanceId === play.instanceId) {
    return { cues: [figureCue("pixelCrumble", centre, tint, 0, frac(TIMING.crumbleEnd, p.D))], row: "replace" };
  }
  return null;
};

/** The gallery's True Craft a Card: the minted card to the hand, then the Spell exiling itself. */
export const trueCraftFixture: RecipeFixture = (_view, instanceId, player) => [
  { type: "addedToHand", player, instanceId: "fx-crafted", defId: "core-055" },
  { type: "exiled", instanceId, defId: "meditative-017", owner: player },
];
