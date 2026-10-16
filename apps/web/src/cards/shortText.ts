// Short rules text (issues #814, #1017): what a face prints when its whole text would not fit its
// rules box. The art never gives way for such a face (fit.ts, `long: false`): it prints a short form,
// or its text cut at the box's last line with "…", and its whole text stands on the left of its
// hover preview (inspect/Extended.tsx), in the touch sheet and in the detail view.
//
// Issue #1017 measured the limit (constants.ts FACE_TEXT_LIMIT) and cut every catalog face under it,
// but for these. Each card's text is a list no box holds, or a face still a line over after the cut:
// - `n` (a number): its first n lines, which its text puts first, "…" in place of the last full
//   stop: #98 Heroic Power's thirteen powers, Meditative #8 Reach the Summit's eleven levels,
//   Combo-Index's grades, Mind Games' and Fortify Mind's three choices, and YileGPT Unleashed's list
//   of Cries; and a few faces that fit, whose last line is a reminder or the designer's flavour
//   that the click-in carries (Tesla, Jade Beauty, Jlockeed's Evil Blueprints);
// - "colon": up to its first colon, "…" in its place: the three Call to Chaos editions, whose list
//   must stay whole in the catalog (each roll's label is a clause of it, R436);
// - "clamp": its whole text, cut by the rules box at its last line with "…": eight faces still about
//   a line over after the cut (Yile's Virus and Knowledge Breaker, and the Radiant faces of KY's Gift,
//   Feng Shui, Money Machine and Windfast), which a 200 px tile prints whole.
// Only a face that prints its catalog text takes a short form: in play a #98 Heroic Power prints its
// rolled power and a Call to Chaos "???" (inPlay.ts) instead.
//
// In play the Summit prints its header and only the next Ascent line (#814): the level the engine's
// preview carries under "Ascent level" (R280, c008_reach_the_summit.rs), which is the play count on
// the instance. A face in play that runs past the hover preview's limit (a fused card's ingredients,
// a heavily Buffed card's keywords) clamps rather than shrinking its art, and its whole text stands
// on its left too.

import type { PreviewValue } from "@jackioh/shared";

/** Meditative #8 Reach the Summit, whose face in play prints only the next Ascent line. */
export const SUMMIT_DEF_ID = "meditative-008";

/** R280: the label the engine's preview carries the Summit's Ascent level under. */
export const ASCENT_LABEL = "Ascent level";

/** One "Ascent N:" line of the Summit's text. */
const ASCENT_LINE = /^Ascent (\d+):/;

/** What a face over the limit prints: its first n lines, up to its first colon, or its text clamped. */
export type ShortForm = number | "colon" | "clamp";

/** Issue #1017: every face over FACE_TEXT_LIMIT after the cut, by card and face. */
export const SHORT_FORMS: Readonly<Record<string, { base?: ShortForm; radiant?: ShortForm }>> = {
  "core-093": { base: 2, radiant: 2 },
  "core-095": { base: "colon", radiant: "colon" },
  "core-098": { base: 2, radiant: 2 },
  "classic-005": { base: 2, radiant: 2 },
  "classicplus-042-1": { radiant: "clamp" },
  "classicplus-073": { base: "colon", radiant: "colon" },
  "meditative-008": { base: 2, radiant: 2 },
  "meditative-022": { base: 1, radiant: 1 },
  "meditative-022-1": { base: 3, radiant: 2 },
  "meditative-039-5": { base: 3, radiant: 3 },
  "meditative-040": { radiant: "clamp" },
  "meditative-045": { base: "clamp", radiant: "clamp" },
  "meditative-049-1": { base: 3, radiant: 3 },
  "meditative-049-2": { base: "clamp", radiant: "clamp" },
  "meditative-054": { radiant: "clamp" },
  "meditative-091": { radiant: "clamp" },
  "meditative-095": { base: "colon", radiant: "colon" },
  "meditative-097": { base: 1, radiant: 1 },
};

/** What ends a short form, in place of its last full stop or colon: the text goes on in the extended description. */
export const MORE = "…";

/** A colon, English or Chinese (R1302), for the "colon" form. */
const COLON = /[:：]/;

/** The stop a short form's "…" takes the place of: a full stop or a colon, English or Chinese. */
const LAST_STOP = /[.:。：]$/;

/** `cut` ending in "…", which replaces its last full stop or colon: "…take all three penalties…". */
function trailOff(cut: string): string {
  return `${cut.trimEnd().replace(LAST_STOP, "")}${MORE}`;
}

/** The short form of a card's face, or null when the face prints its whole text. */
export function shortFormOf(defId: string, radiant: boolean): ShortForm | null {
  const forms = SHORT_FORMS[defId];
  return (radiant ? forms?.radiant : forms?.base) ?? null;
}

/**
 * The words a `n` or "colon" short form prints: the text's first n lines, or the text up to its
 * first colon, ending in "…" in place of the full stop or colon (`trailOff`). Null when there is
 * nothing to cut (a text no longer than its form, or one without a colon) and for "clamp", which
 * prints the whole text.
 */
export function cutText(full: string, form: ShortForm): string | null {
  if (form === "clamp") return null;
  if (form === "colon") {
    const at = full.search(COLON);
    return at < 0 || at === full.length - 1 ? null : trailOff(full.slice(0, at + 1));
  }
  const lines = full.split("\n");
  return lines.length <= form ? null : trailOff(lines.slice(0, form).join("\n"));
}

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
 * another card, no level, text with no Ascent lines (a translated face parses none, so it takes
 * its short form instead), or a level past the printed lines.
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

/** A text limit: the lines a rules box holds at the reading floor, and the characters in each. */
export type TextLimit = { readonly charsPerLine: number; readonly lines: number };

/** The lines `text` takes wrapped by words at `charsPerLine`, each of its own lines starting a new one. */
export function faceLines(text: string, charsPerLine: number): number {
  let count = 0;
  for (const line of text.split("\n")) {
    let width = 0;
    count += 1;
    for (const word of line.split(" ")) {
      if (width === 0) width = word.length;
      else if (width + 1 + word.length <= charsPerLine) width += 1 + word.length;
      else {
        count += 1;
        width = word.length;
      }
    }
  }
  return count;
}

/** Whether `text` runs past `limit`. */
export function overLimit(text: string, limit: TextLimit): boolean {
  return faceLines(text, limit.charsPerLine) > limit.lines;
}
