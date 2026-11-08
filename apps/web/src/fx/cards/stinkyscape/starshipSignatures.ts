// R3334004: the entrances (R3332000) of two of the Starship's three Legendaries, each the card's signature,
// its own first beat beside the Starship's moment (fx/starship.ts, R2801000), which draws the launch that
// follows; the third, #176 Preemptive Strike's, is a card recipe (preemptiveStrike.ts).
// - The Dormant Starship (`stinkyscape-t-dormant-starship`, entrance "shipyardRollout") rolls out of its
//   shipyard: weld sparks flash over its hull as it comes down, then a ring, dust at its foot and a slam
//   shake as it settles, all in Tech's rocket look (#2792).
// - Starfleet Commander (`stinkyscape-137`, entrance "fleetCommand") lands and orders its side's Dormant
//   Starship up: a rocket bolt from its zone to the ship, and there a ring, a crown of gold and a shake.
//   With no Dormant Starship on its side (its Cry does nothing), the ring, the gold and the shake are its own.
// Each keys on its readable `summoned` alone (R202, entrances.ts), the Starship is read from the board's
// public `starship` status (`dormantStarshipOf`), and no cue names a card. Every cue starts inside its
// entry and ends within FX_MAX_TAIL_MS of it (R200), within a Legendary permanent's ceiling (R3332000).

import type { PlayerView } from "@jackioh/shared";

import { DORMANT_STARSHIP_ID } from "../../../cards/fused.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook } from "../../flavors.ts";
import { MOTIF_FOOT } from "../../motifs/build.ts";
import { dormantStarshipOf } from "../../starship.ts";
import type { FxAnchor, FxCue, FxPoint, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, type EntrancePlan } from "../recipe.ts";

/** Every particle count the two entrances throw, at intensity "normal" (rule 9). */
export const STARSHIP_ENTRANCE_TUNING = {
  weld: { count: 10, power: 1.2 },
  dust: { count: 26, power: 1.1 },
  crown: { count: 24, power: 1.2 },
} as const;

/**
 * R3334004: the Dormant Starship's weld sparks: when each flashes, as a fraction of its entry, all before
 * it settles at FX_SLAM_AT, and where on its zone (rule 9).
 */
export const WELDS: readonly { at: number; point: FxPoint }[] = [
  { at: 0.12, point: { x: 0.22, y: 0.62 } },
  { at: 0.28, point: { x: 0.78, y: 0.42 } },
  { at: 0.44, point: { x: 0.5, y: 0.28 } },
];

/** R3334004: Starfleet Commander's bolt leaves as it lands (FX_SLAM_AT) and reaches the ship at this fraction of its entry, inside it. */
export const FLEET_ORDER_LANDS_AT = 0.85;

/** Tech's rocket look (#2792), the Starship's. */
function rocketLook(): FlavorLook {
  return paletteLook("Tech/rocket") ?? FLAVOR_LOOKS.Tech;
}

/** R3334004: the Dormant Starship's entrance: weld sparks over its hull, then it settles with a ring, dust and a shake. */
export function shipyardRollout({ D, tgt, intensity: i }: EntrancePlan): FxCue[] {
  const look = rocketLook();
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    ...WELDS.map((weld): FxCue => ({ ...tunedBurst(i, "spark", tid(tgt, weld.point), "point", frac(weld.at, D), STARSHIP_ENTRANCE_TUNING.weld), colors: look.colors })),
    ringCue(D, look.particle, at, slam),
    tunedBurst(i, "dust", tid(tgt, MOTIF_FOOT), "area", slam, STARSHIP_ENTRANCE_TUNING.dust),
    ...shakeCues(i, ENTRANCE_TRAUMA.shipyardRollout, slam),
  ];
}

/**
 * R3334004: Starfleet Commander's entrance. With a Dormant Starship on its side's board (the view it lands
 * against), a rocket bolt leaves it as it lands and strikes the ship, where a ring, a crown of gold and a
 * shake answer; with none, the ring, the gold and the shake are its own, as it lands.
 */
export function fleetCommand({ D, tgt, intensity: i, view, landing }: EntrancePlan): FxCue[] {
  const look = rocketLook();
  const slam = frac(FX_SLAM_AT, D);
  const ship = view === undefined || landing === undefined ? null : dormantStarshipOf(view, sideOf(view, landing.player));
  const answer = (at: FxAnchor, delayMs: number): FxCue[] => [
    ringCue(D, look.particle, at, delayMs),
    tunedBurst(i, "gold", at, "ring", delayMs, STARSHIP_ENTRANCE_TUNING.crown),
    ...shakeCues(i, ENTRANCE_TRAUMA.fleetCommand, delayMs),
  ];
  if (ship === null) return answer(tid(tgt), slam);
  const arrive = Math.max(slam, frac(FLEET_ORDER_LANDS_AT, D));
  const bolt: FxShotCue = {
    kind: "shot",
    shot: { motion: "bolt", trail: look.particle },
    colors: look.colors,
    preset: look.particle,
    from: tid(tgt),
    to: tid(ship),
    delayMs: slam,
    flightMs: arrive - slam,
    density: i,
  };
  return [bolt, ...answer(tid(ship), arrive)];
}

/** R3334004: the pieces the gallery's Dormant Starship holds, and the lane of the viewer's unit row it stands in. */
export const GALLERY_STARSHIP_PIECES = ["stinkyscape-048", "stinkyscape-052", "stinkyscape-098"] as const;
export const GALLERY_STARSHIP_LANE = 5;

/**
 * R3334004: the gallery's board with a Dormant Starship holding `GALLERY_STARSHIP_PIECES` in
 * `GALLERY_STARSHIP_LANE` of the viewer's unit row, keeping that zone's instance id, so Starfleet
 * Commander has a ship to order up.
 */
export function fleetBoard(view: PlayerView): PlayerView {
  const units = view.you.units.map((unit, k) =>
    unit === null || k !== GALLERY_STARSHIP_LANE - 1
      ? unit
      : { ...unit, defId: DORMANT_STARSHIP_ID, attack: 0, slumber: {}, starship: { pieces: GALLERY_STARSHIP_PIECES.map((defId) => ({ defId })) }, canAct: false },
  );
  return { ...view, you: { ...view.you, units } };
}
