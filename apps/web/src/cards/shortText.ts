// Short rules text (issues #814, #1017): what a face prints when its whole text would not fit its rules box.
// The art never gives way (fit.ts, `long: false`): the face prints a short form, or its text cut at the box's
// last line with "…", and the whole text stands on the left of its hover preview (inspect/Extended.tsx), in the
// touch sheet and in the detail view. #1017 cut every catalog face under the limit (constants.ts FACE_TEXT_LIMIT)
// but these, whose text is a list no box holds or a line over after the cut. The forms: `n` (a number) its first
// n lines, "…" in place of the last full stop (#98 Heroic Power's thirteen powers; faces whose last line is a
// reminder or flavour); "colon" up to its first colon (the Call to Chaos editions, whose list must stay whole in
// the catalog, R436); "clamp" the whole text, cut by the box at its last line (#1563, #1704, #1828).
// Stinkyscape (R1857000, docs/stinkyscape-set.md ST13): Call to Chaos #150 and #151 take "colon", every face its
// rows put over the limit clamps unless the card's part wrote its own; #4.5's and #26's base faces take lines or a
// colon (#2067); Quests #7 and #112 print one stage (questFace.ts, R2450000). The held Training Grounds set
// (R2194000, docs/training-grounds-set.md TG5): Call to Chaos #39 "colon", the Radiant faces of #16 and #38 clamp.
// #2521: Classic+ #34, Meditative+ #1 and ZolarKeth's Radiant face. Only a face printing its catalog text takes a
// short form (in play #98 prints its rolled power and a Call to Chaos "???", inPlay.ts). In play the Summit prints
// its header and the next Ascent line only (#814): the level the engine's preview carries as "Ascent level" (R280,
// c008_reach_the_summit.rs); a face in play past the preview's limit clamps rather than shrinking its art.

import type { PreviewValue } from "@jackioh/shared";

/** Meditative #8 Reach the Summit. */
export const SUMMIT_DEF_ID = "meditative-008";

export const ASCENT_LABEL = "Ascent level";

const ASCENT_LINE = /^Ascent (\d+):/;

export type ShortForm = number | "colon" | "clamp";

export const SHORT_FORMS: Readonly<Record<string, { base?: ShortForm; radiant?: ShortForm }>> = {
  "core-093": { base: 2, radiant: 2 },
  "core-095": { base: "colon", radiant: "colon" },
  "core-098": { base: 2, radiant: 2 },
  "classic-005": { base: 2, radiant: 2 },
  "classic-023": { base: "clamp" },
  "classicplus-007": { radiant: "clamp" },
  "classicplus-019-2": { base: "clamp", radiant: "clamp" },
  "classicplus-042-1": { radiant: "clamp" },
  "classicplus-034": { base: "clamp" },
  "classicplus-073": { base: "colon", radiant: "colon" },
  "meditative-008": { base: 2, radiant: 2 },
  "meditative-022": { base: 1, radiant: 1 },
  "meditative-022-1": { base: 3, radiant: 2 },
  "meditative-039-5": { base: 3, radiant: 3 },
  "meditative-040": { radiant: "clamp" },
  "meditative-045": { base: "clamp", radiant: "clamp" },
  "meditative-049-1": { base: "clamp", radiant: "clamp" },
  "meditative-049-2": { base: "clamp", radiant: "clamp" },
  "meditative-091": { radiant: "clamp" },
  "meditative-095": { base: "colon", radiant: "colon" },
  "meditative-097": { base: 1, radiant: 1 },
  "meditative-098": { radiant: "clamp" },
  "meditativeplus-001": { base: "clamp", radiant: "clamp" },
  "meditativeplus-004": { base: "clamp", radiant: "clamp" },
  "meditativeplus-020": { radiant: "clamp" },
  "meditativeplus-024": { base: "clamp", radiant: "clamp" },
  "stinkyscape-004-4": { base: "clamp", radiant: "clamp" },
  "stinkyscape-004-5": { base: "colon", radiant: "clamp" },
  "stinkyscape-007": { radiant: "clamp" },
  "stinkyscape-016": { base: "clamp" },
  "stinkyscape-018-1": { base: "clamp", radiant: "clamp" },
  "stinkyscape-018-2": { base: "clamp", radiant: "clamp" },
  "stinkyscape-018-3": { radiant: "clamp" },
  "stinkyscape-023": { base: "clamp", radiant: "clamp" },
  "stinkyscape-024": { base: "clamp", radiant: "clamp" },
  "stinkyscape-025": { base: "clamp" },
  "stinkyscape-039": { base: "clamp", radiant: "clamp" },
  "stinkyscape-077": { base: "clamp", radiant: "clamp" },
  "stinkyscape-087": { radiant: "clamp" },
  "stinkyscape-112": { base: "clamp", radiant: "clamp" },
  "stinkyscape-116": { base: "clamp", radiant: "clamp" },
  "stinkyscape-139": { base: "clamp", radiant: "clamp" },
  "stinkyscape-143": { base: "clamp", radiant: "clamp" },
  "stinkyscape-147": { radiant: "clamp" },
  "stinkyscape-148": { base: "clamp", radiant: "clamp" },
  "stinkyscape-150": { base: "colon", radiant: "colon" },
  "stinkyscape-151": { base: "colon", radiant: "colon" },
  "stinkyscape-153": { base: "clamp", radiant: "clamp" },
  "stinkyscape-154": { base: "clamp", radiant: "clamp" },
  "stinkyscape-157": { base: "clamp", radiant: "clamp" },
  "stinkyscape-164": { base: "clamp", radiant: "clamp" },
  "stinkyscape-166": { base: "clamp", radiant: "clamp" },
  "stinkyscape-167": { base: "clamp", radiant: "clamp" },
  "stinkyscape-t-anomaly-found": { base: "clamp", radiant: "clamp" },
  // R2194000: the held Training Grounds set's rows.
  "traininggrounds-016": { radiant: "clamp" },
  "traininggrounds-038": { radiant: "clamp" },
  "traininggrounds-039": { base: "colon", radiant: "colon" },
};

