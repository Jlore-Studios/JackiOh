// R1973002, R3324000: the patch tabs and the patches either side of the one shown, the newest patch collecting its
// data, (R3324001) the current patch and (R3447000) the patch a card's figures were collected on (stats/patches.ts).

import { describe, expect, it } from "vitest";

import { CATALOG_VERSION } from "@jackioh/cards";
import { STATS_PATCH_MIN_GAMES, STATS_PATCHES_OFFERED } from "./config.ts";
import {
  collectedOn,
  currentVersion,
  isCollecting,
  neighbours,
  offeredPatches,
  patchIndex,
  patchName,
  patchTarget,
  type PatchWindow,
} from "./patches.ts";

function patch(versions: string[], games: number): PatchWindow {
  return { versions, games, liveGames: 0 };
}

// Newest first, as `GET /api/stats/cards` lists them.
const PATCHES = [
  patch(["v8", "v8b"], STATS_PATCH_MIN_GAMES - 1),
  patch(["v7"], STATS_PATCH_MIN_GAMES),
  patch(["v6"], 0),
  patch(["v5", "v5b", "v5c"], 2 * STATS_PATCH_MIN_GAMES),
  patch(["v4"], STATS_PATCH_MIN_GAMES),
  patch(["v3"], STATS_PATCH_MIN_GAMES),
  patch(["v2"], STATS_PATCH_MIN_GAMES),
  patch(["v1"], STATS_PATCH_MIN_GAMES),
];

describe("R1973002 the patches /stats can show", () => {
  it("R1973002 names a patch by its first version and asks its figures at its last", () => {
    expect(patchName(PATCHES[3]!)).toBe("v5");
    expect(patchTarget(PATCHES[3]!)).toBe("v5c");
    expect(patchIndex(PATCHES, "v5b")).toBe(3);
    expect(patchIndex(PATCHES, "v9")).toBe(-1);
  });

  it("R1973002 R3324000 a tab for the newest patch, the newest STATS_PATCHES_OFFERED with STATS_PATCH_MIN_GAMES games, and the one shown", () => {
    // The newest, v8, has too few games and a tab, first, whichever patch is shown.
    const offered = offeredPatches(PATCHES, 3);
    expect(offered).toHaveLength(STATS_PATCHES_OFFERED + 1);
    expect(offered.map(patchName)).toEqual(["v8", "v7", "v5", "v4", "v3", "v2"]);
    expect(offeredPatches(PATCHES, 0).map(patchName)).toEqual(["v8", "v7", "v5", "v4", "v3", "v2"]);
    // An older one reached by stepping back has one while it is shown.
    expect(offeredPatches(PATCHES, 7).map(patchName)).toEqual(["v8", "v7", "v5", "v4", "v3", "v2", "v1"]);
    expect(offeredPatches(PATCHES, 2).map(patchName)).toEqual(["v8", "v7", "v6", "v5", "v4", "v3", "v2"]);
    // A newest patch with enough games counts among the five.
    expect(offeredPatches(PATCHES.slice(1), 0).map(patchName)).toEqual(["v7", "v5", "v4", "v3", "v2"]);
    expect(offeredPatches([], -1)).toEqual([]);
  });

  it("R3324000 the newest patch is collecting its data until it has STATS_PATCH_MIN_GAMES games, and no older patch is", () => {
    expect(isCollecting(PATCHES, 0)).toBe(true);
    // v6 has no game at all, but a newer patch shipped: its collecting is over.
    expect(isCollecting(PATCHES, 2)).toBe(false);
    expect(isCollecting(PATCHES, 1)).toBe(false);
    expect(isCollecting(PATCHES.slice(1), 0)).toBe(false);
    expect(isCollecting([patch(["v9"], 0)], 0)).toBe(true);
    expect(isCollecting(PATCHES, -1)).toBe(false);
    expect(isCollecting([], 0)).toBe(false);
  });

  it("R3324001 the current patch is the newest version listed, else the client's own catalog version", () => {
    expect(currentVersion(PATCHES)).toBe("v8b");
    expect(currentVersion(PATCHES.slice(1))).toBe("v7");
    expect(currentVersion([])).toBe(CATALOG_VERSION);
  });

  it("R3447000 a card's figures name the listed patch holding the first version they count, else that version", () => {
    // The previous window's figures while v8 gathers its games, and a window a micro opened: each its patch's name.
    expect(collectedOn(PATCHES, ["v7"])).toBe("v7");
    expect(collectedOn(PATCHES, ["v5b", "v5c"])).toBe("v5");
    expect(collectedOn(PATCHES, ["v8", "v8b"])).toBe("v8");
    // A version the list does not hold, or no list (an older server), names the version itself.
    expect(collectedOn(PATCHES, ["v9"])).toBe("v9");
    expect(collectedOn([], ["v5b"])).toBe("v5b");
    // No window, or one that counts nothing, names none.
    expect(collectedOn(PATCHES, undefined)).toBeNull();
    expect(collectedOn(PATCHES, [])).toBeNull();
  });

  it("R1973002 steps to the patch before and after the one shown", () => {
    expect(neighbours(PATCHES, 0)).toEqual({ previous: PATCHES[1], next: null });
    expect(neighbours(PATCHES, 3)).toEqual({ previous: PATCHES[4], next: PATCHES[2] });
    expect(neighbours(PATCHES, 7)).toEqual({ previous: null, next: PATCHES[6] });
    expect(neighbours(PATCHES, -1)).toEqual({ previous: null, next: null });
  });
});
