// R502, R670, R2784000: what a bespoke recipe (a `{ bespoke }` entry of a set's effects table) is built
// from. A card recipe claims events of its own play's resolution (cardFx.ts); an entrance replaces a
// marquee Unit's rarity entrance (entrances.ts). Each card's recipe is its own file beside its set's table.
// R3332000: an entrance is an entry's `entrance`, beside any entry of a Unit or a Field Spell, or alone.
// R3400001: Bob's "snowplowHeave", Tutorial Master's "chalkboardLesson" and the Training-Grounds Golem's "golemLashing".
// R3405000: an entrance reads its entry's lead (`leadMs`, a Mythic's takeover hold) and starts after it;
// R3405001: Meditative's signature Units' and Field Spells' entrances, and each recipe's gallery fixture.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

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
  // R3405001: Meditative's signature Units' and Field Spells', inside SIGNATURE_PERMANENT_MAX_TRAUMA.
  poolBreath: 0.3,
  cushionDrop: 0.3,
  lanternLight: 0.3,
  floodlights: 0.35,
  chalkHand: 0.35,
  coinFlip: 0.35,
  fiveCompass: 0.4,
  keyRing: 0.4,
  virusBoot: 0.4,
  dojoBoot: 0.4,
  chalkMask: 0.45,
  treeSprout: 0.45,
  crackedEarth: 0.5,
  scrollScholar: 0.5,
  studyCurve: 0.5,
  skullThud: 0.55,
  caneRise: 0.6,
  totemStack: 0.6,
  prisonBars: 0.7,
  jadeBoulder: 0.75,
  bronzeCauldron: 0.75,
  golemLock: 0.75,
  maskSplit: 0.8,
  pavilionUnfurl: 0.8,
  fortressRise: 0.8,
} as const;

/** Where Big Max's fuse sparks, as a fraction of the entry, before the slam at FX_SLAM_AT. */
export const ENTRANCE_FUSE_AT = 0.25;

/** The foot of the unit's zone, where dust and smoke rise (cues.ts's FOOT). */
export const FOOT: FxPoint = { x: 0.5, y: 1 };

export type EntranceKey = keyof typeof ENTRANCE_TRAUMA;

/**
 * What an entrance plans against: the entry's duration, the zone's testid and the intensity. R3332001:
 * the board's view and the readable `summoned` it lands with, for one that reads the board (Felinor
 * Fiender's); the planner passes both, and a recipe that reads neither ignores them. R3405000: `leadMs`,
 * the wait inside the entry before its motion starts (game/animations.ts `leadMs`: a Mythic's takeover
 * hold), which an entrance that reads it starts after; one that does not ignores it.
 */
export type EntrancePlan = {
  D: number;
  tgt: string;
  intensity: number;
  view?: PlayerView;
  landing?: Extract<GameEvent, { type: "summoned" }>;
  leadMs?: number;
};

export type Entrance = (p: EntrancePlan) => FxCue[];

/** R3405001: a recipe's gallery fixture: the events its play of `instanceId` resolves on `view`, as `player` sees them (motifs/gallery.ts). */
export type RecipeFixture = (view: PlayerView, instanceId: string, player: PlayerId) => GameEvent[];
