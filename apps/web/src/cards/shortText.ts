// Short rules text for a face in play (issue #814): a card whose printed text is a long list of
// which only one line matters now prints its header and that line, with the whole text kept in
// the rules box's tooltip and the inspect overlays.
//
// The one card this shortens is Meditative #8 Reach the Summit. Its text is three header lines
// (the Ascent level, what a play does with it, the return to hand) and one "Ascent N:" line per
// level, 0 to 10. In play each play resolves only its own level's line, so the face prints the
// header and the next line only: the level the engine's preview carries under "Ascent level"
// (R280, c008_reach_the_summit.rs), which is the play count on the instance.
//
// A Heroic Power in play already prints only its rolled power and Call to Chaos prints "???"
// (inPlay.ts), so neither needs this. The Almanac, the deck builder and the inspect overlays
// never pass a level and always print the whole text.

import type { PreviewValue } from "@jackioh/shared";

/** Meditative #8 Reach the Summit, the one card with a shorten-able face. */
export const SUMMIT_DEF_ID = "meditative-008";

/** R280: the label the engine's preview carries the Summit's Ascent level under. */
export const ASCENT_LABEL = "Ascent level";

/** One "Ascent N:" line of the Summit's text. */
const ASCENT_LINE = /^Ascent (\d+):/;

/**
 * The Summit's Ascent level as the view gives it (`CardView.preview`, R280). Null for every
 * other card and wherever the view carries no preview (the collection).
 */
export function ascentLevelOf(values: readonly PreviewValue[]): number | null {
  const found = values.find((entry) => entry.label === ASCENT_LABEL);
  return found === undefined ? null : found.value;
}

/**
 * What a Summit face in play prints for `level`: the header (every line before the first
 * "Ascent N:" line) and only the level's own line. Null when there is nothing to shorten to —
 * another card, no level, text with no Ascent lines (a translated face parses none, so it keeps
 * the whole text), or a level past the printed lines — and the caller prints the full text.
 */
export function summitShortText(defId: string, full: string, level: number | null | undefined): string | null {
  if (defId !== SUMMIT_DEF_ID || level === null || level === undefined) return null;
  const lines = full.split("\n");
  const first = lines.findIndex((line) => ASCENT_LINE.test(line.trim()));
  if (first < 0) return null;
  const next = lines.find((line) => {
    const match = ASCENT_LINE.exec(line.trim());
    return match !== null && Number(match[1]) === level;
  });
  if (next === undefined) return null;
  return [...lines.slice(0, first), next].join("\n");
}
