// R3400001: Training Grounds #39 Call to Chaos (Training-Grounds Edition) (`traininggrounds-039`, bespoke
// "chaosDice"), a Legendary Spell's signature: two carved wooden dice tumble across the board from the
// caster's side and clatter to a stop at its centre in magenta and teal sparks; on its roll the top face
// lights with the outcome's emblem as R436's panel names the clause, its reels landing within
// FX_CHAOS_DICE_REEL_MS and the panel gone after FX_CHAOS_DICE_PANEL_MS, so the cast keeps the Legendary
// Spell's ceiling (R3332000). The outcome's own rows play its beat (runners, shields, card backs); a Call
// it casts rolls smaller dice. The roll is public on both seats (R97 redacts only the card), and no cue
// names a card (R202).

import { sideOf, testid } from "../../../game/contract.ts";
import { frac, tid, tunedBurst } from "../../build.ts";
import { chaosCues } from "../../chaos.ts";
import { FX_CENTER, FX_CHAOS_DICE_PANEL_MS, FX_CHAOS_DICE_REEL_MS } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

export const CHAOS_DICE_ID = "traininggrounds-039";

/** Its sparks' look: Tech's glitch sub-look, magenta and teal. */
const DICE_PALETTE = "Tech/glitch";

/** The sparks the dice throw as they stop, at intensity "normal" (rule 9). */
const TUNING = { sparks: { count: 24, power: 1.2 } } as const;

/** The dice tumble the entry and `tailMs` more, and clatter to a stop at `stopAt` of it. */
const TIMING = { tailMs: 400, stopAt: 0.6 } as const;

export const chaosDice: CardRecipe = (event, p, play) => {
  const look = paletteLook(DICE_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const tint = figureTint(look);
  const centre = tid(testid.board, FX_CENTER);
  // A Call this one cast rolls a second, smaller pair.
  const size = p.env.memory.openPlay(CHAOS_DICE_ID)?.instanceId === play.instanceId ? {} : { size: "small" as const };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const stop = frac(TIMING.stopAt, p.D);
    return {
      cues: [
        figureCue("die", centre, tint, 0, p.D + TIMING.tailMs, { side: sideOf(p.view, play.player), ...size }),
        { ...tunedBurst(p.env.intensity, look.particle, centre, "area", stop, TUNING.sparks), colors: look.colors },
      ],
      row: "replace",
    };
  }
  if (event.type === "chaosRolled" && event.instanceId === play.instanceId) {
    const panel = chaosCues(event, FX_CHAOS_DICE_REEL_MS, p.env.intensity).find((cue) => cue.kind === "chaos");
    const cues: FxCue[] = panel === undefined ? [] : [{ ...panel, durationMs: FX_CHAOS_DICE_PANEL_MS }];
    cues.push(figureCue("dieFace", centre, tint, 0, FX_CHAOS_DICE_PANEL_MS, size));
    return { cues, row: "replace" };
  }
  return null;
};
