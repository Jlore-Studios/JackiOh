// The landing fan's deal (landingFan.ts, R374): a random hand on every visit, from the real catalog,
// in the fixed hand's shape. The source of randomness is passed in, so each case here deals from a
// seeded one and gets the same hand every run.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { nameTier, textTier } from "../cards/fit.ts";
import { FULL_ART_NAMES, isFullArt } from "../cards/fullArt.ts";
import { faceModel } from "../cards/model.ts";
import { faceLines, overLimit, shortFormOf } from "../cards/shortText.ts";
import { FEATURE_WEIGHT_DENSE, FEATURE_WEIGHT_PLAIN, FULL_ART_WEIGHT_FACTOR } from "../stats/config.ts";
import { seeded } from "../test/random.ts";
import {
  EVEN,
  FAN_CARD_MAX_PX,
  FAN_FACES,
  FAN_POOL,
  FAN_RADIANT_AT,
  FAN_TEXT_LIMIT,
  ROTATION_POOL,
  dealLandingFan,
  fanPool,
  favouringFullArt,
  featureWeight,
  fitsTheFan,
  pickWeighted,
  rotateFan,
  rotationPool,
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
  it("R639 is every non-token card of the shipped sets, Meditative included since v0.3.5, as the catalog holds it", () => {
    const expected = Object.values(CATALOG).filter(
      (def) => !def.token && (def.set === "Core" || def.set === "Classic" || def.set === "Classic+" || def.set === "Meditative"),
    );
    expect(ROTATION_POOL).toEqual(expected);
    expect(ROTATION_POOL.length).toBeGreaterThan(FAN_POOL.length);
    expect(new Set(ROTATION_POOL.map((def) => def.set))).toEqual(new Set(["Core", "Classic", "Classic+", "Meditative"]));
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
      // R1017002: a hero's card prints its art and no words in the fan, so it counts as plain.
      const plain = fits || isFullArt(def.name);
      expect(featureWeight(def), def.id).toBe(plain ? FEATURE_WEIGHT_PLAIN : FEATURE_WEIGHT_DENSE);
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
    expect(sets).toEqual(new Set(["Core", "Classic", "Classic+", "Meditative"]));
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

const HEROES = ROTATION_POOL.filter((def) => FULL_ART_NAMES.has(def.name));
const RARITIES = ["Common", "Rare", "Epic", "Legendary", "Mythic"] as const;

describe("R1017002 the fan deals only cards that print whole on it", () => {
  it("R1017002 a card fits when both its faces print full art, or their whole text with no short form within FAN_TEXT_LIMIT", () => {
    expect(FAN_TEXT_LIMIT).toEqual({ charsPerLine: 25, lines: 5 });
    for (const fullArt of [true, false]) {
      for (const def of ROTATION_POOL) {
        const expected = [false, true].every((radiant) => {
          if (fullArt && isFullArt(def.name)) return true;
          const text = faceModel({ defId: def.id, def, radiant }).text.full;
          return shortFormOf(def.id, radiant) === null && faceLines(text, FAN_TEXT_LIMIT.charsPerLine) <= FAN_TEXT_LIMIT.lines;
        });
        expect(fitsTheFan(def, fullArt), `${def.id} with full art ${fullArt ? "on" : "off"}`).toBe(expected);
      }
      expect(fanPool(fullArt)).toEqual(FAN_POOL.filter((def) => fitsTheFan(def, fullArt)));
      expect(rotationPool(fullArt)).toEqual(ROTATION_POOL.filter((def) => fitsTheFan(def, fullArt)));
    }
  });

  it("R1017002 a short form, a text over the limit and a Radiant face over it each keep a card out; a hero's full-art card is in", () => {
    const byId = new Map(ROTATION_POOL.map((def) => [def.id, def]));
    const card = (id: string): CardDef => {
      const def = byId.get(id);
      if (def === undefined) throw new Error(`no ${id}`);
      return def;
    };
    const text = (id: string, radiant: boolean): string => faceModel({ defId: id, def: card(id), radiant }).text.full;
    // Reach the Summit and Combo-Index print short forms; League of Losers' text runs past 5 lines of 25;
    // Recycling Initiative's base face fits and its Radiant face does not.
    for (const id of ["meditative-008", "core-093", "classicplus-019", "core-039"]) expect(fitsTheFan(card(id), true), id).toBe(false);
    expect(overLimit(text("classicplus-019", false), FAN_TEXT_LIMIT)).toBe(true);
    expect(overLimit(text("core-039", false), FAN_TEXT_LIMIT)).toBe(false);
    expect(overLimit(text("core-039", true), FAN_TEXT_LIMIT)).toBe(true);
    // Every hero's card is in while full art is on; off, each is held to its text, and Gary the
    // Gambler's coin flips run past the limit.
    expect(HEROES).toHaveLength(FULL_ART_NAMES.size);
    for (const def of HEROES) {
      expect(fitsTheFan(def, true), def.name).toBe(true);
      expect(fanPool(true), def.name).toContain(def);
      const words = [false, true].every((radiant) => !overLimit(text(def.id, radiant), FAN_TEXT_LIMIT));
      expect(fitsTheFan(def, false), def.name).toBe(words);
    }
    expect(fitsTheFan(card("core-004"), false)).toBe(false);
  });

  it("R1017002 the limit is measured at landing.css's widest fan card", () => {
    const sheet = readFileSync(resolve(REPO, "apps/web/src/routes/landing.css"), "utf8");
    const widths = [...sheet.matchAll(/--fan-card-w: clamp\(\d+px, [\d.]+v[wh], (\d+)px\)/g)].map((match) => Number(match[1]));
    expect(widths.length).toBeGreaterThan(0);
    expect(Math.max(...widths)).toBe(FAN_CARD_MAX_PX);
  });

  it("R1017002 every rarity keeps at least two cards in both pools, so the fan deals four rarities and every slot can swap", () => {
    for (const fullArt of [true, false]) {
      for (const pool of [fanPool(fullArt), rotationPool(fullArt)]) {
        for (const rarity of RARITIES) {
          expect(pool.filter((def) => def.rarity === rarity).length, `${rarity}, full art ${fullArt ? "on" : "off"}`).toBeGreaterThanOrEqual(2);
        }
      }
      for (const seed of SEEDS) {
        const hand = dealLandingFan(seeded(seed), fanPool(fullArt), favouringFullArt(EVEN, fullArt));
        expect(hand).toHaveLength(FAN_FACES);
        for (const { def } of hand) expect(fitsTheFan(def, fullArt), def.id).toBe(true);
      }
    }
  });

  it("R1017002 a hero's full-art card weighs FULL_ART_WEIGHT_FACTOR times as much while full art is on, and as any card while it is off", () => {
    expect(FULL_ART_WEIGHT_FACTOR).toBeGreaterThan(1);
    const other = FAN_POOL.find((def) => !isFullArt(def.name));
    if (other === undefined) throw new Error("no other card");
    for (const def of HEROES) {
      expect(favouringFullArt(EVEN, true)(def), def.name).toBe(FULL_ART_WEIGHT_FACTOR);
      expect(favouringFullArt(EVEN, false)(def), def.name).toBe(1);
      expect(favouringFullArt(featureWeight, true)(def), def.name).toBe(FEATURE_WEIGHT_PLAIN * FULL_ART_WEIGHT_FACTOR);
    }
    expect(favouringFullArt(EVEN, true)(other)).toBe(1);
    expect(favouringFullArt(featureWeight, true)(other)).toBe(featureWeight(other));
    // Drawn among the Core Commons that fit, the heroes' share grows by about the factor.
    const commons = fanPool(true).filter((def) => def.rarity === "Common");
    const heroes = commons.filter((def) => isFullArt(def.name)).length;
    const random = seeded(9);
    let drawn = 0;
    const DRAWS = 20_000;
    for (let i = 0; i < DRAWS; i += 1) {
      const def = pickWeighted(commons, random, favouringFullArt(EVEN, true));
      if (def !== undefined && isFullArt(def.name)) drawn += 1;
    }
    const share = (heroes * FULL_ART_WEIGHT_FACTOR) / (commons.length - heroes + heroes * FULL_ART_WEIGHT_FACTOR);
    expect(drawn / DRAWS).toBeCloseTo(share, 1);
  });
});
