// R3405001: Meditative #97 Jlockeed's Evil Blueprints (`meditative-097`, bespoke "blueprints", takeover
// "warRoom"), a Mythic Spell's signature in Tech's hologram look: its takeover is the war room's table under
// red lamps (takeoverArt.ts), and as the hold ends the torn blueprint's pieces lock into one at the board's
// centre in a glint of hologram light. The three buildings it offers are the caster's prompt alone, so no
// beat shows them; the finished blueprint folds into a card back that flies to the caster's hand pile, the
// pick's sheen the caster's alone. It keys on its own play, never on the building added, which no cue
// names (R202).

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe, RecipeFixture } from "../recipe.ts";

/** The sub-look it plots in: Tech's hologram, a lit table. */
const BLUEPRINT_PALETTE = "Tech/hologram";

/** The hologram glint as the pieces lock, at scale 1 (rule 9). */
const TUNING = { glint: { count: 22, power: 1 } } as const;

/** The pieces lock `tailMs` past the cast's entry; the folded back leaves at `ghostAt` of its entry. */
const TIMING = { tailMs: 400, ghostAt: 0.2 } as const;

export const blueprints: CardRecipe = (event, p, play) => {
  const look = paletteLook(BLUEPRINT_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const centre = tid(testid.board, FX_CENTER);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hold = p.entry.setPiece?.holdMs ?? 0;
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return { cues: [figureCue("blueprint", centre, figureTint(look), hold, p.D - hold + TIMING.tailMs), motifBurst(spec, centre, "area", hold, TUNING.glint)], row: "replace" };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
    const leave = frac(TIMING.ghostAt, p.D);
    const cues: FxCue[] = [{ kind: "ghost", from: centre, to: hand, delayMs: leave, durationMs: p.D - leave }];
    if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: hand, delayMs: leave, durationMs: p.D - leave });
    return { cues, row: "replace" };
  }
  return null;
};

/** The gallery's Evil Blueprints: the building picked, added to the caster's hand. */
export const blueprintsFixture: RecipeFixture = (_view, _instanceId, player) => [{ type: "addedToHand", player, instanceId: "fx-building", defId: "meditative-097-1" }];
