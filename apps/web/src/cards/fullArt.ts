// Full-art faces (R1017001, a follow-up to issue #1017): the most iconic cards print their art across
// the whole face, the frame's panel hidden under it, their words on a smoked-glass rules box (cards.css,
// "full art"). With the whole text one hover or click away (inspect/Extended.tsx), a face can give its
// picture the room. They are the cards the hero portraits are drawn from (R641's roster, `PORTRAITS`
// in src/wire/emotes.ts): a card that becomes a hero's face becomes a full-art card, by name, as the
// roster names them, so a portrait the roster adds needs no line here.
//
// Presentation only (CLAUDE.md rule 7): the face is drawn differently and plays the same.

import { PORTRAITS } from "@jackioh/shared";

/** The English names of the cards whose faces print full art: every hero portrait's card. */
export const FULL_ART_NAMES: ReadonlySet<string> = new Set(Object.values(PORTRAITS).map((portrait) => portrait.cardName));

/** Whether the card named `englishName` prints full art (a Chinese face passes its English name). */
export function isFullArt(englishName: string): boolean {
  return FULL_ART_NAMES.has(englishName);
}
