// Glitch (issue #170, SPEC §7): the blank token whose every word is corrupted. Presentation only
// (CLAUDE.md rule 7): a card the viewer may read whose id is GLITCH_DEF_ID wears these words instead
// of the catalog's, and a face-down card stays a back, since its id is the hidden sentinel.
//
// The corruption is drawn deterministically from a fixed source string and a fixed seed, so every
// client prints the same jumble and nothing calls Math.random during a render: each character of the
// source is kept, swapped for a glyph out of GLITCH_GLYPHS or followed by a combining mark, by a
// small linear congruential stream (its constants are Numerical Recipes'). A string's seed is the
// source's own characters folded into GLITCH_SEED, so the name and the text jumble differently.

import { GLITCH_DEF_ID } from "@jackioh/engine/config";

import type { CardDef } from "@jackioh/shared";

import type { CardInfo } from "../game/catalog.ts";

/** The words a Glitch would have, before corruption. */
export const GLITCH_SOURCE = {
  name: "Glitch",
  type: "Spell",
  text: "When played, the match is no longer the match. Nothing here is what it was.",
  cost: "0?",
} as const;

/** The fixed seed every client jumbles with. */
export const GLITCH_SEED = 170;

/** The glyphs a corrupted character becomes: block, box and symbol characters, never a brace (B3.4's `{key}`). */
export const GLITCH_GLYPHS = "█▓▒░▚▞▙▟◢◣◤◥¿¡§¤∆∑≠≈#%&@*?!/\\|<>~^ØÆÞßðµ¶";

/** Combining marks that stack on a kept character (the "zalgo" look), one at most per character. */
export const GLITCH_MARKS = "̶̴̸̡̢̧̨̛̀́̀́̕͘";

/** Out of GLITCH_ODDS_SCALE: below SWAP a character becomes a glyph, below SWAP + MARK it gains a mark. */
export const GLITCH_SWAP_IN = 45;
export const GLITCH_MARK_IN = 35;
export const GLITCH_ODDS_SCALE = 100;

/** The LCG: state' = (state × MULTIPLIER + INCREMENT) mod 2^32. */
const LCG_MULTIPLIER = 1664525;
const LCG_INCREMENT = 1013904223;
/** A character code is folded into the seed by this odd factor (the FNV prime). */
const FOLD_PRIME = 16777619;

/** Whether a card id is Glitch's. */
export function isGlitch(defId: string | null | undefined): boolean {
  return defId === GLITCH_DEF_ID;
}

/** `source` corrupted from `seed`: the same pair always gives the same string. */
export function corrupt(source: string, seed: number = GLITCH_SEED, swapIn: number = GLITCH_SWAP_IN): string {
  let state = seed >>> 0;
  for (const char of source) state = (Math.imul(state ^ (char.codePointAt(0) ?? 0), FOLD_PRIME) >>> 0);
  const next = (bound: number): number => {
    state = (Math.imul(state, LCG_MULTIPLIER) + LCG_INCREMENT) >>> 0;
    return state % bound;
  };
  const glyphs = [...GLITCH_GLYPHS];
  const marks = [...GLITCH_MARKS];
  let out = "";
  for (const char of source) {
    const roll = next(GLITCH_ODDS_SCALE);
    if (char === " ") out += roll < swapIn ? (glyphs[next(glyphs.length)] ?? char) : char;
    else if (roll < swapIn) out += glyphs[next(glyphs.length)] ?? char;
    else if (roll < swapIn + GLITCH_MARK_IN) out += char + (marks[next(marks.length)] ?? "");
    else out += char;
  }
  return out;
}

/** Everything a Glitch prints, corrupted once. */
export const GLITCH_WORDS = {
  name: corrupt(GLITCH_SOURCE.name),
  type: corrupt(GLITCH_SOURCE.type),
  text: corrupt(GLITCH_SOURCE.text),
  // The gem never shows a readable number: every character of it is a glyph.
  cost: corrupt(GLITCH_SOURCE.cost, GLITCH_SEED, GLITCH_ODDS_SCALE),
} as const;

/** Glitch's definition with its name and both faces' text corrupted. */
export function glitchDef(def: CardDef): CardDef {
  return {
    ...def,
    name: GLITCH_WORDS.name,
    base: { ...def.base, text: GLITCH_WORDS.text },
    radiant: { ...def.radiant, text: GLITCH_WORDS.text },
  };
}

/** A lookup's answer for Glitch: its name and text corrupted, on the info and on the def it carries. */
export function glitchInfo(info: CardInfo): CardInfo {
  return {
    ...info,
    name: GLITCH_WORDS.name,
    text: GLITCH_WORDS.text,
    ...(info.def === undefined ? {} : { def: glitchDef(info.def) }),
  };
}
