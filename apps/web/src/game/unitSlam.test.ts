import { describe, expect, it } from "vitest";

import { slamTier, slamTierOfTotal, slamTotal } from "./unitSlam.ts";

const plain = { armor: 0, indestructible: false, tributes: 0 };

describe("unit slam tiers", () => {
  it("splits the tiers at 5/6, 11/12, 17/18, 27/28 and 39/40", () => {
    expect([5, 6, 11, 12, 17, 18, 27, 28, 39, 40].map(slamTierOfTotal)).toEqual([
      "tiny",
      "small",
      "small",
      "medium",
      "medium",
      "large",
      "large",
      "huge",
      "huge",
      "massive",
    ]);
    expect(slamTierOfTotal(0)).toBe("tiny");
    expect(slamTierOfTotal(99)).toBe("massive");
  });

  it("weighs Armor 2, Indestructible 4 and each Tribute 4", () => {
    const armored = { attack: 3, health: 3, armor: 2, indestructible: false, tributes: 1 };
    expect(slamTotal(armored)).toBe(14);
    expect(slamTier(armored)).toBe("medium");
    const titan = { attack: 3, health: 3, armor: 0, indestructible: true, tributes: 2 };
    expect(slamTotal(titan)).toBe(18);
    expect(slamTier(titan)).toBe("large");
  });

  it("reads plain stats and never goes below zero", () => {
    expect(slamTier({ attack: 1, health: 1, ...plain })).toBe("tiny");
    expect(slamTotal({ attack: -2, health: Number.NaN, ...plain })).toBe(0);
  });
});
