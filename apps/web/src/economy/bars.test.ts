// The bar's pure parts (#1658): the sweeps a bar plays from its points before to its points after,
// each sweep's share of the time, which bars an opening moved, the reward each fill reveals and the
// sting a reward plays.

import { describe, expect, it } from "vitest";

import type { OpeningBarsAnswer } from "../net/api.ts";
import {
  FILL_MS,
  barMoved,
  fillReward,
  fraction,
  segmentMs,
  segments,
  stingOf,
  strongestSting,
  type RevealItem,
} from "./bars.ts";

/** An opening where the card bar paid once and the cosmetic bar paid an Epic token and then a pack. */
function answer(over: Partial<OpeningBarsAnswer> = {}): OpeningBarsAnswer {
  return {
    card: { before: 18, after: 3, size: 20, pays: 1 },
    cosmetic: { before: 100, after: 20, size: 150, pays: 2 },
    cosmeticPaid: ["epic", "cosmeticPack"],
    ...over,
  };
}

describe("#1658 the sweeps a bar plays", () => {
  it("a bar that paid once sweeps to full, then restarts at 0 and sweeps to its value", () => {
    expect(segments(20, 3, 18, 1)).toEqual([
      { from: 18, to: 20, burst: true },
      { from: 0, to: 3, burst: false },
    ]);
  });

  it("each further fill is a whole bar of its own", () => {
    expect(segments(20, 3, 18, 3)).toEqual([
      { from: 18, to: 20, burst: true },
      { from: 0, to: 20, burst: true },
      { from: 0, to: 20, burst: true },
      { from: 0, to: 3, burst: false },
    ]);
  });

  it("a bar that paid nothing is one sweep from where it was to where it is", () => {
    expect(segments(20, 12, 5, 0)).toEqual([{ from: 5, to: 12, burst: false }]);
  });

  it("a bar that paid and landed on 0 ends by restarting there", () => {
    expect(segments(20, 0, 18, 1)).toEqual([
      { from: 18, to: 20, burst: true },
      { from: 0, to: 0, burst: false },
    ]);
  });

  it("points past either end of the bar are held to it, and a part of a fill is no fill", () => {
    expect(segments(20, 25, -4, 0)).toEqual([{ from: 0, to: 20, burst: false }]);
    expect(segments(20, 3, 18, 1.9)).toHaveLength(2);
    expect(segments(20, 3, 18, -1)).toEqual([{ from: 18, to: 3, burst: false }]);
  });

  it("a sweep takes its share of the bar's time, and none for no distance or no bar", () => {
    expect(segmentMs({ from: 0, to: 20, burst: true }, 20)).toBe(FILL_MS);
    expect(segmentMs({ from: 10, to: 20, burst: true }, 20)).toBe(FILL_MS / 2);
    expect(segmentMs({ from: 12, to: 12, burst: false }, 20)).toBe(0);
    expect(segmentMs({ from: 0, to: 20, burst: false }, 0)).toBe(0);
  });

  it("a fraction of the bar is held between empty and full", () => {
    expect(fraction(5, 20)).toBe(0.25);
    expect(fraction(-5, 20)).toBe(0);
    expect(fraction(50, 20)).toBe(1);
    expect(fraction(5, 0)).toBe(0);
  });
});

describe("#1658 an opening's bars and rewards", () => {
  it("a bar moved when it paid or its points changed", () => {
    expect(barMoved({ before: 5, after: 5, size: 20, pays: 0 })).toBe(false);
    expect(barMoved({ before: 5, after: 9, size: 20, pays: 0 })).toBe(true);
    // A bar that paid and kept the same remainder moved all the same (R1305000).
    expect(barMoved({ before: 5, after: 5, size: 20, pays: 1 })).toBe(true);
  });

  it("the card bar's every fill reveals a card token", () => {
    const bars = answer({ card: { before: 1, after: 2, size: 20, pays: 2 } });
    expect(fillReward(bars, "card", 1)).toEqual({ key: "card-1", kind: "cardToken", label: "Card token" });
    expect(fillReward(bars, "card", 2)?.key).toBe("card-2");
    expect(fillReward(bars, "card", 3)).toBeNull();
    expect(fillReward(bars, "card", 0)).toBeNull();
  });

  it("the cosmetic bar's fill reveals the step the opening paid for it", () => {
    const bars = answer({ cosmeticPaid: ["rare", "epic", "cosmeticPack"] });
    expect(fillReward(bars, "cosmetic", 1)).toEqual({
      key: "cosmetic-1",
      kind: "cosmeticToken",
      label: "Rare token",
      rarity: "Rare",
    });
    expect(fillReward(bars, "cosmetic", 2)).toEqual({
      key: "cosmetic-2",
      kind: "cosmeticToken",
      label: "Epic token",
      rarity: "Epic",
    });
    expect(fillReward(bars, "cosmetic", 3)).toEqual({ key: "cosmetic-3", kind: "cosmeticPack", label: "Cosmetic pack" });
    expect(fillReward(bars, "cosmetic", 4)).toBeNull();
  });

  it("a reward's sting is its rarity's when it names a printed one, else its kind's", () => {
    const item = (over: Partial<RevealItem>): RevealItem => ({ key: "k", kind: "jlore", label: "Jlore", ...over });
    expect(stingOf(item({ rarity: "Epic" }))).toBe("Epic");
    expect(stingOf(item({ rarity: "Mythic" }))).toBe("Mythic");
    expect(stingOf(item({ kind: "cosmeticPack" }))).toBe("Epic");
    expect(stingOf(item({ kind: "jlore" }))).toBe("Common");
    // A word that is no printed rarity changes nothing.
    expect(stingOf(item({ kind: "cardToken", rarity: "Shiny" }))).toBe("Rare");
  });

  it("rewards shown at once play the strongest sting, and Common for none", () => {
    const rare: RevealItem = { key: "a", kind: "cosmeticToken", label: "Rare token", rarity: "Rare" };
    const epic: RevealItem = { key: "b", kind: "cosmeticToken", label: "Epic token", rarity: "Epic" };
    expect(strongestSting([rare, epic, rare])).toBe("Epic");
    expect(strongestSting([rare])).toBe("Rare");
    expect(strongestSting([])).toBe("Common");
  });
});
