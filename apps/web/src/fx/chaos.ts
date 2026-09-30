// R436: Call to Chaos names the effects it rolled, to both players.
//
// The engine reports the roll as `chaosRolled { player, instanceId, defId, effects }`, the effects as
// keys in the order they resolve, on both seats (R97 redacts only the card, never the roll). This
// module is the client's whole reading of it:
// - `chaosRollOf` is the one adapter onto the event, so a rename is one line;
// - `CHAOS_EFFECT_NAMES` turns a key into the short words a player reads, per edition (the Core
//   Edition's keys are the engine's `CHAOS_EFFECTS` names, callToChaos.ts; the Classic+ Edition's,
//   docs/classic-sets.md B7 #73, are this table's, easy to rename), in v0.2.0's words: "Deck" (R373),
//   "(3) Cost" as the noun and "cost (2) less" as the verb (R432);
// - `chaosCues` plans the reveal the effects layer draws: a slot-machine panel over the board, one
//   line per effect (stacked for three), each reel spinning through the edition's other effects and
//   landing on the one rolled, inside the entry (R200). The same names feed the showcase's live
//   region and its static banner under reduced motion (game/showcase/ChaosBanner.tsx).
//
// Nothing here is a rule: the roll is the event's, and the names are presentation (CLAUDE.md rule 7).
// An unknown key still reads: a key the table lacks is shown as words (R202 is not at stake: the
// roll is public).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { frac, tunedBurst } from "./build.ts";
import {
  FX_BANNER_TAIL_MS,
  FX_CENTER,
  FX_CHAOS_DECOY_STEP,
  FX_CHAOS_LAND_AT,
  FX_CHAOS_LAND_LAST,
  FX_CHAOS_REEL_DECOYS,
  FX_CHAOS_STAGGER,
  FX_TEXT,
} from "./constants.ts";
import type { FxAnchor, FxChaosCue, FxChaosLine, FxCue } from "./types.ts";

/** The two editions (catalog ids). */
export const CHAOS_CORE = "core-095";
export const CHAOS_CLASSIC_PLUS = "classicplus-073";

/** Each edition's effects, key → the words a player reads, in the order its card text lists them. */
export const CHAOS_EFFECT_NAMES: Readonly<Record<string, Readonly<Record<string, string>>>> = {
  [CHAOS_CORE]: {
    units: "Summon 3 random (3) Cost Units",
    heal: "Heal the caster's hero 30",
    draw: "Draw the whole Deck, gain 4 mana",
    add: "Add 3 random cards costing (0)",
    radiant: "The caster's hand becomes Radiant",
    tokens: "Summon 5 Radiant Rush Tokens",
    discount: "Hand and Deck cost (2) less",
    golem: "Summon a Chaos Golem",
    backrow: "Fill the backrow with Field Spells and Traps",
    recast: "Cast a random Call to Chaos",
  },
  [CHAOS_CLASSIC_PLUS]: {
    fruits: "Add 5 Fruits costing (0)",
    books: "Add 3 Books costing (0)",
    destroy: "Destroy all enemy permanents",
    classics: "Add 3 Classic cards costing (0)",
    upgrade: "Upgrade hand and Deck twice",
    fuse: "Fuse a card into every Deck card",
    degrade: "Degrade the enemy's field and hand 3 times",
    golem: "Summon a Classic Golem",
    replace: "Replace the Deck with Calls to Chaos",
    recast: "Cast a random Call to Chaos",
  },
};

/**
 * Words for a key the editions spell differently, when the event does not say which edition rolled
 * (a card the viewer may not read, R97).
 */
export const CHAOS_GENERIC_NAMES: Readonly<Record<string, string>> = {
  golem: "Summon a Golem",
};

/** The roll as the client reads it. */
export type ChaosRoll = { player: PlayerId; instanceId: string; defId: string; effects: readonly string[] };

/** The one adapter onto `chaosRolled`. */
export function chaosRollOf(event: GameEvent): ChaosRoll | null {
  if (event.type !== "chaosRolled") return null;
  return { player: event.player, instanceId: event.instanceId, defId: event.defId, effects: [...event.effects] };
}

function own<T>(table: Readonly<Record<string, T>>, key: string): T | undefined {
  return Object.prototype.hasOwnProperty.call(table, key) ? table[key] : undefined;
}

/** A key no table has, as words: a key that already reads as words stays; one word is capitalised. */
function asWords(key: string): string {
  const trimmed = key.trim();
  if (trimmed === "") return "Unknown effect";
  if (/\s/.test(trimmed)) return trimmed;
  return trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
}

