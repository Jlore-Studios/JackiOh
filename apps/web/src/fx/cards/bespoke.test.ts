// The table key predicates (fx/cards/bespoke.ts): whether a string names a bespoke recipe or an
// entrance. `isCardRecipeKey` is read in three per-set suites, but neither predicate is pinned
// colocated; a mutant accepting a wrong key, or dropping a real one, fails here.

import { describe, expect, it } from "vitest";

import { ENTRANCE_TRAUMA } from "./recipe.ts";
import { CARD_RECIPES, isCardRecipeKey, isEntranceKey } from "./bespoke.ts";

describe("bespoke table keys", () => {
  it("accepts every bespoke recipe by name, and nothing else", () => {
    const keys = Object.keys(CARD_RECIPES);
    expect(keys.length).toBeGreaterThan(0);
    for (const key of keys) expect(isCardRecipeKey(key), key).toBe(true);
    expect(isCardRecipeKey("coinCall")).toBe(true);
    expect(isCardRecipeKey("nope")).toBe(false);
    expect(isCardRecipeKey("")).toBe(false);
  });

  it("accepts every entrance by name, and nothing else", () => {
    const keys = Object.keys(ENTRANCE_TRAUMA);
    expect(keys.length).toBeGreaterThan(0);
    for (const key of keys) expect(isEntranceKey(key), key).toBe(true);
    expect(isEntranceKey("voidCollapse")).toBe(true);
    expect(isEntranceKey("poolBreath")).toBe(true);
    expect(isEntranceKey("nope")).toBe(false);
    expect(isEntranceKey("")).toBe(false);
  });

  it("pins the Void's entrance the heaviest, at the legendaries' weight below it", () => {
    expect(ENTRANCE_TRAUMA.voidCollapse).toBe(0.8);
    expect(ENTRANCE_TRAUMA.bigBoom).toBe(0.75);
    expect(ENTRANCE_TRAUMA.shipyardRollout).toBe(0.5);
  });
});
