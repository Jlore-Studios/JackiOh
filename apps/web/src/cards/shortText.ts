// Short rules text (issues #814, #1017): what a face prints when its whole text would not fit its
// rules box. The art never gives way for such a face (fit.ts, `long: false`): it prints a short form,
// or its text cut at the box's last line with "…", and its whole text stands on the left of its
// hover preview (inspect/Extended.tsx), in the touch sheet and in the detail view.
//
// Issue #1017 measured the limit (constants.ts FACE_TEXT_LIMIT) and cut every catalog face under it,
// but for these. Each card's text is a list no box holds, or a face still a line over after the cut:
// - `n` (a number): its first n lines, which its text puts first, "…" in place of the last full
//   stop: lists such as #98 Heroic Power's thirteen powers, and a few faces that fit, whose last
//   line is a reminder or the designer's flavour that the click-in carries;
// - "colon": up to its first colon, "…" in its place: the three Call to Chaos editions, whose list
//   must stay whole in the catalog (each roll's label is a clause of it, R436);
// - "clamp": its whole text, cut by the rules box at its last line with "…": seven faces still about
//   a line over after the cut, which a 200 px tile prints whole, three #1563 left a line over, one
//   #1704 did, and #1828's Can't attack put both faces of Jungle Loser a line over.
// The Stinkyscape set (R1857000) printed its rows before its cards were built: its two Call to
// Chaos editions (#150 and #151) take "colon" as the other three do, and every other face its rows
// put over the limit clamps, unless the card's part (docs/stinkyscape-set.md ST13) wrote its own.
// Where a face just at the limit took the long layout at 170 px (#4.5 1v1 Final Destination's and
// #26 ZolarKeth's base faces), it prints its first lines or up to its first colon instead (#2067).
// The set's two staged Quests, #7 The Greatest Heist and #112 Job Application, print one stage
// (questFace.ts, R2450000), measured as they print it.
// The held Training Grounds set (R2194000) does the same while it is built: its Call to Chaos edition (#39) takes
// "colon", and the Radiant faces of #16 Celestial Caller and #38 Tutorial Master clamp, until each
// card's part (docs/training-grounds-set.md TG5) writes its own.
// Issue #2521's texts: Classic+ #34 Memory Leak's base face and Meditative+ #1 Bob's Plan's clamp,
// and ZolarKeth's Radiant face prints its first line, as its base face does.
// Only a face that prints its catalog text takes a short form: in play a #98 Heroic Power prints its
// rolled power and a Call to Chaos "???" (inPlay.ts) instead.
//
// In play the Summit prints its header and only the next Ascent line (#814): the level the engine's
// preview carries under "Ascent level" (R280, c008_reach_the_summit.rs), which is the play count on
// the instance. A face in play that runs past the hover preview's limit clamps rather than shrinking
// its art, and its whole text stands on its left too.

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
 * The words a `n` or "colon" short form prints, ending in "…" (`trailOff`). Null when there is
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
 * "Ascent N:" line) and only the level's own line. Null when there is nothing to shorten to (a
 * translated face parses no Ascent lines, so it takes its short form instead).
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
