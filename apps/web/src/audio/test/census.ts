// Test helper (not a test file): R5188001's census of the sounds each card part adds (sound-space.test.ts).
// Pure: it reads tables the test hands it, never the disk.

import type { Rarity } from "@jackioh/shared";

import { CENSUS_SOUND_FLOOR, CENSUS_VOICE_CEILING } from "../constants.ts";
import { isRecord } from "../../wire/json.ts";

/** An effect's unit is its hook's name behind this; every other unit is a line. */
const EFFECT_UNIT = "fx:";

/**
 * One card's sounds, as `census-baseline.json` keeps them, from its `card-audio.json5` entry with its
 * words joined in (`withVoiceText`): each line a voiced hook speaks by its line's name (a trigger's
 * `trigger1` on, a pool's `play`, `play2` on, a single line's `play`) and each hook's effect, one or a
 * list, as `fx:<hook>`. Sorted.
 */
export function soundUnits(entry: unknown): string[] {
  const units: string[] = [];
  for (const [hook, assignment] of Object.entries(isRecord(entry) ? entry : {})) {
    if (!isRecord(assignment)) continue;
    if (assignment.voice !== undefined) {
      const lines = assignment.lines;
      if (Array.isArray(lines)) {
        lines.forEach((_, i) => units.push(hook === "trigger" ? `trigger${i + 1}` : i === 0 ? hook : `${hook}${i + 1}`));
      } else {
        units.push(hook);
      }
    }
    if (assignment.effect !== undefined) units.push(`${EFFECT_UNIT}${hook}`);
  }
  return units.sort();
}

/** The ids from `first` to `last`, both included, in `ids`' (the catalog's) order. Throws on an unknown id or a reversed range. */
export function idsIn([first, last]: readonly [string, string], ids: readonly string[]): string[] {
  const from = ids.indexOf(first);
  const to = ids.indexOf(last);
  if (from < 0) throw new Error(`census: ${first} is not a catalog id`);
  if (to < 0) throw new Error(`census: ${last} is not a catalog id`);
  if (to < from) throw new Error(`census: ${first} comes after ${last} in the catalog`);
  return ids.slice(from, to + 1);
}

/**
 * R5188001: one line per card of `ids` whose sounds added since `baseline` (a unit `now` has that the
 * baseline's entry does not) fall below its rarity's CENSUS_SOUND_FLOOR, or whose lines among them pass
 * its CENSUS_VOICE_CEILING; none when every card is within both.
 */
export function census(
  ids: readonly string[],
  now: Readonly<Record<string, readonly string[]>>,
  baseline: Readonly<Record<string, readonly string[]>>,
  rarityOf: (id: string) => Rarity,
): string[] {
  const problems: string[] = [];
  for (const id of ids) {
    const before = new Set(baseline[id] ?? []);
    const added = (now[id] ?? []).filter((unit) => !before.has(unit));
    const lines = added.filter((unit) => !unit.startsWith(EFFECT_UNIT)).length;
    const rarity = rarityOf(id);
    const floor = CENSUS_SOUND_FLOOR[rarity];
    const ceiling = CENSUS_VOICE_CEILING[rarity];
    if (added.length < floor) problems.push(`${id}: ${added.length} sounds added, below the ${rarity} floor of ${floor}`);
    if (lines > ceiling) problems.push(`${id}: ${lines} lines added, above the ${rarity} ceiling of ${ceiling}`);
  }
  return problems;
}
