// R3405001: Meditative #51 Devin Bot (`meditative-051`, entrance "cushionDrop", bespoke "devinBot", hologram
// scanlines on his tile), a Legendary Unit's signature. The little robot drops onto a meditation cushion in
// his zone and folds his legs at the slam in a soft puff, a power-down chime ringing out from him as his
// eye-lights dim. His Cry hums, and each 1/1 it fills his board with (`summoned`, an empty zone each, left
// to right, an entry each) pops out of him as a hologram wireframe beamed into its zone, where it
// solidifies with a small bounce. A summoned card with an entrance of its own plays that entrance instead.
// It keys on his play and the event's type, never on a card's definition, and names no card (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";
import { standingAt } from "./knowledgeBreaker.ts";

/** The look his holograms wear: Tech's hologram sub-look, his aura's. */
const DEVIN_PALETTE = "Tech/hologram";

/** A plain 1/1 Common Unit with no entrance of its own (Core #4 Gary the Gambler), for his gallery fixture's summon. */
const GALLERY_ONE_ONE = "core-004";

/** His eye-lights, high in his zone. */
const EYES: FxPoint = { x: 0.5, y: 0.35 };

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  puff: { count: 14, power: 0.6 },
  dim: { count: 6, power: 0.3 },
  bounce: { count: 8, power: 0.6 },
} as const;

/**
 * As fractions of the motion after the lead: his eye-lights dim at `dimAt`; the cushion lasts the entry
 * and `tailMs` more. At a summon, as fractions of the entry: the wireframe beams out of him until
 * `beamEnd` and solidifies with its bounce at `solidAt`, lasting until `tailMs` past the entry.
 */
const TIMING = { dimAt: 0.8, tailMs: 300, beamEnd: 0.45, solidAt: 0.6 } as const;

export const cushionDrop: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(DEVIN_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const span = D - lead;
  const slam = lead + frac(FX_SLAM_AT, span);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [
    figureCue("cushion", at, figureTint(look), lead, D - lead + TIMING.tailMs, side),
    { ...tunedBurst(i, "dust", tid(tgt, FOOT), "area", slam, TUNING.puff), colors: look.colors },
    ringCue(D, look.particle, at, slam),
    ...shakeCues(i, ENTRANCE_TRAUMA.cushionDrop, slam),
    // His eye-lights dim, their last glints fading.
    { ...tunedBurst(i, look.particle, tid(tgt, EYES), "point", lead + frac(TIMING.dimAt, span), TUNING.dim), colors: look.colors },
  ];
  return cues;
};

export const devinBot: CardRecipe = (event, p, play) => {
  // Each 1/1 his Cry fills his board with, never his own landing.
  if (event.type !== "summoned" || event.row !== "units" || event.player !== play.player || event.instanceId === play.instanceId) return null;
  if (play.hook !== undefined) return null;
  const look = paletteLook(DEVIN_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const i = p.env.intensity;
  const side = sideOf(p.view, play.player);
  const zone = testid.zone(side, event.row, event.lane);
  const from = standingAt(p, play);
  const beam = frac(TIMING.beamEnd, p.D);
  const solid = frac(TIMING.solidAt, p.D);
  return {
    cues: [
      // The hum, and the wireframe beamed out of him into its zone.
      ringCue(p.D, look.particle, from, 0),
      { kind: "shot", shot: look.projectile, colors: look.colors, preset: look.particle, from, to: tid(zone), delayMs: 0, flightMs: beam, density: i },
      figureCue("wireframe", tid(zone), figureTint(look), 0, p.D + TIMING.tailMs, { side }),
      { ...tunedBurst(i, look.particle, tid(zone, FOOT), "point", solid, TUNING.bounce), colors: look.colors },
    ],
    row: "replace",
  };
};

/** R3405001: his Cry's first 1/1, into `player`'s first empty unit zone (the first, on a full board). */
export const devinBotFixture: RecipeFixture = (view, _instanceId, player: PlayerId): GameEvent[] => {
  const side = player === view.viewer ? view.you : view.opponent;
  const open = side.units.findIndex((unit) => unit === null);
  return [{ type: "summoned", player, instanceId: "fx-devin-1-1", defId: GALLERY_ONE_ONE, row: "units", lane: open < 0 ? 1 : open + 1 }];
};
