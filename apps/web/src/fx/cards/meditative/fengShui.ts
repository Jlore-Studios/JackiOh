// R3405001: Meditative #40 Feng Shui (`meditative-040`, entrance "fiveCompass", bespoke "fengShui", an
// orbiting aura on its zone), a Legendary Field Spell's signature. As it lands a five-coloured compass of
// water, wood, fire, metal and earth unfolds from its zone, turns once and clicks into alignment at the
// slam. Its Activate opens an auspicious blossom on its zone and rolls the Auspicious Rock it adds to its
// owner's hand pile as a tumbling card back; the Rock's face (and its Radiance) is its owner's to read, so
// the hand pile's sheen plays on their seat alone. Its judging of each face-up play (`fengShui`, and the
// Radiance or the hit that follows) runs inside the judged card's own play, never under this card's play
// or hooks, so no recipe of its own sees it: the board's fengShui moment draws it (systemMoments.ts). It
// keys on its play or hook and the card's owner, never on a card's definition, and draws card backs alone
// (R202); its beat keys on the event, not on the hook (the gallery plays it under its play, a game under
// its Activate's marker).

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { fieldTile } from "./introspection.ts";

/** The look its compass's rim and its blossom wear: Hustle's jade, the CN set's. */
const FENG_SHUI_PALETTE = "Hustle/jade";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  align: { count: 20, power: 1 },
  bloom: { count: 12, power: 0.6 },
} as const;

/** The compass lasts from its lead to its entry's end and `compassTailMs` more; the Rock rolls off its zone `rollAt` into its entry. */
const TIMING = { compassTailMs: 300, rollAt: 0.25 } as const;

export const fiveCompass: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(FENG_SHUI_PALETTE) ?? FLAVOR_LOOKS.Hustle;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  return [
    figureCue("baguaCompass", at, figureTint(look), lead, D - lead + TIMING.compassTailMs, side),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.align), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.fiveCompass, slam),
  ];
};

export const fengShui: CardRecipe = (event, p, play) => {
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  const look = paletteLook(FENG_SHUI_PALETTE) ?? FLAVOR_LOOKS.Hustle;
  const side = sideOf(p.view, play.player);
  const tile = fieldTile(p.view, play);
  const hand = tid(animTestid.hand(side));
  const roll = frac(TIMING.rollAt, p.D);
  const cues: FxCue[] = [
    figureCue("fengBlossom", tile, figureTint(look), 0, p.D, { side }),
    { ...tunedBurst(p.env.intensity, look.particle, tile, "area", 0, TUNING.bloom), colors: look.colors },
    { kind: "ghost", from: tile, to: hand, delayMs: roll, durationMs: p.D - roll, spin: true },
  ];
  // The Rock's face is its owner's to read.
  if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: hand, delayMs: roll, durationMs: p.D - roll });
  return { cues, row: "replace" };
};

/** Its Activate's Auspicious Rock, added to its owner's hand. */
export const fengShuiFixture: RecipeFixture = (_view, _instanceId, player) => [{ type: "addedToHand", player, instanceId: "fx-rock", defId: "meditative-039-1" }];