/** The words for one rolled effect of `defId`'s edition. */
export function chaosEffectName(defId: string, key: string): string {
  const edition = own(CHAOS_EFFECT_NAMES, defId);
  const named = edition === undefined ? undefined : own(edition, key);
  if (named !== undefined) return named;
  // An edition the viewer cannot name: a key every edition that has it spells the same way reads so.
  const spellings = new Set(
    Object.values(CHAOS_EFFECT_NAMES).flatMap((table) => {
      const name = own(table, key);
      return name === undefined ? [] : [name];
    }),
  );
  if (spellings.size === 1) return [...spellings][0] ?? asWords(key);
  return own(CHAOS_GENERIC_NAMES, key) ?? asWords(key);
}

/** The rolled effects' words, in the order they resolve. */
export function chaosNames(roll: ChaosRoll): string[] {
  return roll.effects.map((key) => chaosEffectName(roll.defId, key));
}

/** The names a reel spins through: the edition's, or every edition's for an edition the viewer cannot name. */
function reelPool(defId: string): string[] {
  const edition = own(CHAOS_EFFECT_NAMES, defId);
  const names = edition === undefined ? Object.values(CHAOS_EFFECT_NAMES).flatMap((table) => Object.values(table)) : Object.values(edition);
  return [...new Set(names)];
}

function gcd(a: number, b: number): number {
  return b === 0 ? a : gcd(b, a % b);
}

/** The walk's stride: FX_CHAOS_DECOY_STEP, or the next number that visits every name before repeating. */
function strideFor(length: number): number {
  let stride = FX_CHAOS_DECOY_STEP;
  while (gcd(stride, length) !== 1) stride += 1;
  return stride;
}

/**
 * One line's reel: FX_CHAOS_REEL_DECOYS of the edition's other effects, then the one rolled. The
 * decoys are the table's, in a fixed walk from the rolled effect's place, so a reel is the same every
 * time and says nothing the roll does not.
 */
export function chaosReel(defId: string, key: string, line: number): string[] {
  const landed = chaosEffectName(defId, key);
  const pool = reelPool(defId).filter((name) => name !== landed);
  if (pool.length === 0) return [landed];
  const table = own(CHAOS_EFFECT_NAMES, defId);
  const start = table === undefined ? 0 : Math.max(0, Object.keys(table).indexOf(key));
  const stride = strideFor(pool.length);
  const decoys: string[] = [];
  for (let k = 0; k < FX_CHAOS_REEL_DECOYS; k += 1) {
    const at = (start + line * stride + (k + 1) * stride) % pool.length;
    decoys.push(pool[at] ?? landed);
  }
  return [...decoys, landed];
}

/** When line `i` of `n` lands, as a fraction of the entry: the stagger shrinks so the last lands by FX_CHAOS_LAND_LAST. */
export function chaosLandAt(i: number, n: number): number {
  const stagger = n > 1 ? Math.min(FX_CHAOS_STAGGER, (FX_CHAOS_LAND_LAST - FX_CHAOS_LAND_AT) / (n - 1)) : 0;
  return FX_CHAOS_LAND_AT + i * stagger;
}

/** The prismatic sparks thrown as each line lands (rule 9). */
const CHAOS_TUNING = { landing: { count: 26, power: 1.2 } } as const;

const PANEL: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

/**
 * R436, R200: the reveal for one `chaosRolled` entry of duration D. Every line lands inside the
 * entry, and the panel is gone D + FX_BANNER_TAIL_MS after it starts (FX_BANNER_TAIL_MS ≤ T).
 */
export function chaosCues(event: GameEvent, D: number, intensity: number): FxCue[] {
  const roll = chaosRollOf(event);
  if (roll === null || roll.effects.length === 0) return [];
  const n = roll.effects.length;
  const lines: FxChaosLine[] = roll.effects.map((key, i) => ({
    text: chaosEffectName(roll.defId, key),
    reel: chaosReel(roll.defId, key, i),
    landMs: Math.min(D, frac(chaosLandAt(i, n), D)),
  }));
  const panel: FxChaosCue = { kind: "chaos", title: FX_TEXT.chaosRolled, lines, delayMs: 0, durationMs: D + FX_BANNER_TAIL_MS };
  return [panel, ...lines.map((line) => tunedBurst(intensity, "prismatic", PANEL, "point", line.landMs, CHAOS_TUNING.landing))];
}
