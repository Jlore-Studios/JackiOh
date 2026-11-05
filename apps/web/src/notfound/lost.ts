// #3119, R3119000: what the 404 panel draws, worked out from the path alone. Presentation only
// (CLAUDE.md rule 7): no rule reads it and nothing reaches the engine or the server. Every pick is a
// hash of the path (cards/art/hash.ts), never `Math.random`, so one wrong link always shows the same.

import { CARD_HOOK_NAMES } from "../audio/constants.ts";
import type { CardHook } from "../audio/types.ts";
import { CARD_AUDIO, lineFor } from "../audio/voiceData.ts";
import { hashId } from "../cards/art/hash.ts";
import { corrupt, GLITCH_ODDS_SCALE } from "../cards/glitch.ts";
import { paths } from "../net/navigate.ts";

/** The catalog's cards that suit a page nobody can find, each with a voice line of its own. */
export const LOST_CARDS = [
  "classicplus-006", // Wrong-House Attacker
  "core-040", // Echoes of the Forgotten
  "classic-050", // Voidwalker
  "classic-089", // Paul Allen's Ghost
  "core-100", // Ceaseless Void
  "meditative-097-1", // Empty Plot
  "core-051-1", // KY's Empty Notebook
] as const;

export type LostCardId = (typeof LOST_CARDS)[number];

/** The lost card a path shows: the same path, the same card. */
export function lostCardFor(path: string): LostCardId {
  return LOST_CARDS[hashId(path) % LOST_CARDS.length] ?? LOST_CARDS[0];
}

/** The line a lost card says when tapped: its first hook with a line, in CARD_HOOKS's order. */
export function lostCardLine(defId: string): CardHook | null {
  return CARD_HOOK_NAMES.find((hook) => lineFor(CARD_AUDIO, defId, hook) !== null) ?? null;
}

/** The three face-down cards' destinations, in the order they are dealt. Their words go through `t()` where drawn. */
export const WAYS_OUT = [
  { key: "play", to: paths.play, label: "Play", detail: "Queue for a match." },
  { key: "practice", to: paths.practice, label: "Practice", detail: "Play the AI." },
  { key: "almanac", to: paths.almanac, label: "Almanac", detail: "Read every card." },
] as const;

export type WayOut = (typeof WAYS_OUT)[number];

/** How long the glitched path takes to settle by one more character. */
export const GLITCH_SETTLE_STEP_MS = 45;

/**
 * The path as the panel prints it once `settled` characters have settled: those as they are, the
 * rest every one a Glitch glyph (cards/glitch.ts), seeded by the path so it jumbles the same each time.
 */
export function glitchedPath(path: string, settled: number): string {
  const chars = [...path];
  const kept = Math.max(0, Math.min(settled, chars.length));
  const rest = chars.slice(kept).join("");
  return chars.slice(0, kept).join("") + (rest === "" ? "" : corrupt(rest, hashId(path), GLITCH_ODDS_SCALE));
}
