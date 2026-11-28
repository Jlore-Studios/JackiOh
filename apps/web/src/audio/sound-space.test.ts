// R5188001: the room the card parts' sounds may take. The three sound tables fit a gzip cap, as the
// first load fits its own (net/first-load.test.ts, #3499), and every card in the census's ranges adds
// at least its rarity's floor of sounds and at most its ceiling of lines since census-baseline.json,
// the sounds `main` had before R5188000.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import type { Rarity } from "@jackioh/shared";

import { CATALOG } from "../wire/cards.ts";
import { isRecord } from "../wire/json.ts";
import { CENSUS_SOUND_FLOOR, CENSUS_VOICE_CEILING, SOUND_TABLES_GZIP_BUDGET_BYTES } from "./constants.ts";
import { withVoiceText } from "./soundTables.ts";
import { census, idsIn, soundUnits } from "./test/census.ts";
import baselineJson from "./census-baseline.json";

const AUDIO = dirname(fileURLToPath(import.meta.url));
/** The sound tables the client loads with its first sound (soundTables.ts). */
const SOUND_TABLES = ["card-audio.json5", "voice-text.json", "voice-manifest.json"] as const;

/**
 * R5188001: the cards each card part has given its sounds, one range of catalog ids per line, both
 * ends included. A part adds its own range as it lands; none has yet.
 */
const CENSUS_RANGES: readonly (readonly [string, string])[] = [];

const BASELINE = baselineJson as Readonly<Record<string, readonly string[]>>;
const CATALOG_IDS = Object.keys(CATALOG);

function read(file: string): string {
  return readFileSync(join(AUDIO, file), "utf8");
}

/** Every card's sounds now, as census-baseline.json keeps them. */
function soundsNow(): Record<string, string[]> {
  const joined = withVoiceText(JSON5.parse(read("card-audio.json5")), JSON.parse(read("voice-text.json")));
  const cards = isRecord(joined) && isRecord(joined.cards) ? joined.cards : {};
  return Object.fromEntries(Object.entries(cards).map(([id, entry]) => [id, soundUnits(entry)]));
}

function rarityOf(id: string): Rarity {
  const rarity = CATALOG[id]?.rarity;
  if (rarity === undefined) throw new Error(`${id}: not in the catalog`);
  return rarity;
}

describe("the sound tables' room (R5188001)", () => {
  it("R5188001 the three sound tables fit SOUND_TABLES_GZIP_BUDGET_BYTES gzipped", () => {
    const sizes = SOUND_TABLES.map((file) => gzipSync(readFileSync(join(AUDIO, file))).length);
    const total = sizes.reduce((sum, size) => sum + size, 0);
    // Each table holds what it did on `main` before R5188000 or more (90,954 bytes in all then).
    expect(total).toBeGreaterThan(90_000);
    expect(total, `${SOUND_TABLES.map((file, i) => `${file} ${String(sizes[i])}`).join(", ")}`).toBeLessThanOrEqual(
      SOUND_TABLES_GZIP_BUDGET_BYTES,
    );
  });

  it("R5188001 every card in CENSUS_RANGES meets its rarity's floor and voice ceiling", () => {
    const ids = CENSUS_RANGES.flatMap((range) => idsIn(range, CATALOG_IDS));
    expect(new Set(ids).size, "no card is in two ranges").toBe(ids.length);
    expect(census(ids, soundsNow(), BASELINE, rarityOf)).toEqual([]);
  });

  it("R5188001 census-baseline.json is main's sounds before R5188000, by catalog card", () => {
    const now = soundsNow();
    const baselineIds = Object.keys(BASELINE);
    expect(baselineIds.length).toBeGreaterThan(1000);
    expect(baselineIds.filter((id) => !Object.hasOwn(CATALOG, id)), "baseline ids not in the catalog").toEqual([]);
    // Today's tables add to the baseline only the four effect-only proof hooks (R5188000); a card part's
    // census reads its own additions the same way.
    const added = baselineIds.flatMap((id) => {
      const before = new Set(BASELINE[id]);
      return (now[id] ?? []).filter((unit) => !before.has(unit)).map((unit) => `${id} ${unit}`);
    });
    expect(added).toEqual(expect.arrayContaining(["core-033 fx:activate", "core-037 fx:startOfTurn", "core-091 fx:whenever", "core-093 fx:endOfTurn"]));
  });

  it("R5188001 the census reports a card below its floor and one above its ceiling", () => {
    const baseline = { "core-004": ["attack", "death", "play"], "core-005": ["cast"], "core-006": ["cast"] };
    const now = {
      // Common: one sound added, under its floor of two.
      "core-004": ["attack", "death", "fx:hurt", "play"],
      // Common: two lines added, over its ceiling of one, and three sounds, over its floor.
      "core-005": ["cast", "cast2", "fx:hit", "hit"],
      // Common: a line of a pool and an effect: within both.
      "core-006": ["cast", "cast2", "fx:cast"],
    };
    const common = (): Rarity => "Common";
    expect(census(["core-004", "core-005", "core-006"], now, baseline, common)).toEqual([
      `core-004: 1 sounds added, below the Common floor of ${String(CENSUS_SOUND_FLOOR.Common)}`,
      `core-005: 2 lines added, above the Common ceiling of ${String(CENSUS_VOICE_CEILING.Common)}`,
    ]);
    // A card new since the baseline counts every sound it has.
    expect(census(["core-007"], { "core-007": ["fx:play"] }, {}, (): Rarity => "Token")).toEqual([]);
    expect(census(["core-007"], { "core-007": ["play"] }, {}, (): Rarity => "Token")).toEqual([
      `core-007: 1 lines added, above the Token ceiling of ${String(CENSUS_VOICE_CEILING.Token)}`,
    ]);
  });

  it("R5188001 a range is catalog ids in catalog order", () => {
    const ids = ["core-001", "core-002", "core-002-1", "core-003"];
    expect(idsIn(["core-002", "core-003"], ids)).toEqual(["core-002", "core-002-1", "core-003"]);
    expect(idsIn(["core-001", "core-001"], ids)).toEqual(["core-001"]);
    expect(() => idsIn(["core-003", "core-001"], ids)).toThrow("core-003 comes after core-001");
    expect(() => idsIn(["core-001", "core-999"], ids)).toThrow("core-999 is not a catalog id");
    expect(() => idsIn(["nope", "core-001"], ids)).toThrow("nope is not a catalog id");
    // The catalog's own order: a set's cards and its tokens run on from its first id.
    expect(idsIn(["core-001", "core-003"], CATALOG_IDS)[0]).toBe("core-001");
  });

  it("R5188001 soundUnits counts each line of a pool and a list of effects as one effect", () => {
    expect(
      soundUnits({
        play: { voice: "v", lines: ["a", "b", "c"], effect: ["x", "y"] },
        trigger: { voice: "v", lines: ["t"] },
        hurt: { voice: "v", text: "Ow." },
        hit: { effect: "z" },
      }),
    ).toEqual(["fx:hit", "fx:play", "hurt", "play", "play2", "play3", "trigger1"]);
  });
});
