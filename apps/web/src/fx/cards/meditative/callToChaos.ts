// R3405001: Meditative #95 Call to Chaos (Meditative Edition) (`meditative-095`, bespoke "jadeDie"), a
// Legendary Spell's signature: a many-sided jade die crashes onto the board's centre, raked-gravel ripples
// cracking outward from where it lands, and spins on its point, toppling onto one glowing face as it
// slows. On its roll the outcome bursts from that face in a ring of jade light as R436's panel names the
// clause, its reels landing within FX_CHAOS_DICE_REEL_MS and the panel gone after FX_CHAOS_DICE_PANEL_MS, so
// the cast keeps the Legendary Spell's ceiling (R3332000), as Training Grounds' chaosDice.ts does. The
// outcome's own rows play its beat (summons, armour, bounces, card backs); a Call it casts rolls a smaller
// die. The roll is public on both seats (R97 redacts only the card), and no cue names a card (R202).

import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, tid } from "../../build.ts";
import { chaosCues } from "../../chaos.ts";
import { FX_CENTER, FX_CHAOS_DICE_PANEL_MS, FX_CHAOS_DICE_REEL_MS } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe, RecipeFixture } from "../recipe.ts";

export const JADE_DIE_ID = "meditative-095";

/** The sub-look it is carved from: Hustle's jade. */
const DIE_PALETTE = "Hustle/jade";

/** The gravel it throws as it lands, at scale 1 (rule 9). */
const TUNING = { gravel: { count: 22, power: 1.1 } } as const;

/** The die crashes, spins and topples over the entry and `tailMs` more, landing at `crashAt` of it. */
const TIMING = { tailMs: 400, crashAt: 0.4 } as const;

/** The gallery's roll: one clause of the edition's ten. */
const JADE_DIE_FIXTURE_EFFECT = "Your hero gains 5 Armor and you heal it 5";

export const jadeDie: CardRecipe = (event, p, play) => {
  const look = paletteLook(DIE_PALETTE) ?? FLAVOR_LOOKS.Hustle;
  const centre = tid(testid.board, FX_CENTER);
  // A Call this one cast rolls a smaller die.
  const size = p.env.memory.openPlay(JADE_DIE_ID)?.instanceId === play.instanceId ? {} : { size: "small" as const };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const crash = frac(TIMING.crashAt, p.D);
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return {
      cues: [
        figureCue("jadeDie", centre, figureTint(look), 0, p.D + TIMING.tailMs, { side: sideOf(p.view, play.player), ...size }),
        ringCue(p.D, "dust", centre, crash),
        motifBurst(spec, centre, "ring", crash, TUNING.gravel, "crumb"),
      ],
      row: "replace",
    };
  }
  if (event.type === "chaosRolled" && event.instanceId === play.instanceId) {
    const panel = chaosCues(event, FX_CHAOS_DICE_REEL_MS, p.env.intensity).find((cue) => cue.kind === "chaos");
    const cues: FxCue[] = panel === undefined ? [] : [{ ...panel, durationMs: FX_CHAOS_DICE_PANEL_MS }];
    cues.push(ringCue(p.D, "poison", centre, 0));
    return { cues, row: "replace" };
  }
  return null;
};

/** The gallery's Call to Chaos (Meditative Edition): its roll lands on one clause. */
export const jadeDieFixture: RecipeFixture = (_view, instanceId, player) => [{ type: "chaosRolled", player, instanceId, defId: JADE_DIE_ID, effects: [JADE_DIE_FIXTURE_EFFECT] }];
