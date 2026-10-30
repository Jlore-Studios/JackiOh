// R437: the marks a card carries, as the client draws them.
//
// A mark is an effect aimed at a card and waiting (#50 K-Pop Fanatic's steal at the start of its
// controller's next turn). The engine puts it on the card's view in both views, `CardView.marks`
// (`{ mark, color }`), and reports it coming and going with the `marked` event. This module is the
// one place the client reads either, so a rename in the view or the event is one line here:
// `marksOf` for the view, `markEventOf` for the event.
//
// Two tables turn a mark into something a player can read and see, and both are open to any card:
// - `MARK_PALETTES`: a color key ("purple") → the colors of the corruption aura and the brand
//   flourish, and the particle preset the effects layer throws in that color. An unknown key falls
//   back to DEFAULT_MARK_COLOR, so a new card's color never draws nothing.
// - `MARK_WORDS`: a mark ("steal") → its short name and the sentence the badge's tooltip and hidden
//   text say, so the mark never rests on color alone. An unknown mark gets the generic words.
//
// Nothing here decides a rule or reads anything but the view and the event (CLAUDE.md rule 7).

import type { CardMark, CardView, GameEvent } from "@jackioh/shared";

import type { FxPreset } from "../fx/types.ts";
import { BERSERK_WORDS, FROZEN_WORDS } from "./cardState.ts";

/** The colors one mark is drawn in: a bright rim, a pale core, a deep glow; and its particles. */
export type MarkPalette = {
  /** The crackling edge, the badge's ring and the motes. */
  rim: string;
  /** The hottest part of a mote and the sigil's center. */
  core: string;
  /** The vignette over the card and the halo around the badge. */
  glow: string;
  /** The effects layer's particles for this color (fx/presets.ts). */
  preset: FxPreset;
};

export const MARK_PALETTES = {
  purple: { rim: "#b46bff", core: "#f2e2ff", glow: "#7a2cff", preset: "arcane" },
  green: { rim: "#5fe08a", core: "#e4ffec", glow: "#1f9e4f", preset: "poison" },
  crimson: { rim: "#ff4a5f", core: "#ffe1e5", glow: "#b3122e", preset: "blood" },
  // B5 E35: the engine's Berserk mark (`BERSERK_MARK`) is "red", drawn as crimson.
  red: { rim: "#ff4a5f", core: "#ffe1e5", glow: "#b3122e", preset: "blood" },
  gold: { rim: "#ffd24a", core: "#fff7d6", glow: "#d19a00", preset: "gold" },
  cyan: { rim: "#4fe3ff", core: "#e0fbff", glow: "#0fa3c4", preset: "frost" },
  blue: { rim: "#6b94ff", core: "#e3eaff", glow: "#2349c9", preset: "frost" },
  orange: { rim: "#ff9a3d", core: "#fff1de", glow: "#d1570a", preset: "ember" },
  // Allure's (`ALLURE_MARK_COLOR`), pink: Jade Beauty's and K-Pop Fanatic's (R963, R1045000).
  pink: { rim: "#ff7ac8", core: "#ffe3f3", glow: "#c2187a", preset: "arcane" },
  // #2270: Brittle's last count (`BRITTLE_LAST_COLOR`), Allure's aura in gray; no mark is drawn in it.
  gray: { rim: "#b8bec8", core: "#f0f2f5", glow: "#5f6672", preset: "smoke" },
  // #2437: Temporary's last count in a hand (`TEMPORARY_LAST_COLOR`), Allure's aura in fuchsia; no mark is drawn in it.
  fuchsia: { rim: "#e879f9", core: "#fae8ff", glow: "#a21caf", preset: "arcane" },
} as const satisfies Readonly<Record<string, MarkPalette>>;

export type MarkColor = keyof typeof MARK_PALETTES;

/** What an unknown color key is drawn in: the corruption purple of the `steal` mark. */
export const DEFAULT_MARK_COLOR: MarkColor = "purple";

function isMarkColor(color: string): color is MarkColor {
  return Object.prototype.hasOwnProperty.call(MARK_PALETTES, color);
}

/** The color key a mark is actually drawn in: its own when the table has it, else the default. */
export function markColorOf(color: string): MarkColor {
  return isMarkColor(color) ? color : DEFAULT_MARK_COLOR;
}

export function paletteFor(color: string): MarkPalette {
  return MARK_PALETTES[markColorOf(color)];
}

/** A mark's words: a short name for the badge, and the sentence its tooltip and hidden text say. */
export type MarkWords = { name: string; text: string };

/** The engine's `ALLURE_MARK` (`effects/delay.rs`): the mark Allure puts on a card it will take (R963, R1045000). */
export const ALLURE_MARK = "allure";

