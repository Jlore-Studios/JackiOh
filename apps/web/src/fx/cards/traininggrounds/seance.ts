// R3400001: Training Grounds #37 Seance (`traininggrounds-037`, bespoke "seance"), a Legendary Spell's
// signature: candle flames ring the chosen permanent's tile and lean in as the board dims to plum, embers
// flare there and halfway through a pale spirit rises from it; the copy it adds peels off that tile as a
// card back and drifts to the caster's hand pile. The tile is the first target its announce names, when the
// board shows that card face up in a zone; a face-down card has no id in the view, so its candles stand at
// the board's centre and its double stays a card back (R202). It never reads the added card.

import type { PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import type { FxAnchor, FxMemory, FxPlay } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the séance throws, at intensity "normal" (rule 9). */
const TUNING = {
  embers: { count: 16, power: 0.8 },
  spirit: { count: 20, power: 0.9 },
} as const;

/** The candles and the dusk last the entry and `tailMs` more; the spirit rises at `riseAt` of it. */
const TIMING = { tailMs: 400, riseAt: 0.5 } as const;

const HIDDEN_ID = "hidden";

/** R3400001, R202: the tile of the permanent `play`'s announce chose, when the board shows it face up; else the board's centre. */
export function seanceTile(memory: Pick<FxMemory, "recent">, view: PlayerView, play: FxPlay): FxAnchor {
  const announce = [...memory.recent()].reverse().find((event) => event.type === "cardAnnounced" && event.instanceId === play.instanceId);
  const target = announce?.type === "cardAnnounced" ? announce.targets[0] : undefined;
  const at = target === undefined || target === HIDDEN_ID ? null : locateInstance(view, target);
  return at !== null && at.startsWith("card-") ? tid(at) : tid(testid.board, FX_CENTER);
}

export const seance: CardRecipe = (event, p, play) => {
  const look = FLAVOR_LOOKS.Spirit;
  const tile = seanceTile(p.env.memory, p.view, play);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const i = p.env.intensity;
    const rise = frac(TIMING.riseAt, p.D);
    const tint = figureTint(look);
    return {
      cues: [
        figureCue("dusk", tid(testid.board), tint, 0, p.D + TIMING.tailMs),
        figureCue("candles", tile, tint, 0, p.D + TIMING.tailMs),
        tunedBurst(i, "ember", tile, "ring", 0, TUNING.embers),
        figureCue("spirit", tile, tint, rise, p.D - rise + TIMING.tailMs),
        { ...tunedBurst(i, "sparkle", tile, "area", rise, TUNING.spirit), colors: look.colors },
      ],
      row: "replace",
    };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    return { cues: [{ kind: "ghost", from: tile, to: tid(animTestid.hand(sideOf(p.view, play.player))), delayMs: 0, durationMs: p.D }], row: "replace" };
  }
  return null;
};
