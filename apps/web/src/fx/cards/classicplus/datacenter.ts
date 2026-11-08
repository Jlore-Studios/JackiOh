// R3399000: Classic+ Claude's Datacenter (`classicplus-078`, entrance "obeliskRise", bespoke "printSlot"), its signature (#3399):
// Entrance: crystal server obelisks rise out of the zone's floor row by row (circuit sparks, bottom row to
// top, two obelisks a row), status lights blinking on in a cascade (a spark at each row), and at the slam a
// warm heat shimmer rolls outward (a sheen, an ember ring and a shake).
// Text: on activation a fan spins up (circuit sparks at the datacenter), a card prints out of a glowing slot
// and flies to the hand pile of the side that gets it (`addedToHand`: a ghost, a back to the opponent, whom
// the engine never shows the card), its cost set (`costChanged`: a spark at the hand pile, unless the view
// hides it).
// R202: no cue names a card; the printed card is a ghost back.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid, HIDDEN_ID } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the datacenter throws, at intensity "normal" (rule 9). */
const TUNING = {
  obelisk: { count: 6, power: 0.7 },
  light: { count: 4, power: 0.4 },
  slam: { count: 14, power: 0.9 },
  fan: { count: 8, power: 0.6 },
  slot: { count: 8, power: 0.7 },
  tick: { count: 8, power: 0.7 },
} as const;

const PALETTE = "Tech/hologram";
/** Entrance: each row's height in the zone, bottom first, the first row's start and the step between rows. */
const ROWS = [0.85, 0.55, 0.25] as const;
const OBELISK_XS = [0.3, 0.7] as const;
const ROW_AT = 0.1;
const ROW_STEP = 0.15;
const LIGHT_LAG = 0.05;
/** Activation. */
const FAN_AT = 0.05;
const PRINT_AT = 0.15;
const PRINT_LAND_AT = 0.4;
const TICK_AT = 0.45;

export const obeliskRise: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Tech;
  const cues: FxCue[] = [];
  ROWS.forEach((y, k) => {
    const rowAt = frac(ROW_AT + k * ROW_STEP, D);
    for (const x of OBELISK_XS) cues.push({ ...tunedBurst(i, "circuit", tid(tgt, { x, y }), "point", rowAt, TUNING.obelisk), colors: look.colors });
    cues.push({ ...tunedBurst(i, "spark", tid(tgt, { x: 0.5, y }), "area", Math.min(slam, rowAt + frac(LIGHT_LAG, D)), TUNING.light), colors: look.colors });
  });
  cues.push(
    ringCue(D, "ember", at, slam),
    { kind: "sheen", at, delayMs: slam, durationMs: D - slam },
    { ...tunedBurst(i, "ember", at, "area", slam, TUNING.slam), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.obeliskRise, slam),
  );
  return cues;
};

/** Classic+ #78's Activate: the fan, the slot and the printed card. */
export const printSlot: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate") return null;
  const i = p.env.intensity;
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Tech;
  const mine = sideOf(p.view, play.player);
  const slot: FxAnchor = (play.instanceId === HIDDEN_ID ? null : hitAnchor(p.view, play.instanceId)) ?? tid(animTestid.library(mine));
  if (event.type === "addedToHand") {
    const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
    const cues: FxCue[] = [
      { ...tunedBurst(i, "circuit", slot, "area", frac(FAN_AT, p.D), TUNING.fan), colors: look.colors },
      { ...tunedBurst(i, "spark", slot, "point", frac(PRINT_AT, p.D), TUNING.slot), colors: look.colors },
      { kind: "ghost", from: slot, to: hand, delayMs: frac(PRINT_AT, p.D), durationMs: frac(PRINT_LAND_AT - PRINT_AT, p.D) },
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "costChanged") {
    if (event.hiddenFrom?.includes(p.view.viewer) === true) return null;
    const hand = tid(animTestid.hand(mine));
    return { cues: [{ ...tunedBurst(i, "spark", hand, "point", frac(TICK_AT, p.D), TUNING.tick), colors: look.colors }], row: "keep" };
  }
  return null;
};

/** The gallery's Datacenter: it prints one card for its player, its cost set. */
export function printSlotFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-078", controller: player, hook: "activate" },
    { type: "addedToHand", player, instanceId: "fx-print", defId: "classicplus-t-ai-01" },
    { type: "costChanged", instanceId: "fx-print", cost: 0 },
  ];
}
