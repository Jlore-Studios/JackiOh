// R3331001: the detail floor (ART.md, "The detail floor"): how much a card's picture draws, measured
// on its base face, and the least each rarity draws, so a rarer card is more detailed. A card under its
// floor today is named in `ART_DETAIL_SHORT` (fixtures/art-detail-short.json), a ratchet that only
// shrinks: each set's picture pass lifts its cards over their floors and takes them off it, and the last
// pass empties it. A ceiling keeps phones fast: no picture's data URI is longer than
// `ART_DETAIL_MAX_BYTES`. `artRichness.test.ts` holds every catalog entry to all three.

import type { Rarity } from "@jackioh/shared";

import SHORT from "./fixtures/art-detail-short.json";
import { SCENE_LAYERS } from "./scene.ts";

/** A rarity's floor: the elements its picture draws, and the scene layers it uses (0: any). */
export type ArtDetailFloor = { readonly elements: number; readonly layers: number };

/** Common: one clear subject and a simple setting. */
export const ART_DETAIL_MIN_COMMON: ArtDetailFloor = { elements: 30, layers: 0 };
/** Rare: a setting with props. */
export const ART_DETAIL_MIN_RARE: ArtDetailFloor = { elements: 40, layers: 0 };
/** Epic: a scene with a foreground, a middle and a background. */
export const ART_DETAIL_MIN_EPIC: ArtDetailFloor = { elements: 60, layers: 0 };
/** Legendary: a composed scene with lighting and a moment of action, in every layer. */
export const ART_DETAIL_MIN_LEGENDARY: ArtDetailFloor = { elements: 90, layers: SCENE_LAYERS.length };
/** Mythic: a set piece of its own, many parts deep, in every layer. */
export const ART_DETAIL_MIN_MYTHIC: ArtDetailFloor = { elements: 140, layers: SCENE_LAYERS.length };
/** A token, which prints no rarity of its own in its window, is held to Common's. */
export const ART_DETAIL_MIN_TOKEN: ArtDetailFloor = ART_DETAIL_MIN_COMMON;

export const ART_DETAIL_MIN: Readonly<Record<Rarity, ArtDetailFloor>> = {
  Common: ART_DETAIL_MIN_COMMON,
  Rare: ART_DETAIL_MIN_RARE,
  Epic: ART_DETAIL_MIN_EPIC,
  Legendary: ART_DETAIL_MIN_LEGENDARY,
  Mythic: ART_DETAIL_MIN_MYTHIC,
  Token: ART_DETAIL_MIN_TOKEN,
};

/**
 * The longest a picture's data URI may be, base or Radiant, so a phone parses each fast: the proof scenes run
 * to about 38 KB and the painters' pictures to about 24 KB.
 */
export const ART_DETAIL_MAX_BYTES = 48 * 1024;

/** The cards under their floor today, by id, sorted: a set's pass takes its cards off. */
export const ART_DETAIL_SHORT: ReadonlySet<string> = new Set<string>(SHORT);

/**
 * The most `ART_DETAIL_SHORT` may name: its length as the last pass left it, so it can only shrink (561 after
 * #3331; 445 after #3390, R3390000, which took Classic+'s 116 off; 337 after #3392, R3392000, which took Core's
 * 108 off; 289 after #3393, R3393000, which took Meditative+'s and Training Grounds' 48 off; 157 after #3401,
 * R3401000, which took Meditative's 132 off; 129 after #3403, R3403000, which took the 28 of Stinkyscape's later
 * cards still on it off).
 */
export const ART_DETAIL_SHORT_MAX = 129;

/** The shapes a picture draws: its paths, circles, rects, ellipses, polygons, polylines and lines, after its `<defs>`. */
export function artElements(svg: string): number {
  const cut = svg.indexOf("</defs>");
  const drawn = cut < 0 ? svg : svg.slice(cut);
  return drawn.match(/<(path|circle|rect|ellipse|polygon|polyline|line)\b/g)?.length ?? 0;
}
