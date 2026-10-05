// The Glitch token on the client (SPEC §7, R662): a hidden card whose face is blank and whose every
// text, its name, its rules text and its flavour line, is drawn as a jumbled mess. The jumble is a
// pure function of the text it stands for, seeded from it, so every client and every reload draws the
// same one, and the true words never reach the screen. Presentation only (CLAUDE.md rule 7): the
// catalog prints Glitch's real name and text, and nothing here reads or decides a rule.

import { GLITCH_DEF_ID } from "@jackioh/engine/config";

import { hashId, seededRandom } from "./art/hash.ts";

/** The glyphs a corrupted character may be drawn as: blocks, shades and the debris of broken text. */
const GLITCH_GLYPHS: readonly string[] = [..."█▓▒░▌▐▀▄▖▗▘▝▚▞◤◥◣◢¤§¶ǂ#%&@$?!/\\|<>~^_=*"];
/** Combining marks stacked on a corrupted character, as broken text renders. */
const GLITCH_MARKS: readonly string[] = [
  "\u0334", "\u0335", "\u0336", "\u0337", "\u0338", "\u0315", "\u031b", "\u0321", "\u0322", "\u0327",
  "\u0328", "\u0340", "\u0341", "\u0345", "\u0358",
];
/** How often a character is replaced by a glyph, kept with its case flipped, or kept as it is. */
const GLYPH_SHARE = 0.4;
const FLIP_SHARE = 0.25;
/** The most combining marks one character carries. */
const MARKS_MAX = 2;
/** Mixed into the seed so a corrupted text is not the art's hash of the same words. */
const GLITCH_SALT = "glitch";

/** R662: whether a face, a log line or a prompt option shows Glitch, and must be drawn corrupted. */
export function isGlitch(defId: string | null | undefined): boolean {
  return defId === GLITCH_DEF_ID;
}

function pick<T>(list: readonly T[], random: () => number): T {
  const at = Math.min(list.length - 1, Math.floor(random() * list.length));
  return list[at] as T;
}

function flipCase(char: string): string {
  const upper = char.toUpperCase();
  return upper === char ? char.toLowerCase() : upper;
}

/** R662: a card's name as the client shows it: Glitch's corrupted, every other card's as printed. */
export function displayName(def: { readonly id: string; readonly name: string }): string {
  return isGlitch(def.id) ? corruptedText(def.name) : def.name;
}

/**
 * R662: `text` as Glitch shows it — each character replaced by a glyph, its case flipped or kept, and
 * loaded with combining marks; spaces and line breaks stay, so the jumble keeps the text's shape. The
 * same text always gives the same jumble.
 */
export function corruptedText(text: string): string {
  const random = seededRandom(hashId(`${GLITCH_SALT}:${text}`));
  let out = "";
  for (const char of text) {
    if (char.trim() === "") {
      out += char;
      continue;
    }
    const roll = random();
    out += roll < GLYPH_SHARE ? pick(GLITCH_GLYPHS, random) : roll < GLYPH_SHARE + FLIP_SHARE ? flipCase(char) : char;
    const marks = Math.floor(random() * (MARKS_MAX + 1));
    for (let at = 0; at < marks; at += 1) out += pick(GLITCH_MARKS, random);
  }
  return out;
}