/** The engine's `ALLURE_MARK_COLOR` (`effects/delay.rs`). */
export const ALLURE_MARK_COLOR: MarkColor = "pink";

/** The engine's `DUO_MARK` (`effects/delay.rs`): the mark Duo and Squad put on a card they will make Radiant (R1850001). */
export const DUO_MARK = "duo";

/** The engine's `DUO_MARK_COLOR` (`effects/delay.rs`): Allure's aura, recolored for Radiance. */
export const DUO_MARK_COLOR: MarkColor = "gold";

/** The engine's `END_OF_TURN_MARK` (`effects/delay.rs`): the mark a card a destroy at the end of this turn watches wears (R2321001). */
export const END_OF_TURN_MARK = "endOfTurn";

/** The engine's `END_OF_TURN_MARK_COLOR` (`effects/delay.rs`). */
export const END_OF_TURN_MARK_COLOR: MarkColor = "orange";

/** #2270: what a card on the field at its last Brittle count wears, the aura alone (no badge, no mark): Allure's, recolored gray. */
export const BRITTLE_LAST_COLOR: MarkColor = "gray";

/** #2437: what a hand card at its last Temporary count wears (R637, R2179001), the aura alone (no badge, no mark): Allure's, recolored fuchsia. */
export const TEMPORARY_LAST_COLOR: MarkColor = "fuchsia";

export const MARK_WORDS: Readonly<Record<string, MarkWords>> = {
  // Meditative #76 Do or Die: "At the start of your next turn, steal each marked card" (your: the caster's).
  steal: { name: "Steal", text: "Marked: stolen at the start of its marker's next turn" },
  // B5 E35: the mark a unit going Berserk is announced with (C+ #19.2, C+ #19.5).
  berserk: { name: "Berserk", text: BERSERK_WORDS },
  // R1174000: the mark a freeze lands and thaws with (`FROZEN_MARK`), in the frost the engine's "cyan" names.
  frozen: { name: "Frozen", text: FROZEN_WORDS },
  // Classic #20 The Power to Punish: the unit marked for death wears #50's aura, in red (R437).
  destroy: { name: "Destroy", text: "Marked: destroyed at the start of its marker's next turn" },
  // Allure (R963, R1045000): Meditative #39.5 Jade Beauty's every enemy Unit, #50 K-Pop Fanatic's one enemy permanent.
  allure: {
    name: "Allure",
    text: "Marked: joins its marker's side at the start of their next turn, or is destroyed with no room",
  },
  // Duo and Squad (R1850001): Stinkyscape #110 iKwzki's one permanent, its Radiant face's every permanent of yours.
  duo: { name: "Duo", text: "Marked: made Radiant at the start of its marker's next turn" },
  // R2321001: Training Grounds #15 In a hurry's Rush Tokens, while their destroy at the end of this turn waits.
  [END_OF_TURN_MARK]: { name: "End of turn", text: "Marked: destroyed at the end of this turn" },
};

/** The words for a mark the table does not know. */
export const UNKNOWN_MARK_WORDS: MarkWords = { name: "Mark", text: "Marked: an effect is waiting on this card" };

export function markWords(mark: string): MarkWords {
  return Object.prototype.hasOwnProperty.call(MARK_WORDS, mark) ? (MARK_WORDS[mark] ?? UNKNOWN_MARK_WORDS) : UNKNOWN_MARK_WORDS;
}

/** The sparkle the badge draws, so the mark is a shape as well as a color. */
export const MARK_GLYPH = "✦";

/** The drifting motes of one aura (marks.css places them by index). */
export const MARK_MOTES = 6;

function isMark(entry: unknown): entry is CardMark {
  if (typeof entry !== "object" || entry === null) return false;
  const record = entry as Record<string, unknown>;
  return typeof record.mark === "string" && typeof record.color === "string";
}

/**
 * The marks the view puts on a card: a `CardView`, or a face-down backrow entry, whose back carries
 * its mark to the player who cannot read it (R437, R33), read defensively as Backrow reads its cost.
 * None for a card without marks, or a malformed entry.
 */
export function marksOf(card: Pick<CardView, "marks"> | object | null | undefined): readonly CardMark[] {
  if (card === null || card === undefined || !("marks" in card)) return [];
  const marks: unknown = card.marks;
  if (!Array.isArray(marks)) return [];
  return marks.filter(isMark);
}

/** The `marked` event as the client reads it: a mark coming onto (`added`) or leaving a card. */
export type MarkChange = { instanceId: string; mark: string; color: string; added: boolean };

export function markEventOf(event: GameEvent): MarkChange | null {
  if (event.type !== "marked") return null;
  return { instanceId: event.instanceId, mark: event.mark, color: event.color, added: event.added };
}
