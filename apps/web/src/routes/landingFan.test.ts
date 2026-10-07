// The landing fan's deal (landingFan.ts, R374): a random hand on every visit, from the real catalog,
// in the fixed hand's shape. The source of randomness is passed in, so each case here deals from a
// seeded one and gets the same hand every run.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { nameTier, textTier } from "../cards/fit.ts";
import { faceModel } from "../cards/model.ts";
import { FEATURE_WEIGHT_DENSE, FEATURE_WEIGHT_PLAIN } from "../stats/config.ts";
import { seeded } from "../test/random.ts";
import {
  EVEN,
  FAN_FACES,
  FAN_POOL,
  FAN_RADIANT_AT,
  ROTATION_POOL,
  dealLandingFan,
  featureWeight,
  pickWeighted,
  rotateFan,
} from "./landingFan.ts";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");
const CATALOG = JSON.parse(readFileSync(resolve(REPO, "crates/cards/catalog.json"), "utf8")) as Record<string, CardDef>;

const SEEDS = Array.from({ length: 200 }, (_, i) => i + 1);

describe("R374 the landing fan is dealt at random", () => {
  it("R374 the pool is every non-token Core card, exactly as the catalog holds it", () => {
    // R374 deals from Core; Classic and Classic+ (R380) share the catalog but not the landing hand.
    const expected = Object.values(CATALOG).filter((def) => !def.token && def.set === "Core");
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

describe("R639 the rotation's pool", () => {
  it("R639 is every non-token card of Core, Classic and Classic+, as the catalog holds it", () => {
    const expected = Object.values(CATALOG).filter(
      (def) => !def.token && (def.set === "Core" || def.set === "Classic" || def.set === "Classic+"),
    );
    expect(ROTATION_POOL).toEqual(expected);
    expect(ROTATION_POOL.length).toBeGreaterThan(FAN_POOL.length);
    expect(new Set(ROTATION_POOL.map((def) => def.set))).toEqual(new Set(["Core", "Classic", "Classic+"]));
  });
});

describe("R639 favouring the cards that print at full size", () => {
  it("R639 a card whose name and text fit the two shortest tiers weighs more than one that needs shrinking, and none weighs 0", () => {
    expect(FEATURE_WEIGHT_PLAIN).toBeGreaterThan(FEATURE_WEIGHT_DENSE);
    expect(FEATURE_WEIGHT_DENSE).toBeGreaterThan(0);
    const weights = ROTATION_POOL.map((def) => featureWeight(def));
    expect(new Set(weights)).toEqual(new Set([FEATURE_WEIGHT_PLAIN, FEATURE_WEIGHT_DENSE]));
    for (const def of ROTATION_POOL) {
      const face = faceModel({ defId: def.id, def, radiant: false });
      const fits = ["s", "m"].includes(nameTier(face.name)) && ["s", "m"].includes(textTier(face.text.full));
      expect(featureWeight(def), def.id).toBe(fits ? FEATURE_WEIGHT_PLAIN : FEATURE_WEIGHT_DENSE);
    }
  });

  it("R639 a weighted draw picks plain cards more often, and still reaches every card", () => {
    const plain = ROTATION_POOL.filter((def) => featureWeight(def) === FEATURE_WEIGHT_PLAIN);
    const dense = ROTATION_POOL.filter((def) => featureWeight(def) === FEATURE_WEIGHT_DENSE);
    expect(plain.length).toBeGreaterThan(0);
    expect(dense.length).toBeGreaterThan(0);

    const random = seeded(99);
    const drawn = new Map<string, number>();
    const draws = 40_000;
    for (let i = 0; i < draws; i += 1) {
      const def = pickWeighted(ROTATION_POOL, random, featureWeight);
      if (def !== undefined) drawn.set(def.id, (drawn.get(def.id) ?? 0) + 1);
    }
    const average = (cards: readonly CardDef[]): number =>
      cards.reduce((sum, def) => sum + (drawn.get(def.id) ?? 0), 0) / cards.length;
    // Per card, a plain one comes up about FEATURE_WEIGHT_PLAIN / FEATURE_WEIGHT_DENSE times as often.
    const ratio = average(plain) / average(dense);
    expect(ratio).toBeGreaterThan((FEATURE_WEIGHT_PLAIN / FEATURE_WEIGHT_DENSE) * 0.8);
    expect(ratio).toBeLessThan((FEATURE_WEIGHT_PLAIN / FEATURE_WEIGHT_DENSE) * 1.25);
    // Less likely, not excluded: every dense card still came up.
    expect(dense.every((def) => (drawn.get(def.id) ?? 0) > 0)).toBe(true);
  });

  it("R639 a weight that is not above 0, or not a number, counts as 1, so no card is out of reach", () => {
    const [a, b] = ROTATION_POOL;
    if (a === undefined || b === undefined) throw new Error("pool too small");
    const weigh = (def: CardDef): number => (def.id === a.id ? 0 : Number.NaN);
    const seen = new Set<string>();
    const random = seeded(4);
    for (let i = 0; i < 200; i += 1) seen.add(pickWeighted([a, b], random, weigh)?.id ?? "");
    expect(seen).toEqual(new Set([a.id, b.id]));
    expect(pickWeighted([], random)).toBeUndefined();
  });

  it("R639 the draw is the seed's: the same source picks the same card, and the edges of the source stay in range", () => {
    expect(pickWeighted(ROTATION_POOL, seeded(8), featureWeight)?.id).toBe(pickWeighted(ROTATION_POOL, seeded(8), featureWeight)?.id);
    for (const value of [0, 0.999999999, 1]) {
      expect(pickWeighted(ROTATION_POOL, () => value, featureWeight)).toBeDefined();
    }
  });

  it("R639 a deal from the whole pool is still four cards of four rarities with the middle one Radiant", () => {
    for (const seed of SEEDS) {
      const hand = dealLandingFan(seeded(seed), ROTATION_POOL, featureWeight);
      expect(hand, `seed ${String(seed)}`).toHaveLength(FAN_FACES);
      expect(new Set(hand.map(({ def }) => def.rarity)).size).toBe(FAN_FACES);
      expect(hand.map(({ radiant }) => radiant)).toEqual([0, 1, 2, 3].map((at) => at === FAN_RADIANT_AT));
    }
  });
});

describe("R639 one rotation step", () => {
  it("R639 swaps the slot for a card of the same rarity the fan is not showing, keeps the others and the Radiant slot", () => {
    for (const seed of SEEDS) {
      const hand = dealLandingFan(seeded(seed), ROTATION_POOL, featureWeight);
      for (const slot of [0, 1, 2, 3]) {
        const next = rotateFan(hand, slot, seeded(seed + 1000));
        expect(next).toHaveLength(hand.length);
        expect(next[slot]?.def.id).not.toBe(hand[slot]?.def.id);
        expect(next[slot]?.def.rarity).toBe(hand[slot]?.def.rarity);
        expect(next[slot]?.radiant).toBe(hand[slot]?.radiant);
        expect(next.filter((_, at) => at !== slot)).toEqual(hand.filter((_, at) => at !== slot));
        expect(new Set(next.map(({ def }) => def.id)).size).toBe(next.length);
      }
    }
  });

  it("R639 takes the slot modulo the hand, so a counter that keeps rising keeps rotating", () => {
    const hand = dealLandingFan(seeded(5), ROTATION_POOL, featureWeight);
    expect(rotateFan(hand, 5, seeded(6))).toEqual(rotateFan(hand, 1, seeded(6)));
    expect(rotateFan(hand, -1, seeded(6))).toEqual(rotateFan(hand, 3, seeded(6)));
  });

  it("R639 keeps a slot that has nothing to swap in, and an empty hand stays empty", () => {
    const hand = dealLandingFan(seeded(5), ROTATION_POOL, featureWeight);
    // A pool of only the cards on show leaves nothing of any rarity to bring in.
    expect(rotateFan(hand, 0, seeded(1), hand.map(({ def }) => def))).toEqual(hand);
    expect(rotateFan([], 0, seeded(1))).toEqual([]);
  });

  it("R639 over many steps it brings in cards of every set", () => {
    let hand = dealLandingFan(seeded(21), ROTATION_POOL, featureWeight);
    const random = seeded(22);
    const sets = new Set<string>();
    for (let step = 0; step < 200; step += 1) {
      hand = rotateFan(hand, step, random);
      for (const { def } of hand) sets.add(def.set);
    }
    expect(sets).toEqual(new Set(["Core", "Classic", "Classic+"]));
  });
});

describe("R704 swaps below the threshold", () => {
  it("R704 a step among Core's cards keeps the rarities, repeats none, and reaches every card evenly", () => {
    for (const seed of SEEDS) {
      const hand = dealLandingFan(seeded(seed));
      for (const slot of [0, 1, 2, 3]) {
        const next = rotateFan(hand, slot, seeded(seed + 1000), FAN_POOL, EVEN);
        expect(next, `seed ${String(seed)} slot ${String(slot)}`).toHaveLength(hand.length);
        expect(next[slot]?.def.id).not.toBe(hand[slot]?.def.id);
        expect(next[slot]?.def.rarity).toBe(hand[slot]?.def.rarity);
        expect(next[slot]?.def.set).toBe("Core");
        expect(next[slot]?.radiant).toBe(hand[slot]?.radiant);
        expect(next.filter((_, at) => at !== slot)).toEqual(hand.filter((_, at) => at !== slot));
        expect(new Set(next.map(({ def }) => def.id)).size).toBe(next.length);
      }
    }
    // Evenly: every Core card of the slot's rarity arrives, about as often as every other.
    const hand = dealLandingFan(seeded(5));
    const rarity = hand[0]?.def.rarity;
    const pool = FAN_POOL.filter((def) => def.rarity === rarity);
    expect(pool.length).toBeGreaterThan(1);
    const arrivals = new Map<string, number>();
    const random = seeded(6);
    let current = hand;
    for (let step = 0; step < 20_000; step += 1) {
      current = rotateFan(current, 0, random, FAN_POOL, EVEN);
      const id = current[0]?.def.id;
      if (id !== undefined) arrivals.set(id, (arrivals.get(id) ?? 0) + 1);
    }
    expect([...arrivals.keys()].sort()).toEqual(pool.map((def) => def.id).sort());
    const counts = [...arrivals.values()];
    expect(Math.max(...counts) / Math.min(...counts)).toBeLessThan(1.5);
  });
});
