// Tests for a card's tier fit on /stats (SPEC §9.11, R3184001).

import { describe, expect, it } from "vitest";

import type { PublicCardStat, PublicStatsTiers } from "../net/api.ts";
import { cardTierFit, tierBand, tierFit } from "./tierFit.ts";

/** R3022000's band, 45% to 55%: A to D are 2.5 points each. */
const LOW = 0.45;
const HIGH = 0.55;
const TIERS: PublicStatsTiers = { center: 0, spread: 0.05, cards: 0, bandLow: LOW, bandHigh: HIGH };

/** A card with enough games, tier B, its Adjusted range 49% to 53%. */
const CARD: PublicCardStat = {
  id: "core-001",
  name: "Spark Pup",
  cost: 1,
  rarity: "Common",
  set: "core",
  games: 50,
  winRate: 0.6,
  adjustedWinRate: 0.51,
  adjustedLow: 0.49,
  adjustedHigh: 0.53,
  tier: "B",
  playRate: 0.5,
  hasEnoughGames: true,
};

describe("R3184001 tier fit", () => {
  it("R3184001 a tier's band: S and F open beyond the band's edges, A to D its quarters from the top", () => {
    expect(tierBand("S", LOW, HIGH)).toEqual([HIGH, Infinity]);
    expect(tierBand("F", LOW, HIGH)).toEqual([-Infinity, LOW]);
    for (const [tier, from, to] of [
      ["A", 0.525, 0.55],
      ["B", 0.5, 0.525],
      ["C", 0.475, 0.5],
      ["D", 0.45, 0.475],
    ] as const) {
      const [low, high] = tierBand(tier, LOW, HIGH);
      expect(low).toBeCloseTo(from, 12);
      expect(high).toBeCloseTo(to, 12);
    }
  });

  it("R3184001 the share of the range inside its tier's band", () => {
    // B: 50% to 52.5% of a 49% to 53% range, 2.5 of 4 points.
    expect(tierFit("B", 0.49, 0.53, LOW, HIGH)).toBeCloseTo(0.625, 12);
    // Wholly inside, and wholly outside (an A whose range sits in B).
    expect(tierFit("B", 0.505, 0.52, LOW, HIGH)).toBeCloseTo(1, 12);
    expect(tierFit("A", 0.5, 0.52, LOW, HIGH)).toBe(0);
    // An A above the band whose range still reaches 50%: only 52.5% to 55% of 49% to 63% is A's.
    expect(tierFit("A", 0.49, 0.63, LOW, HIGH)).toBeCloseTo(0.025 / 0.14, 12);
  });

  it("R3184001 S and F are open-ended: all of the range past the band's edge is theirs", () => {
    // S: 55% up, so 6 of 53% to 61%'s 8 points; a range wholly above the band is wholly S.
    expect(tierFit("S", 0.53, 0.61, LOW, HIGH)).toBeCloseTo(0.75, 12);
    expect(tierFit("S", 0.56, 0.7, LOW, HIGH)).toBe(1);
    // F: 45% down, so 7 of 38% to 46%'s 8 points.
    expect(tierFit("F", 0.38, 0.46, LOW, HIGH)).toBeCloseTo(0.875, 12);
    expect(tierFit("F", 0.3, 0.44, LOW, HIGH)).toBe(1);
  });

  it("R3184001 a range of no width is wholly in its tier or wholly out", () => {
    expect(tierFit("B", 0.51, 0.51, LOW, HIGH)).toBe(1);
    expect(tierFit("B", 0.53, 0.53, LOW, HIGH)).toBe(0);
    expect(tierFit("S", 0.6, 0.6, LOW, HIGH)).toBe(1);
    expect(tierFit("F", 0.46, 0.46, LOW, HIGH)).toBe(0);
  });

  it("R3184001 a card's fit, and none below the floor, with no tier, no range or no band", () => {
    expect(cardTierFit(CARD, TIERS)).toBeCloseTo(0.625, 12);
    expect(cardTierFit({ ...CARD, hasEnoughGames: false }, TIERS)).toBeNull();
    expect(cardTierFit({ ...CARD, tier: null }, TIERS)).toBeNull();
    expect(cardTierFit({ ...CARD, tier: undefined }, TIERS)).toBeNull();
    expect(cardTierFit({ ...CARD, adjustedLow: null }, TIERS)).toBeNull();
    expect(cardTierFit({ ...CARD, adjustedHigh: undefined }, TIERS)).toBeNull();
    expect(cardTierFit(CARD, undefined)).toBeNull();
  });
});
