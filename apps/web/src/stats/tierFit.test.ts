// Tests for a card's tier fit on /stats (SPEC §9.11, R3184000).

import { describe, expect, it } from "vitest";

import { tierBounds, tierFit } from "./tierFit.ts";

/** The server's band (R3022000): 45% to 55%. */
const BAND = { bandLow: 0.45, bandHigh: 0.55 };

describe("R3184000 tier fit", () => {
  it("R3184000 splits the band into quarters from the top, with S and F open beyond it", () => {
    const close = (tier: "S" | "A" | "B" | "C" | "D" | "F"): [number, number] => {
      const { low, high } = tierBounds(tier, BAND);
      return [Number(low.toFixed(4)), Number(high.toFixed(4))];
    };
    expect(close("S")).toEqual([0.55, Number.POSITIVE_INFINITY]);
    expect(close("A")).toEqual([0.525, 0.55]);
    expect(close("B")).toEqual([0.5, 0.525]);
    expect(close("C")).toEqual([0.475, 0.5]);
    expect(close("D")).toEqual([0.45, 0.475]);
    expect(close("F")).toEqual([Number.NEGATIVE_INFINITY, 0.45]);
  });

  it("R3184000 is the share of the range inside the tier's quarter, to a whole percent", () => {
    // B is 50% to 52.5%: a range of 49% to 54% has 2.5 of its 5 points inside.
    expect(tierFit("B", 0.49, 0.54, BAND)).toBe(50);
    // Wholly inside.
    expect(tierFit("B", 0.505, 0.52, BAND)).toBe(100);
    // Wholly outside.
    expect(tierFit("A", 0.4, 0.5, BAND)).toBe(0);
    // 47.5% to 50% of 46% to 52%: 2.5 of 6 points, 41.7%, shown 42%.
    expect(tierFit("C", 0.46, 0.52, BAND)).toBe(42);
  });

  it("R3184000 reads S's stretch as the band's top and up, and F's as its bottom and down", () => {
    // 53% to 63%: 8 of 10 points above 55%.
    expect(tierFit("S", 0.53, 0.63, BAND)).toBe(80);
    expect(tierFit("S", 0.56, 0.7, BAND)).toBe(100);
    // 40% to 46%: 5 of 6 points below 45%.
    expect(tierFit("F", 0.4, 0.46, BAND)).toBe(83);
    expect(tierFit("F", 0.3, 0.44, BAND)).toBe(100);
  });

  it("R3184000 a range of no width is all inside its tier or all outside it", () => {
    expect(tierFit("B", 0.51, 0.51, BAND)).toBe(100);
    expect(tierFit("A", 0.51, 0.51, BAND)).toBe(0);
    expect(tierFit("S", 0.6, 0.6, BAND)).toBe(100);
    expect(tierFit("F", 0.6, 0.6, BAND)).toBe(0);
  });

  it("R3184000 is nothing with no tier, no range or no band", () => {
    expect(tierFit(null, 0.49, 0.54, BAND)).toBeNull();
    expect(tierFit(undefined, 0.49, 0.54, BAND)).toBeNull();
    expect(tierFit("B", null, 0.54, BAND)).toBeNull();
    expect(tierFit("B", 0.49, undefined, BAND)).toBeNull();
    expect(tierFit("B", 0.49, 0.54, undefined)).toBeNull();
  });
});