// R1017000: every catalog face whose printed text runs past TEXT_TIER_MAX.xl (260
// characters), as "defId base|radiant". textLimit.test.ts pins this list on every pull
// request; the B15 and MN09 component specs read it for their clamp diagnostics.
export const LONG_FACES: readonly string[] = [
  "meditative-008 base",
  "meditative-008 radiant",
  "meditative-022 base",
  "meditative-022 radiant",
  "meditative-022-1 base",
  "meditative-022-1 radiant",
  "meditative-049-1 base",
  "meditative-049-1 radiant",
  "meditative-095 base",
  "meditative-095 radiant",
  "core-093 base",
  "core-093 radiant",
  "core-095 base",
  "core-095 radiant",
  "core-098 base",
  "core-098 radiant",
  "classicplus-073 base",
  "classicplus-073 radiant",
  "stinkyscape-150 base",
  "stinkyscape-150 radiant",
  "stinkyscape-151 base",
  "stinkyscape-151 radiant",
  // R2202001: the held Training Grounds set's longest, its Call to Chaos edition's table.
  "traininggrounds-039 base",
  "traininggrounds-039 radiant",
];

/** What ends a short form: the text goes on in the extended description. */
export const MORE = "…";

/** A colon, English or Chinese (R1302), for the "colon" form. */
const COLON = /[:：]/;

const LAST_STOP = /[.:。：]$/;

/** `cut` ending in "…", which replaces its last full stop or colon. */
function trailOff(cut: string): string {
  return `${cut.trimEnd().replace(LAST_STOP, "")}${MORE}`;
}

/** The short form of a card's face, or null when the face prints its whole text. */
export function shortFormOf(defId: string, radiant: boolean): ShortForm | null {
  const forms = SHORT_FORMS[defId];
  return (radiant ? forms?.radiant : forms?.base) ?? null;
}

/**
 * The words a `n` or "colon" short form prints, ending in "…" (`trailOff`). Null when there is nothing to cut
 * and for "clamp", which prints the whole text.
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
 * What a Summit face in play prints for `level`: the header (every line before the first "Ascent N:" line)
 * and only the level's own line. Null when there is nothing to shorten to (a translated face parses no Ascent lines).
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

export function overLimit(text: string, limit: TextLimit): boolean {
  return faceLines(text, limit.charsPerLine) > limit.lines;
}
