// R3332001: Felinor Fiender (`core-092`, entrance "felinorAssembly"), the first signature (R3332000). It
// has the stats of every Felinor its controller has, so it assembles from them: each Felinor the side
// shows face up sends a streak of cat light into the zone it lands in, in lane order, all arriving at the
// slam, where the light gathers in a ring and a crown of gold. A Felinor buried under a Stack counts toward
// its stats but lies face down (§3.2), so it sends nothing (R202); a Felinor it stacks onto streaks up from
// its own foot. No cue names a card.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_FELINOR_STREAK_AT, FX_FELINOR_STREAK_STEP, FX_FELINOR_STREAKS_MAX, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type Entrance } from "../recipe.ts";

/** Every particle count the assembly throws, at intensity "normal" (rule 9). */
const TUNING = {
  assemble: { count: 30, power: 1.1 },
  crown: { count: 24, power: 1.2 },
} as const;

/** The tag every Felinor carries (§8). */
const FELINOR_TAG = "Felinor";

/** The look the streaks wear: Beast's Felinor sub-look. */
const FELINOR_PALETTE = "Beast/felinor";

type Summoned = Extract<GameEvent, { type: "summoned" }>;

/**
 * R3332001, R202: the unit zones, in lane order, of every Felinor on the landing's side that the view
 * shows face up, the landing card itself left out: the top of each zone, read by its public tags.
 */
export function felinorZones(view: PlayerView, landing: Summoned): string[] {
  const side = sideOf(view, landing.player);
  return view[side].units.flatMap((unit, k) => {
    if (unit === null || unit.defId === HIDDEN_ID || unit.instanceId === landing.instanceId) return [];
    const tags = unit.tags ?? (CATALOG[unit.defId] ?? view.defs?.[unit.defId])?.tags ?? [];
    return tags.includes(FELINOR_TAG) ? [testid.zone(side, "units", k + 1)] : [];
  });
}

export const felinorAssembly: Entrance = ({ D, tgt, intensity: i, view, landing }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const look = paletteLook(FELINOR_PALETTE) ?? FLAVOR_LOOKS.Beast;
  const zones = view === undefined || landing === undefined ? [] : felinorZones(view, landing).slice(0, FX_FELINOR_STREAKS_MAX);
  const streaks = zones.map((zone, n): FxShotCue => {
    const delayMs = frac(FX_FELINOR_STREAK_AT + n * FX_FELINOR_STREAK_STEP, D);
    return {
      kind: "shot",
      shot: { motion: "bolt", trail: look.particle },
      colors: look.colors,
      preset: look.particle,
      from: zone === tgt ? tid(zone, FOOT) : tid(zone),
      to: at,
      delayMs,
      flightMs: Math.max(0, slam - delayMs),
      density: i,
    };
  });
  const cues: FxCue[] = [
    ...streaks,
    ringCue(D, look.particle, at, slam),
    { ...tunedBurst(i, look.particle, at, "area", slam, TUNING.assemble), colors: look.colors },
    tunedBurst(i, "gold", at, "ring", slam, TUNING.crown),
    ...shakeCues(i, ENTRANCE_TRAUMA.felinorAssembly, slam),
  ];
  return cues;
};

/** R3332001: the Felinors the gallery's board shows, by its lanes, so Felinor Fiender has some to assemble from. */
export const GALLERY_FELINORS = ["core-t-felinor", "core-043", "core-012"] as const;
export const GALLERY_FELINOR_LANES = [1, 2, 5] as const;

/**
 * R3332001: the gallery's board with a Felinor in each of `GALLERY_FELINOR_LANES` of the viewer's unit
 * row, each keeping its instance id; lane 4's Stack pile stays, its buried cards face down.
 */
export function felinorBoard(view: PlayerView): PlayerView {
  const units = view.you.units.map((unit, k) => {
    const at = (GALLERY_FELINOR_LANES as readonly number[]).indexOf(k + 1);
    const defId = GALLERY_FELINORS[at];
    return unit === null || defId === undefined ? unit : { ...unit, defId };
  });
  return { ...view, you: { ...view.you, units } };
}
