// R502, R670, R2784000: what a bespoke recipe (a `{ bespoke }` entry of a set's effects table) is built
// from. A card recipe claims events of its own play's resolution (cardFx.ts); an entrance replaces a
// marquee Unit's rarity entrance (entrances.ts). Each card's recipe is its own file beside its set's table.
// R3332000: an entrance is an entry's `entrance`, beside any entry of a Unit or a Field Spell, or alone.
// R3400001: Bob's "snowplowHeave", Tutorial Master's "chalkboardLesson" and the Training-Grounds Golem's "golemLashing".
// R3389001: Classic's signatures' six new entrances, their shakes and the beats they time (rule 9).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, type AnimationEntry } from "../../game/animations.ts";
import { sideOf, testid } from "../../game/contract.ts";
import { FX_LEGENDARY_TRAUMA } from "../constants.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxPlay, FxPoint, FxProjectileCue } from "../types.ts";
import { tid } from "../build.ts";
import { hitAnchor } from "../motifs/build.ts";

export type CardFxPlan = { entry: AnimationEntry; view: PlayerView; env: FxPlanEnv; D: number };

/** A recipe's answer: its cues, and whether the event's own row recipe still plays (`keep`) or not. */
export type CardFxResult = { cues: FxCue[]; row: "keep" | "replace" };

export type CardRecipe = (event: GameEvent, p: CardFxPlan, play: FxPlay) => CardFxResult | null;

export function heroOf(view: PlayerView, play: FxPlay): FxAnchor {
  return tid(testid.hero(sideOf(view, play.player)));
}

export function projectile(intensity: number, preset: FxProjectileCue["preset"], from: FxAnchor, to: FxAnchor, delayMs: number, flightMs: number): FxProjectileCue {
  return { kind: "projectile", preset, from, to, delayMs, flightMs, density: intensity };
}

/** R3389001: whether the entry resolving now is `play`'s own (its play or its hook), not a cast inside it or another card's. */
export function ownWindow(p: CardFxPlan, play: FxPlay): boolean {
  const now = p.env.memory.resolving();
  return now !== undefined && now.instanceId === play.instanceId && now.player === play.player;
}

/**
 * R3389001, R202: where the card `instanceId` stands: as the view shows it, else in the zone its newest
 * readable landing or firing the memory holds names (the board shows the view before a burst until the
 * burst ends, so a card that just landed may not be in it yet); else null.
 */
export function cardAnchor(p: CardFxPlan, instanceId: string): FxAnchor | null {
  if (instanceId === HIDDEN_ID) return null;
  const shown = hitAnchor(p.view, instanceId);
  if (shown !== null) return shown;
  const recent = p.env.memory.recent();
  for (let k = recent.length - 1; k >= 0; k -= 1) {
    const event = recent[k];
    if (event?.type === "summoned" && event.instanceId === instanceId) return tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane));
    if (event?.type === "trapFired" && event.instanceId === instanceId) return tid(testid.zone(sideOf(p.view, event.controller), event.row, event.lane));
  }
  return null;
}

/** R3389001: the `k`-th of a run's steps, SIGNATURE_STEP_AT of the entry apart from `from`, never past its end. */
export function stepMs(D: number, from: number, k: number): number {
  return Math.min(D, from + Math.round(k * SIGNATURE_STEP_AT * D));
}

/** Each entrance's shake at the slam, on top of nothing (rule 9). The Void's is the heaviest. */
export const ENTRANCE_TRAUMA = {
  voidCollapse: 0.8,
  bigBoom: 0.75,
  titanBloom: FX_LEGENDARY_TRAUMA,
  tyrantSigil: FX_LEGENDARY_TRAUMA,
  felinorAssembly: FX_LEGENDARY_TRAUMA,
  // R3334004: the Dormant Starship rolls out of its shipyard quieter than a Legendary lands (its launch is
  // its loud moment); Starfleet Commander's order lands at the Legendary's weight.
  shipyardRollout: 0.5,
  fleetCommand: FX_LEGENDARY_TRAUMA,
  // R3400001: Meditative+'s and Training Grounds' signature Units, inside SIGNATURE_PERMANENT_MAX_TRAUMA.
  snowplowHeave: 0.75,
  chalkboardLesson: FX_LEGENDARY_TRAUMA,
  golemLashing: 0.75,
  // R3389001: Classic's signatures.
  seeingOrb: 0.4,
  sceptrePlant: 0.4,
  galeBurst: 0.45,
  silkDrop: 0.35,
  royalCloak: 0.4,
  abyssSurface: 0.5,
} as const;

/** R3389001: the shake of each Classic signature recipe that shakes the board, on top of nothing (rule 9). */
export const SIGNATURE_TRAUMA = { taxSeizure: 0.4, graveRise: 0.6, craterLock: 0.5 } as const;

/** R3389001: where a signature's second beat starts, as a fraction of its entry: after the first beat's gather. */
export const SIGNATURE_BEAT_AT = 0.35;

/** R3389001: how far a signature's run steps on per card it moves (Second Wind's piles, Spell Tyrant's ghosts), a fraction of the entry. */
export const SIGNATURE_STEP_AT = 0.08;

/** R3389001: the most cards a signature's run draws moving in one entry (a whole deck blown away draws this many backs). */
export const SIGNATURE_RUN_MAX = 4;

/** Where Big Max's fuse sparks, as a fraction of the entry, before the slam at FX_SLAM_AT. */
export const ENTRANCE_FUSE_AT = 0.25;

/** The foot of the unit's zone, where dust and smoke rise (cues.ts's FOOT). */
export const FOOT: FxPoint = { x: 0.5, y: 1 };

export type EntranceKey = keyof typeof ENTRANCE_TRAUMA;

/**
 * What an entrance plans against: the entry's duration, the zone's testid and the intensity. R3332001:
 * the board's view and the readable `summoned` it lands with, for one that reads the board (Felinor
 * Fiender's); the planner passes both, and a recipe that reads neither ignores them.
 */
export type EntrancePlan = {
  D: number;
  tgt: string;
  intensity: number;
  view?: PlayerView;
  landing?: Extract<GameEvent, { type: "summoned" }>;
  /**
   * R3389000: when in the entry the card lands, after any hold (a Mythic's takeover, R2722001): at
   * FX_SLAM_AT of what is left. Absent, an entrance lands at FX_SLAM_AT of D, as it did before the hold.
   */
  landMs?: number;
};

export type Entrance = (p: EntrancePlan) => FxCue[];
