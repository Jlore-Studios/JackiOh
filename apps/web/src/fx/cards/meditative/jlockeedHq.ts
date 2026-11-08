// R3405001: Meditative #97.9 Jlockeed's Headquarters (`meditative-097-9`, a token printed Mythic, entrance
// "fortressRise", bespoke "jlockeedHq", takeover "fortress", embers on its tile), a Mythic Unit's signature.
// Its takeover (its "fortress" art, fx/takeover.ts) holds the entry first; after that hold the gunmetal
// fortress shrinks down into its zone, its missile racks folding, red beacons flashing at its corners, and
// it settles at the slam in a puff of exhaust. Its Activate fills its board: each Jlockeed Unit it summons
// (`summoned`, an empty zone each, left to right) draws its hangar doors rolling open and a contractor's
// truck streaming out to that zone, where it unloads in a puff of dust. Its `activated` comes before its
// Activate's marker opens its entry, so no beat keys on it; a summoned card with an entrance of its own
// plays that entrance instead. It keys on its own play or Activate and the event's type, never on a card's
// definition, and names no card (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look its fortress wears: Tech's rocket sub-look, its aura's. */
const HQ_PALETTE = "Tech/rocket";

/** A plain Unit with no entrance of its own, for its gallery fixture's truck to unload. */
const GALLERY_UNIT = "core-006";

/** Its two beacons, the top corners of its zone. */
const BEACONS: readonly FxPoint[] = [
  { x: 0.12, y: 0.08 },
  { x: 0.88, y: 0.08 },
];

/** Every particle count its signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  beacon: { count: 5, power: 0.4 },
  settle: { count: 18, power: 1 },
  unload: { count: 10, power: 0.7 },
} as const;

/**
 * As fractions of the motion after the lead: the beacons flash at each of `beacons`; the fortress lasts
 * the entry and `tailMs` more. At a summon, as fractions of the entry: the truck rolls out of the doors
 * at `rollAt` and reaches its zone at the entry's end; the doors last until `tailMs` past it.
 */
const TIMING = { beacons: [0.15, 0.4], tailMs: 300, rollAt: 0.25 } as const;

export const fortressRise: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(HQ_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const span = D - lead;
  const slam = lead + frac(FX_SLAM_AT, span);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [figureCue("fortress", at, figureTint(look), lead, D - lead + TIMING.tailMs, side)];
  for (const flash of TIMING.beacons) {
    for (const beacon of BEACONS) cues.push({ ...tunedBurst(i, "spark", tid(tgt, beacon), "point", lead + frac(flash, span), TUNING.beacon), colors: look.colors });
  }
  cues.push(
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, tid(tgt, FOOT), "area", slam, TUNING.settle), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.fortressRise, slam),
  );
  return cues;
};

export const jlockeedHq: CardRecipe = (event, p, play) => {
  // Each Unit its Activate fills its board with (an `activate` entry; its play's in the gallery), never its own landing.
  if (event.type !== "summoned" || event.player !== play.player || event.instanceId === play.instanceId) return null;
  if (play.hook !== undefined && play.hook !== "activate") return null;
  const look = paletteLook(HQ_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const i = p.env.intensity;
  const caster = sideOf(p.view, play.player);
  // Out of its hangar while the board shows it, else its controller's hero.
  const hq = tid(locateInstance(p.view, play.instanceId) ?? testid.hero(caster));
  const zone = testid.zone(caster, event.row, event.lane);
  const roll = frac(TIMING.rollAt, p.D);
  return {
    cues: [
      figureCue("convoy", hq, figureTint(look), 0, p.D + TIMING.tailMs, { side: caster }),
      { kind: "shot", shot: look.projectile, colors: look.colors, preset: look.particle, from: hq, to: tid(zone), delayMs: roll, flightMs: p.D - roll, density: i },
      { ...tunedBurst(i, "dust", tid(zone, FOOT), "point", p.D, TUNING.unload), colors: look.colors },
    ],
    row: "replace",
  };
};

/** R3405001: its Activate's first Unit, unloaded into its controller's first empty unit zone (the first, on a full board). */
export const jlockeedHqFixture: RecipeFixture = (view, _instanceId, player: PlayerId): GameEvent[] => {
  const side = player === view.viewer ? view.you : view.opponent;
  const open = side.units.findIndex((unit) => unit === null);
  return [{ type: "summoned", player, instanceId: "fx-jlockeed", defId: GALLERY_UNIT, row: "units", lane: open < 0 ? 1 : open + 1 }];
};
