// Issue #3734 (R3734000): the Cosmetics tab's search, rarity, ownership and sort over the account's tiles.

import { describe, expect, it } from "vitest";

import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { accountTiles } from "./collection.ts";
import {
  DEFAULT_COSMETICS_BROWSE,
  browseTiles,
  cosmeticsFiltersOn,
  type CosmeticsBrowse,
} from "./cosmeticsBrowse.ts";

/** Every card back, the account owning the first two of them. */
const backs = COSMETICS.filter((cosmetic) => cosmetic.kind === "cardBack");
const OWNED = new Set<string>(backs.slice(0, 2).map((cosmetic) => cosmetic.id));
const TILES = accountTiles("cardBack", (id) => OWNED.has(id));
const ids = (browse: Partial<CosmeticsBrowse>): string[] =>
  browseTiles(TILES, { ...DEFAULT_COSMETICS_BROWSE, ...browse }).map((tile) => tile.cosmetic.id);

describe("R3734000 the Cosmetics tab's browse", () => {
  it("R3734000 the defaults keep every tile in the registry's order", () => {
    expect(TILES.length).toBeGreaterThan(3);
    expect(ids({})).toEqual(TILES.map((tile) => tile.cosmetic.id));
    expect(cosmeticsFiltersOn(DEFAULT_COSMETICS_BROWSE)).toBe(0);
  });

  it("R3734000 the search keeps the names that hold every word, in any case and order", () => {
    const sample = backs[1];
    expect(sample).toBeDefined();
    const name = sample?.name ?? "";
    expect(ids({ search: name.toUpperCase() })).toContain(sample?.id);
    expect(ids({ search: `  ${name.toLowerCase()}  ` })).toContain(sample?.id);
    expect(ids({ search: "zzz no such back" })).toEqual([]);
    const words = name.split(" ");
    expect(ids({ search: [...words].reverse().join(" ") })).toContain(sample?.id);
    expect(cosmeticsFiltersOn({ ...DEFAULT_COSMETICS_BROWSE, search: "x" })).toBe(1);
  });

  it("R3734000 the rarity chips OR, and none on is every rarity", () => {
    const rarities = [...new Set(TILES.map((tile) => tile.cosmetic.rarity))];
    expect(rarities.length).toBeGreaterThan(1);
    const [first, second] = rarities;
    expect(first).toBeDefined();
    const ofFirst = TILES.filter((tile) => tile.cosmetic.rarity === first).map((tile) => tile.cosmetic.id);
    expect(ids({ rarities: new Set(first === undefined ? [] : [first]) })).toEqual(ofFirst);
    const both = TILES.filter((tile) => tile.cosmetic.rarity === first || tile.cosmetic.rarity === second).map((tile) => tile.cosmetic.id);
    expect(ids({ rarities: new Set([first, second].flatMap((rarity) => (rarity === undefined ? [] : [rarity]))) })).toEqual(both);
  });

  it("R3734000 owned and missing split the tiles exactly", () => {
    const owned = ids({ ownership: "owned" });
    const missing = ids({ ownership: "missing" });
    expect(new Set(owned)).toEqual(OWNED);
    expect(owned.filter((id) => missing.includes(id))).toEqual([]);
    expect([...owned, ...missing].sort()).toEqual(TILES.map((tile) => tile.cosmetic.id).sort());
  });

  it("R3734000 sorting by name orders the names, and by rarity the rarities, each stable for equals", () => {
    const byName = browseTiles(TILES, { ...DEFAULT_COSMETICS_BROWSE, sort: "name" }).map((tile) => tile.cosmetic.name);
    expect(byName).toEqual([...byName].sort((left, right) => left.localeCompare(right, undefined, { sensitivity: "base" })));
    const order = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
    const byRarity = browseTiles(TILES, { ...DEFAULT_COSMETICS_BROWSE, sort: "rarity" });
    const ranks = byRarity.map((tile) => order.indexOf(tile.cosmetic.rarity));
    expect(ranks).toEqual([...ranks].sort((left, right) => left - right));
    // Equals keep the registry's order.
    for (let at = 1; at < byRarity.length; at += 1) {
      const before = byRarity[at - 1];
      const after = byRarity[at];
      if (before === undefined || after === undefined || before.cosmetic.rarity !== after.cosmetic.rarity) continue;
      expect(TILES.indexOf(before)).toBeLessThan(TILES.indexOf(after));
    }
  });
});
