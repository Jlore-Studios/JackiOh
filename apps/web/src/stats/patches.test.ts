// R1968002: the patch picker's choices and the patches either side of the one shown (stats/patches.ts).

import { describe, expect, it } from "vitest";

import { STATS_PATCH_MIN_GAMES, STATS_PATCHES_OFFERED } from "./config.ts";
import { neighbours, offeredPatches, patchIndex, patchName, patchTarget, type PatchWindow } from "./patches.ts";

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

describe("R1968002 the patches /stats can show", () => {
  it("R1968002 names a patch by its first version and asks its figures at its last", () => {
    expect(patchName(PATCHES[3]!)).toBe("v5");
    expect(patchTarget(PATCHES[3]!)).toBe("v5c");
    expect(patchIndex(PATCHES, "v5b")).toBe(3);
    expect(patchIndex(PATCHES, "v9")).toBe(-1);
  });

  it("R1968002 offers the newest STATS_PATCHES_OFFERED patches with STATS_PATCH_MIN_GAMES games, and the one shown", () => {
    const offered = offeredPatches(PATCHES, 3);
    expect(offered).toHaveLength(STATS_PATCHES_OFFERED);
    expect(offered.map(patchName)).toEqual(["v7", "v5", "v4", "v3", "v2"]);
    // The newest is shown with too few games: it is offered too, first.
    expect(offeredPatches(PATCHES, 0).map(patchName)).toEqual(["v8", "v7", "v5", "v4", "v3", "v2"]);
    // So is an older one reached by stepping back.
    expect(offeredPatches(PATCHES, 7).map(patchName)).toEqual(["v7", "v5", "v4", "v3", "v2", "v1"]);
    expect(offeredPatches([], -1)).toEqual([]);
  });

  it("R1968002 steps to the patch before and after the one shown", () => {
    expect(neighbours(PATCHES, 0)).toEqual({ previous: PATCHES[1], next: null });
    expect(neighbours(PATCHES, 3)).toEqual({ previous: PATCHES[4], next: PATCHES[2] });
    expect(neighbours(PATCHES, 7)).toEqual({ previous: null, next: PATCHES[6] });
    expect(neighbours(PATCHES, -1)).toEqual({ previous: null, next: null });
  });
});
