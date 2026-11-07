import { describe, expect, it } from "vitest";

import { DAMAGE_FEEL, damageFeel, damageTier, PLAY_IMPACT_TIERS, PLAY_TIERS_ASCENDING, playTier } from "./damageFeel.ts";

describe("damage feel", () => {
  it("classifies the requested boundary hits", () => {
    expect([1, 5, 8, 15, 25].map(damageTier)).toEqual(["tiny", "normal", "moderate", "big", "giga"]);
  });

  it("keeps prevented damage tiny and exposes all tuning from one config", () => {
    expect(damageTier(0)).toBe("tiny");
    expect(damageFeel(25)).toBe(DAMAGE_FEEL.giga);
    expect(DAMAGE_FEEL.moderate.crowd).toBe("ooh");
    expect(DAMAGE_FEEL.big.shakePx).toBe(6);
  });
});

describe("R3346000 a play's weight by the mana it cost", () => {
  it("R3346000 a cost of 0 to 2 is light, 3 to 5 medium, 6 to 8 heavy and 9 or more massive", () => {
    expect([0, 2, 3, 5, 6, 8, 9, 20].map(playTier)).toEqual(["light", "light", "medium", "medium", "heavy", "heavy", "massive", "massive"]);
  });

  it("R3346000 a cost that is not a finite, non-negative number is light", () => {
    expect([Number.NaN, -1, Number.NEGATIVE_INFINITY].map(playTier)).toEqual(["light", "light", "light"]);
    // A cost with no end is no cost to weigh: it does not read as the heaviest play.
    expect(playTier(Number.POSITIVE_INFINITY)).toBe("light");
  });

  it("R3346000 the tiers ascend in cost and in weight, every one named and light starting at zero", () => {
    expect(PLAY_TIERS_ASCENDING).toEqual(["light", "medium", "heavy", "massive"]);
    expect(PLAY_IMPACT_TIERS.light.min).toBe(0);
    for (let i = 1; i < PLAY_TIERS_ASCENDING.length; i += 1) {
      const below = PLAY_IMPACT_TIERS[PLAY_TIERS_ASCENDING[i - 1] ?? "light"];
      const tier = PLAY_IMPACT_TIERS[PLAY_TIERS_ASCENDING[i] ?? "light"];
      expect(tier.min).toBeGreaterThan(below.min);
      expect(tier.weight).toBeGreaterThan(below.weight);
    }
    for (const tier of PLAY_TIERS_ASCENDING) {
      expect(PLAY_IMPACT_TIERS[tier].weight).toBeGreaterThan(0);
      expect(PLAY_IMPACT_TIERS[tier].weight).toBeLessThanOrEqual(1);
      expect(playTier(PLAY_IMPACT_TIERS[tier].min), `a cost of ${String(PLAY_IMPACT_TIERS[tier].min)} is ${tier}`).toBe(tier);
    }
  });
});
