// R3399000: Classic+ Mommy Barker (`classicplus-013`, entrance "kitchenDoor", bespoke "pancakeTray"), her signature (#3399):
// Entrance: she bursts in through a swinging kitchen door (crumb flecks and a ring at the zone's foot,
// early), and at the slam barks once: a ring of flour dust shudders out across her whole side of the board.
// Text: on her death her apron flutters down (smoke at where she stood) and a covered pancake tray slides
// out of it to the owner's hand pile (a ghost, a back to everyone). A death's events come after her unit is
// gone, so the tray starts from her old zone when the view still reads her, else from the owner's graveyard.
// R202: no cue names a card; the hand pile and the graveyard are anchors.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid, HIDDEN_ID } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { hitAnchor } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the door and the tray throw, at intensity "normal" (rule 9). */
const TUNING = {
  door: { count: 10, power: 0.9 },
  bark: { count: 14, power: 1 },
  lane: { count: 6, power: 0.7 },
  apron: { count: 10, power: 0.5 },
  tray: { count: 10, power: 0.7 },
} as const;

/** The door bursts open early; the bark shudders out at the slam and across the lanes, a little after. */
const DOOR_AT = 0.2;
const LANE_STEP = 0.04;
/** The tray leaves the apron here, and the apron's flutter starts with it. */
const TRAY_AT = 0.1;
const TRAY_LAND_AT = 0.7;
const TRAY_MS = 600;
const PALETTE = FLAVOR_LOOKS.Snack;

export const kitchenDoor: Entrance = ({ D, tgt, intensity: i, view, landing }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const side = view !== undefined && landing !== undefined ? sideOf(view, landing.player) : undefined;
  const lanes: FxCue[] =
    side === undefined
      ? []
      : LANES.flatMap((lane, k) => {
          const name = testid.zone(side, "units", lane);
          const zone = tid(name);
          return name === tgt ? [] : [{ ...tunedBurst(i, "dust", zone, "area", Math.min(D, slam + frac(k * LANE_STEP, D)), TUNING.lane), colors: PALETTE.colors }];
        });
  return [
    { ...tunedBurst(i, "crumb", tid(tgt, FOOT), "area", frac(DOOR_AT, D), TUNING.door), colors: PALETTE.colors },
    ringCue(D, "dust", tid(tgt, FOOT), frac(DOOR_AT, D)),
    { ...tunedBurst(i, "dust", at, "area", slam, TUNING.bark), colors: PALETTE.colors },
    ringCue(D, "dust", at, slam),
    ...lanes,
    ...shakeCues(i, ENTRANCE_TRAUMA.kitchenDoor, slam),
  ];
};

/** Her death: the tray slides from her old zone (or the graveyard) to its owner's hand pile. */
export const pancakeTray: CardRecipe = (event, p, play) => {
  if (event.type !== "addedToHand" || play.hook !== "death") return null;
  const side = sideOf(p.view, play.player);
  const old: FxAnchor = (play.instanceId === HIDDEN_ID ? null : hitAnchor(p.view, play.instanceId)) ?? tid(animTestid.graveyard(side));
  const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
  const cues: FxCue[] = [
    { ...tunedBurst(p.env.intensity, "smoke", old, "area", frac(TRAY_AT, p.D), TUNING.apron), colors: PALETTE.colors },
    { kind: "ghost", from: old, to: hand, delayMs: frac(TRAY_AT, p.D), durationMs: TRAY_MS },
  ];
  if (event.player === p.view.viewer) cues.push(tunedBurst(p.env.intensity, "crumb", hand, "area", frac(TRAY_LAND_AT, p.D), TUNING.tray));
  return { cues, row: "keep" };
};

/** Her death adds a pancake to its owner's hand. */
export function pancakeTrayFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-013", controller: player, hook: "death" },
    { type: "addedToHand", player, instanceId: "fx-tray", defId: "classicplus-012-1" },
  ];
}
