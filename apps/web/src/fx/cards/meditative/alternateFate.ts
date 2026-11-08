// R3405001: Meditative #26 Alternate Fate (`meditative-026`, entrance "crackedEarth", bespoke
// "alternateFate", a void aura on its zone), a Legendary Field Spell's signature. As it lands both sides'
// mana crystals crack dry as cracked earth spreads out from its zone under a violet dusk, and the slam
// splits the ground there. At its end of turn, lane by lane and the active side first, each empty backrow
// zone the engine fills cracks and a stone well shoves up out of it, blue light spilling over its rim
// onto its owner's side; a Mana Well has no entrance of its own, so each `summoned` into a backrow zone
// under its play or hook is this beat, the card's own landing excepted. It keys on its play or hook and
// the zone each summon names, never on a card's definition (R202); its beat keys on the event, not on the
// hook (the gallery plays it under its play, a game under its hook's marker).

import { animTestid, targetFor } from "../../../game/animations.ts";
import { sideOf, type Side } from "../../../game/contract.ts";
import { FX_FRACTURE_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look its dusk and cracks wear: Dark's violet. */
const DUSK_PALETTE = "Dark";

/** The look of the light its wells spill: Spirit's pale blue. */
const WELL_PALETTE = "Spirit";

/** Both seats, whose crystals both run dry. */
const SIDES: readonly Side[] = ["you", "opponent"];

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  split: { count: 22, power: 1.1 },
  crack: { count: 10, power: 0.7 },
  spill: { count: 14, power: 0.6 },
} as const;

/** The cracked earth lasts from its lead to its entry's end and `earthTailMs` more; a well stands its entry and `wellTailMs` more. */
const TIMING = { earthTailMs: 300, wellTailMs: 300 } as const;

/** Where each side's mana crystals crack, as a fraction of the time to the slam: yours first. */
const DRAIN_AT: Readonly<Record<Side, number>> = { you: 0.2, opponent: 0.35 };

export const crackedEarth: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const dusk = paletteLook(DUSK_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [figureCue("crackedEarth", at, figureTint(dusk), lead, D - lead + TIMING.earthTailMs, side)];
  for (const seat of SIDES) {
    const drain = lead + frac(DRAIN_AT[seat], slam - lead);
    cues.push({ kind: "fracture", at: tid(animTestid.mana(seat)), delayMs: drain, durationMs: D - drain + FX_FRACTURE_TAIL_MS });
  }
  cues.push(
    ringCue(D, dusk.particle, at, slam),
    { ...tunedBurst(i, dusk.particle, tid(tgt, FOOT), "ring", slam, TUNING.split), colors: dusk.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.crackedEarth, slam),
  );
  return cues;
};

export const alternateFate: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned" || event.row !== "backrow" || event.instanceId === play.instanceId) return null;
  const zone = targetFor(event, p.view);
  if (zone === null) return null;
  const dusk = paletteLook(DUSK_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const light = paletteLook(WELL_PALETTE) ?? FLAVOR_LOOKS.Spirit;
  const i = p.env.intensity;
  const at = tid(zone);
  return {
    cues: [
      { ...tunedBurst(i, "dust", tid(zone, FOOT), "area", 0, TUNING.crack), colors: dusk.colors },
      figureCue("stoneWell", at, figureTint(light), 0, p.D + TIMING.wellTailMs, { side: sideOf(p.view, event.player) }),
      { ...tunedBurst(i, light.particle, at, "area", 0, TUNING.spill), colors: light.colors },
    ],
    row: "replace",
  };
};

/** Two of its wells, the active side's first: one in its player's fourth backrow zone, one in the other side's second. */
export const alternateFateFixture: RecipeFixture = (view, _instanceId, player) => {
  const other = player === view.you.player ? view.opponent.player : view.you.player;
  return [
    { type: "summoned", player, instanceId: "fx-well-1", defId: "core-006", row: "backrow", lane: 4 },
    { type: "summoned", player: other, instanceId: "fx-well-2", defId: "core-006", row: "backrow", lane: 2 },
  ];
};
