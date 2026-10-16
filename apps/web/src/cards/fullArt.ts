// Full-art faces (R1017001, a follow-up to issue #1017): the most iconic cards print their art across
// the whole face and no words, as Hearthstone's Signature cards do: the cost, the rarity gem opposite
// it, the name banner at the foot and a Unit's attack and health (cards.css, "full art"). Their whole text is
// one hover or click away (inspect/Extended.tsx, the detail view's rules). They are the cards the hero
// portraits are drawn from (R641's roster, `PORTRAITS` in src/wire/emotes.ts): a card that becomes a
// hero's face becomes a full-art card, by name, as the roster names them, so a portrait the roster
// adds needs no line here. The Full-art cards setting (settings.ts `fullArt`) turns them off, and they
// print their text like any card.
//
// Presentation only (CLAUDE.md rule 7): the face is drawn differently and plays the same.

import { PORTRAITS } from "@jackioh/shared";

import type { FaceModel } from "./model.ts";
import { readCardSettings } from "./settings.ts";
import { isGlitch } from "./glitch.ts";

/** The English names of the cards whose faces print full art: every hero portrait's card. */
export const FULL_ART_NAMES: ReadonlySet<string> = new Set(Object.values(PORTRAITS).map((portrait) => portrait.cardName));

/** Whether the card named `englishName` prints full art (a Chinese face passes its English name). */
export function isFullArt(englishName: string): boolean {
  return FULL_ART_NAMES.has(englishName);
}

/**
 * Whether `face`'s full layout prints full art: a hero portrait's card while the Full-art cards
 * setting is on. A compact face and a board minion never do (CardFace, MinionFace).
 */
export function showsFullArt(face: FaceModel, fullArtOn: boolean = readCardSettings().fullArt): boolean {
  return fullArtOn && !isGlitch(face.defId) && isFullArt(face.englishName ?? face.name);
}
