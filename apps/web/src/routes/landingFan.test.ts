// The landing fan's deal (landingFan.ts, R374): a random hand on every visit, from the real catalog,
// in the fixed hand's shape. The source of randomness is passed in, so each case here deals from a
// seeded one and gets the same hand every run.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { seeded } from "../test/random.ts";
import { FAN_FACES, FAN_POOL, FAN_RADIANT_AT, dealLandingFan } from "./landingFan.ts";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const CATALOG = JSON.parse(readFileSync(resolve(REPO, "packages/cards/catalog.json"), "utf8")) as Record<string, CardDef>;

const SEEDS = Array.from({ length: 200 }, (_, i) => i + 1);

describe("R374 the landing fan is dealt at random", () => {
  it("R374 the pool is every non-token Core card, exactly as the catalog holds it", () => {
    const expected = Object.values(CATALOG).filter((def) => !def.token);
    expect(FAN_POOL).toHaveLength(100);
    expect(FAN_POOL).toEqual(expected);
  });

  it("R374 every deal is four real cards of four rarities, the middle one Radiant", () => {
    for (const seed of SEEDS) {
      const hand = dealLandingFan(seeded(seed));
      expect(hand, `seed ${String(seed)}`).toHaveLength(FAN_FACES);
      expect(new Set(hand.map(({ def }) => def.rarity)).size, `seed ${String(seed)}`).toBe(FAN_FACES);
      expect(hand.map(({ radiant }) => radiant)).toEqual([0, 1, 2, 3].map((at) => at === FAN_RADIANT_AT));
      for (const { def } of hand) {
        expect(def.token).toBe(false);
        expect(def).toEqual(CATALOG[def.id]);
      }
    }
  });

  it("R374 the same source deals the same hand, and different visits deal different hands", () => {
    const ids = (seed: number): string[] => dealLandingFan(seeded(seed)).map(({ def }) => def.id);
    expect(ids(7)).toEqual(ids(7));
    const hands = new Set(SEEDS.map((seed) => ids(seed).join(",")));
    // Not one fixed hand any more: two hundred visits see well over a hundred different hands.
    expect(hands.size).toBeGreaterThan(SEEDS.length / 2);
    // And the deal reaches the whole pool, not a corner of it.
    const seen = new Set(SEEDS.flatMap(ids));
    expect(seen.size).toBeGreaterThan(FAN_POOL.length / 2);
  });

  it("R374 a source at its edges (0, and just under 1) still deals a legal hand", () => {
    for (const value of [0, 0.999999999]) {
      const hand = dealLandingFan(() => value);
      expect(hand).toHaveLength(FAN_FACES);
      expect(new Set(hand.map(({ def }) => def.rarity)).size).toBe(FAN_FACES);
    }
  });

  it("R374 a pool with fewer rarities deals fewer cards and repeats none; an empty one deals none", () => {
    const twoRarities = FAN_POOL.filter((def) => def.rarity === "Common" || def.rarity === "Rare");
    const hand = dealLandingFan(seeded(3), twoRarities);
    expect(hand).toHaveLength(2);
    expect(new Set(hand.map(({ def }) => def.rarity))).toEqual(new Set(["Common", "Rare"]));
    expect(dealLandingFan(seeded(3), [])).toEqual([]);
  });
});
