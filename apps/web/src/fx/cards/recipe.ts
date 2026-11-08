// R502, R670, R2784000: what a bespoke recipe (a `{ bespoke }` entry of a set's effects table) is built
// from. A card recipe claims events of its own play's resolution (cardFx.ts); an entrance replaces a
// marquee Unit's rarity entrance (entrances.ts). Each card's recipe is its own file beside its set's table.
// R3332000: an entrance is an entry's `entrance`, beside any entry of a Unit or a Field Spell, or alone.
// R3400001: Bob's "snowplowHeave", Tutorial Master's "chalkboardLesson" and the Training-Grounds Golem's "golemLashing".

import type { GameEvent, PlayerView } from "@jackioh/shared";

import type { AnimationEntry } from "../../game/animations.ts";
import { sideOf, testid } from "../../game/contract.ts";
import { FX_LEGENDARY_TRAUMA } from "../constants.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxPlay, FxPoint, FxProjectileCue } from "../types.ts";
import { tid } from "../build.ts";

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

/** Each entrance's shake at the slam, on top of nothing (rule 9). The Void's is the heaviest. */
export const ENTRANCE_TRAUMA = {
  voidCollapse: 0.8,
  bigBoom: 0.75,
  titanBloom: FX_LEGENDARY_TRAUMA,
  tyrantSigil: FX_LEGENDARY_TRAUMA,
  felinorAssembly: FX_LEGENDARY_TRAUMA,
  // R3400001: Meditative+'s and Training Grounds' signature Units, inside SIGNATURE_PERMANENT_MAX_TRAUMA.
  snowplowHeave: 0.75,
  chalkboardLesson: FX_LEGENDARY_TRAUMA,
  golemLashing: 0.75,
  // R3407000: Stinkyscape part two's (fx/cards/stinkyscape/), each within SIGNATURE_PERMANENT_MAX_TRAUMA.
  melchiorBoot: 0.3,
  balthasarLamp: 0.2,
  casperMirror: 0.25,
  freshJackVault: 0.6,
  ikwzkiDawn: 0.3,
  shockwaveRing: 0.7,
  resumeTube: 0.3,
  scornGate: 0.6,
  megabaseDock: 0.7,
  in33dh3Drop: 0.4,
  primeCrown: 0.4,
  blobPlop: 0.3,
  alteranDive: 0.5,
  golemCrash: 0.7,
  sanctumRise: 0.35,
} as const;

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
   * R3407000: how long the entry holds before its motion (a Mythic's takeover, a slam's anticipation;
   * `leadMs`), so an entrance that lands with its card (at FX_SLAM_AT of what follows) times its beats
   * after it; absent, none.
   */
  wait?: number;
};

export type Entrance = (p: EntrancePlan) => FxCue[];
