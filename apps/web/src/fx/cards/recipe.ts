// R502, R670, R2784000: what a bespoke recipe (a `{ bespoke }` entry of a set's effects table) is built
// from. A card recipe claims events of its own play's resolution (cardFx.ts); an entrance replaces a
// marquee Unit's rarity entrance (entrances.ts). Each card's recipe is its own file beside its set's table.
// R3332000: an entrance is an entry's `entrance`, beside any entry of a Unit or a Field Spell, or alone.
// R3400001: Bob's "snowplowHeave", Tutorial Master's "chalkboardLesson" and the Training-Grounds Golem's "golemLashing".
// R3389001: Classic's signatures' six new entrances, their shakes and the beats they time (rule 9).
// R3402001: Core's three signature entrances, Silly Silas's spotlight, Combo-Index's stadium and Heroic
// Power's pedestal, join them.
// R3405000: an entrance reads its entry's lead (`leadMs`, a Mythic's takeover hold) and starts after it;
// R3405001: Meditative's signature Units' and Field Spells' entrances, and each recipe's gallery fixture.
// R3395000: Stinkyscape 001 to 090's signature Units' and Field Spells' (fx/cards/stinkyscape/).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, type AnimationEntry } from "../../game/animations.ts";
import { sideOf, testid } from "../../game/contract.ts";
import { FX_LEGENDARY_TRAUMA } from "../constants.ts";
import type { FxAnchor, FxCue, FxPlanEnv, FxPlay, FxPoint, FxProjectileCue } from "../types.ts";
import { tid } from "../build.ts";
import { hitAnchor } from "../motifs/build.ts";

export type CardFxPlan = { entry: AnimationEntry; view: PlayerView; env: FxPlanEnv; D: number };

/**
 * A recipe's answer: its cues, and whether the event's own row recipe still plays (`keep`) or not.
 * R3395001: `moment: "replace"` also draws it in place of the event's system moment (systemMoments.ts),
 * which a recipe opts into beat by beat (Tylannt's dial for the shared quest moment); without it the
 * system moment draws beside the recipe, as before.
 */
export type CardFxResult = { cues: FxCue[]; row: "keep" | "replace"; moment?: "replace" };

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
  // R3395000: Stinkyscape 001 to 090's signature Units and Field Spells, a Legendary's at FX_LEGENDARY_TRAUMA and a
  // Mythic's (Tylannt, Rod of Ages, Paragon-X, Prototype-X2) a step louder, inside SIGNATURE_PERMANENT_MAX_TRAUMA.
  vaultStride: 0.75,
  heistDrop: FX_LEGENDARY_TRAUMA,
  rodRise: 0.75,
  lecternRise: FX_LEGENDARY_TRAUMA,
  spotlightKiss: FX_LEGENDARY_TRAUMA,
  labVault: FX_LEGENDARY_TRAUMA,
  emitterBloom: FX_LEGENDARY_TRAUMA,
  barrelSpin: FX_LEGENDARY_TRAUMA,
  breechLock: FX_LEGENDARY_TRAUMA,
  turretScan: FX_LEGENDARY_TRAUMA,
  intakeMaw: FX_LEGENDARY_TRAUMA,
  wraithGather: FX_LEGENDARY_TRAUMA,
  heraldHorn: FX_LEGENDARY_TRAUMA,
  paragonStreak: FX_LEGENDARY_TRAUMA,
  paragonDive: 0.75,
  flagshipDescent: FX_LEGENDARY_TRAUMA,
  testLurch: FX_LEGENDARY_TRAUMA,
  forgeBurst: 0.75,
  redCarpet: FX_LEGENDARY_TRAUMA,
  venomStep: FX_LEGENDARY_TRAUMA,
  // R3389001: Classic's signatures.
  seeingOrb: 0.4,
  sceptrePlant: 0.4,
  galeBurst: 0.45,
  silkDrop: 0.35,
  royalCloak: 0.4,
  abyssSurface: 0.5,
  // R3399000: Classic+'s signatures, in catalog order.
  pancakeStack: FX_LEGENDARY_TRAUMA,
  waffleIron: FX_LEGENDARY_TRAUMA,
  griddleFreeze: FX_LEGENDARY_TRAUMA,
  kitchenDoor: FX_LEGENDARY_TRAUMA,
  shieldPlant: FX_LEGENDARY_TRAUMA,
  vineNap: FX_LEGENDARY_TRAUMA,
  spotlightPose: FX_LEGENDARY_TRAUMA,
  staffShuffle: FX_LEGENDARY_TRAUMA,
  flexBurst: FX_LEGENDARY_TRAUMA,
  drumLeap: FX_LEGENDARY_TRAUMA,
  giftDesk: FX_LEGENDARY_TRAUMA,
  bannerPlant: FX_LEGENDARY_TRAUMA,
  greatBanner: FX_LEGENDARY_TRAUMA,
  briefcaseStride: FX_LEGENDARY_TRAUMA,
  templeStack: FX_LEGENDARY_TRAUMA,
  vineSwing: FX_LEGENDARY_TRAUMA,
  obeliskRise: FX_LEGENDARY_TRAUMA,
  // R3402001: Core's signature Unit's and Field Spells' entrances, inside SIGNATURE_PERMANENT_MAX_TRAUMA.
  silasSpotlight: 0.6,
  comboStadium: 0.5,
  heroicPedestal: FX_LEGENDARY_TRAUMA,
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
  /**
   * R3389000: when in the entry the card lands, after any hold (a Mythic's takeover, R2722001): at
   * FX_SLAM_AT of what is left. Absent, an entrance lands at FX_SLAM_AT of D, as it did before the hold.
   */
  landMs?: number;
  /** R3405000: the hold itself, before which an entrance that reads it starts nothing (`landMs` is its landing after it). */
  leadMs?: number;
};

export type Entrance = (p: EntrancePlan) => FxCue[];

/** R3405001: a recipe's gallery fixture: the events its play of `instanceId` resolves on `view`, as `player` sees them (motifs/gallery.ts). */
export type RecipeFixture = (view: PlayerView, instanceId: string, player: PlayerId) => GameEvent[];
