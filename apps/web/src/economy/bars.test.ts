// The bar's pure parts (#1658): the sweeps a bar plays from its points before to its points after,
// each sweep's share of the time, the flights a spent slot's points take into the bars and their time
// (R1953000), the reward each fill reveals and the sting a reward plays.

import { describe, expect, it } from "vitest";

import type { OpeningBarsAnswer } from "../net/api.ts";
import {
  BAR_FLIGHT_MS,
  BAR_FLIGHT_STAGGER_MS,
  FILL_MS,
  fillReward,
  flightsMs,
  flightsOf,
  fraction,
  segmentMs,
  segments,
  sizeAt,
  stingOf,
  strongestSting,
  type RevealItem,
} from "./bars.ts";

/**
 * An opening whose spent slots gave 20 and 200 points to each bar: the card bar paid once and the
 * cosmetic bar paid an Epic token and then a pack.
 */
function answer(over: Partial<OpeningBarsAnswer> = {}): OpeningBarsAnswer {
  return {
    card: { before: 18, after: 38, size: 200, pays: 1 },
    cosmetic: { before: 100, after: 20, size: 150, pays: 2 },
    cosmeticPaid: ["epic", "cosmeticPack"],
    slotPoints: [0, 20, 0, 200],
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

describe("#2728 a bar whose steps differ in size", () => {
  it("R2728000 each fill sweeps to its own step's size and the last rests on the step after", () => {
    expect(segments(50, 10, 60, 2, [75, 100, 50])).toEqual([
      { from: 60, to: 75, burst: true },
      { from: 0, to: 100, burst: true },
      { from: 0, to: 10, burst: false },
    ]);
    // Points past either end are held to the bar of their own step.
    expect(segments(50, 60, 80, 1, [75, 50])).toEqual([
      { from: 75, to: 75, burst: true },
      { from: 0, to: 50, burst: false },
    ]);
    // A step's size is the answer's; with none for it (an answer kept from before #2728), the one size.
    expect(sizeAt(50, [75, 100, 50], 0)).toBe(75);
    expect(sizeAt(50, [75, 100, 50], 1)).toBe(100);
    expect(sizeAt(50, [75], 1)).toBe(50);
    expect(sizeAt(50, undefined, 1)).toBe(50);
    expect(segments(50, 10, 40, 2)).toEqual([
      { from: 40, to: 50, burst: true },
      { from: 0, to: 50, burst: true },
      { from: 0, to: 10, burst: false },
    ]);
  });
});

describe("#1953 the flights of an opening", () => {
  const source = (index: number): string => `slot-${String(index)}`;

  it("R1953000 one flight for each slot the server gave points, in slot order, with its points, rarity and slot", () => {
    expect(flightsOf(answer(), ["Common", "Legendary", "Rare", "Mythic"], source)).toEqual([
      { key: "flight-1", points: 20, rarity: "Legendary", source: "slot-1" },
      { key: "flight-3", points: 200, rarity: "Mythic", source: "slot-3" },
    ]);
    const unspent = answer({
      card: { before: 18, after: 18, size: 200, pays: 0 },
      cosmetic: { before: 100, after: 100, size: 150, pays: 0 },
      cosmeticPaid: [],
      slotPoints: [0, 0, 0, 0],
    });
    expect(flightsOf(unspent, ["Common", "Legendary", "Rare", "Mythic"], source), "no spent slot, no flight").toEqual([]);
  });

  it("R1953000 the flights take one stagger each after the first, then the last one's flight, and none take nothing", () => {
    expect(flightsMs(0)).toBe(0);
    expect(flightsMs(1)).toBe(BAR_FLIGHT_MS);
    expect(flightsMs(3)).toBe(2 * BAR_FLIGHT_STAGGER_MS + BAR_FLIGHT_MS);
  });
});

describe("#1658 an opening's bars and rewards", () => {
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

  it("R2728000 a Legendary step's fill reveals a Legendary token", () => {
    const bars = answer({ cosmeticPaid: ["rare", "epic", "legendary"] });
    const legendary = fillReward(bars, "cosmetic", 3);
    expect(legendary).toEqual({ key: "cosmetic-3", kind: "cosmeticToken", label: "Legendary token", rarity: "Legendary" });
    expect(stingOf(legendary!)).toBe("Legendary");
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
