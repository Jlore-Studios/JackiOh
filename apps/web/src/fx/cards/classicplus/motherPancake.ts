// R3399000: Classic+ The Mother Pancake (`classicplus-012`, entrance "pancakeStack", bespoke "pancakeFlip"), her signature (#3399):
// Entrance: pancakes drop into the zone one by one, each a flip of crumbs before it lands on the last (the
// staggered bursts before the slam), and at the slam the top one opens her face, a butter pat slides off in
// gold and flour puffs; her Taunt shows as a wide flour ring. Text: at the end of turn a small pancake flips
// off her top in a high arc to its owner's hand pile (a ghost, a back); the opponent's is the same back.
// Nothing moves from the concept. R202: a ghost is a back and no cue names a card; the owner alone gets the
// hand pile's sparkle (the opponent sees the back and the pile).

import { sideOf, testid } from "../../../game/contract.ts";
import { animTestid } from "../../../game/animations.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the stack and the flip throw, at intensity "normal" (rule 9). */
const TUNING = {
  drop: { count: 8, power: 0.6 },
  flour: { count: 22, power: 1 },
  butter: { count: 10, power: 0.7 },
  flipOff: { count: 8, power: 0.7 },
  land: { count: 10, power: 0.6 },
} as const;

/** The fractions of an entrance's D at which each pancake drops, all before the slam (R200). */
const DROP_AT = [0.12, 0.26, 0.4] as const;
/** The butter pat slides off just after the slam, and the pancake's ghost flight lasts this long. */
const BUTTER_AT = 0.7;
const FLIP_MS = 600;
/** The flip leaves her top at this fraction of the entry, and lands with the pile's sparkle at the next. */
const FLIP_AT = 0.15;
const FLIP_LAND_AT = 0.7;
/** The Taunt's ring rides on the foot of the zone. */
const PALETTE = FLAVOR_LOOKS.Snack;

export const pancakeStack: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const drops: FxCue[] = DROP_AT.flatMap((x) => [
    { ...tunedBurst(i, "crumb", at, "area", frac(x, D), TUNING.drop), colors: PALETTE.colors },
    ringCue(D, "dust", tid(tgt, FOOT), frac(x, D)),
  ]);
  return [
    ...drops,
    { ...tunedBurst(i, "dust", tid(tgt, FOOT), "area", slam, TUNING.flour), colors: PALETTE.colors },
    ringCue(D, "dust", at, slam),
    tunedBurst(i, "gold", at, "point", frac(BUTTER_AT, D), TUNING.butter),
    ...shakeCues(i, ENTRANCE_TRAUMA.pancakeStack, slam),
  ];
};

/** Her end of turn: the pancake that comes off her top flies to the hand pile of whoever she belongs to. */
export const pancakeFlip: CardRecipe = (event, p, play) => {
  if (event.type !== "addedToHand" || play.hook !== "trigger") return null;
  const side = sideOf(p.view, event.player);
  const hand = tid(animTestid.hand(side));
  const home = tid(testid.zone(sideOf(p.view, play.player), "units", 1));
  const from = ((): ReturnType<typeof tid> => {
    const zone = p.view[sideOf(p.view, play.player)].units.findIndex((unit) => unit?.instanceId === play.instanceId);
    return zone < 0 ? home : tid(testid.zone(sideOf(p.view, play.player), "units", zone + 1));
  })();
  const cues: FxCue[] = [
    { ...tunedBurst(p.env.intensity, "crumb", from, "point", frac(FLIP_AT, p.D), TUNING.flipOff), colors: PALETTE.colors },
    { kind: "ghost", from, to: hand, delayMs: frac(FLIP_AT, p.D), durationMs: FLIP_MS },
  ];
  if (event.player === p.view.viewer) cues.push(tunedBurst(p.env.intensity, "sparkle", hand, "area", frac(FLIP_LAND_AT, p.D), TUNING.land));
  return { cues, row: "keep" };
};

/** Her end of turn adds one pancake to its owner's hand. */
export function pancakeFlipFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-012", controller: player, hook: "endOfTurn" },
    { type: "addedToHand", player, instanceId: "fx-pancake", defId: "classicplus-012-1" },
  ];
}
