// R436: Call to Chaos names the effects it rolled, to both players.
//
// The engine reports the roll as `chaosRolled { player, instanceId, defId, effects }`, each effect as
// the clause its card prints (the engine's `label`: `CHAOS_EFFECTS` in subsystems/callToChaos.ts,
// `CHAOS_PLUS_EFFECTS` in callToChaosPlus.ts), in the order they resolve, on both seats (R97 redacts
// only the card, never the roll). This module is the client's whole reading of it:
// - `chaosRollOf` is the one adapter onto the event, so a rename is one line;
// - `CHAOS_EFFECT_NAMES` turns a clause into the short words a reel shows, per edition, in v0.2.0's
//   words: "Deck" (R373), "(3) Cost" as the noun and "cost (2) less" as the verb (R432);
// - `chaosCues` plans the reveal the effects layer draws: a slot-machine panel over the board, one
//   line per effect (stacked for three), each reel spinning through the edition's other effects and
//   landing on the one rolled, inside the entry (R200). The same names feed the showcase's live
//   region and its static banner under reduced motion (game/showcase/ChaosBanner.tsx).
//
// Nothing here is a rule: the roll is the event's, and the names are presentation (CLAUDE.md rule 7).
// An unknown clause still reads: one the table lacks is shown as the engine wrote it (R202 is not at
// stake: the roll is public).

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

/** Each edition's effects, the clause the engine names → the words a reel shows, in the card's order. */
export const CHAOS_EFFECT_NAMES: Readonly<Record<string, Readonly<Record<string, string>>>> = {
  [CHAOS_CORE]: {
    "Summon 3 random (3) Cost Units": "Summon 3 random (3) Cost Units",
    "Heal your hero 30": "Heal the caster's hero 30",
    "Draw your whole deck and gain 4 mana": "Draw the whole Deck, gain 4 mana",
    "Add 3 random cards to your hand, which cost (0)": "Add 3 random (0) Cost cards",
    "Make your hand Radiant": "The caster's hand becomes Radiant",
    "Summon 5 Radiant Rush Tokens": "Summon 5 Radiant Rush Tokens",
    "Cards in your hand and deck cost (2) less": "Hand and Deck cost (2) less",
    "Summon a Chaos Golem": "Summon a Chaos Golem",
    "Summon 5 random Field Spells or Traps into your backrow, Traps face-down": "Fill the backrow with Field Spells and Traps",
    "Cast a random Call to Chaos": "Cast a random Call to Chaos",
  },
  [CHAOS_CLASSIC_PLUS]: {
    "Add 5 random Fruits to your hand, which cost (0)": "Add 5 (0) Cost Fruits",
    "Add 3 random Books to your hand, which cost (0)": "Add 3 (0) Cost Books",
    "Destroy all enemy permanents": "Destroy all enemy permanents",
    "Add 3 random Classic cards to your hand, which cost (0)": "Add 3 (0) Cost Classic cards",
    "Upgrade every card in your hand and deck twice": "Upgrade hand and Deck twice",
    "Fuse a random card into each card in your deck, each keeping its cost": "Fuse a card into every Deck card",
    "Degrade every card on your opponent's field and in their hand three times": "Degrade the enemy's field and hand 3 times",
    "Summon a Classic Golem": "Summon a Classic Golem",
    "Replace your deck with random Call to Chaos cards, which cost (0)": "Replace the Deck with Calls to Chaos",
    "Cast a random Call to Chaos": "Cast a random Call to Chaos",
  },
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
  return spellings.size === 1 ? ([...spellings][0] ?? asWords(key)) : asWords(key);
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
