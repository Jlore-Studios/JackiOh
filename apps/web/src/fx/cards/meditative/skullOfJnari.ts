// R3405001: Meditative #63 Skull of J'Nari (`meditative-063`, entrance "skullThud", bespoke "skullJnari",
// violet embers on its zone), a Legendary Field Spell's signature. As it lands a horned skull drops into its
// zone and thuds down at the slam in a puff of dust, rolls upright and chatters, violet fire kindling in its
// eye sockets. At its end of turn, for each Unit its hook summons from its owner's hand, it cuts itself
// off with a shriek (a ring off its zone), its eye-fire spills across into the zone the Unit lands in,
// flames lick up there and the Unit rises through the floor in a ring of violet sparks. The card leaving the hand flies no card back (a summoned Unit is no pile's), so the hand's part is
// the board's own; its voice lines are the audio's. It keys on its play or hook and the summon's owner and
// row, never on a card's definition, and names no card (R202); its beat keys on the event, not on the hook
// (the gallery plays it under its play, a game under its hook's marker).

import { targetFor } from "../../../game/animations.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, projectile, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { fieldTile } from "./introspection.ts";

/** The look its eye-fire wears: Dark's violet. */
const SKULL_PALETTE = "Dark";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  thud: { count: 16, power: 0.7 },
  kindle: { count: 10, power: 0.6 },
  rise: { count: 14, power: 0.9 },
} as const;

/**
 * The skull lasts from its lead to its entry's end and `skullTailMs` more, its kindled eye-fire from the
 * slam to `fireTailMs` past the end; a summon's eye-fire spills across in `spillFlight` of its entry and
 * burns on the zone `fireTailMs` past it.
 */
const TIMING = { skullTailMs: 300, fireTailMs: 250, spillFlight: 0.3 } as const;

/** Where its eye sockets are in its zone, as fx.css draws the skull. */
const EYES: FxPoint = { x: 0.5, y: 0.46 };

/** The Unit the gallery's Skull summons: a plain Common with no entrance of its own, into its player's first unit zone. */
const FIXTURE_UNIT = { defId: "core-008", lane: 1 } as const;

export const skullThud: Entrance = ({ D, tgt, intensity: i, leadMs }) => {
  const look = paletteLook(SKULL_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const tint = figureTint(look);
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  // Upright on both seats: no mirror for the other side.
  return [
    figureCue("hornedSkull", at, tint, lead, D - lead + TIMING.skullTailMs),
    tunedBurst(i, "dust", tid(tgt, FOOT), "area", slam, TUNING.thud),
    figureCue("eyeFire", at, tint, slam, D - slam + TIMING.fireTailMs),
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, tid(tgt, EYES), "point", slam, TUNING.kindle), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.skullThud, slam),
  ];
};

export const skullJnari: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned" || event.row !== "units" || event.player !== play.player || event.instanceId === play.instanceId) return null;
  const zone = targetFor(event, p.view);
  if (zone === null) return null;
  const look = paletteLook(SKULL_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const i = p.env.intensity;
  const tile = fieldTile(p.view, play);
  const at = tid(zone);
  const flight = frac(TIMING.spillFlight, p.D);
  const cues: FxCue[] = [
    // It cuts itself off with a shriek, and its eye-fire spills across.
    ringCue(p.D, look.particle, tile, 0),
    projectile(i, look.particle, tile, at, 0, flight),
    figureCue("eyeFire", at, figureTint(look), flight, p.D - flight + TIMING.fireTailMs),
    { ...tunedBurst(i, look.particle, tid(zone, FOOT), "ring", flight, TUNING.rise), colors: look.colors },
  ];
  return { cues, row: "replace" };
};

/** Its end of turn's summon: a plain Unit from its player's hand into their first unit zone. */
export const skullJnariFixture: RecipeFixture = (_view, _instanceId, player) => [
  { type: "summoned", player, instanceId: "fx-raised", defId: FIXTURE_UNIT.defId, row: "units", lane: FIXTURE_UNIT.lane },
];
