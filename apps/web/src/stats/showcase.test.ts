// R1584000: which stats the floor's showcase shows, and the words for each.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { EMPTY_STATS, NO_COUNTS, type CardCounters, type PlayerStats } from "./model.ts";
import type { PocketPick } from "./pocketPick.ts";
import { pocketLine, showcaseStats, type ShowcaseKey } from "./showcase.ts";

beforeEach(() => {
  resetLocaleForTests();
});
afterEach(() => {
  resetLocaleForTests();
});

/** A tally of 12 games with each card's counters as given (the rest 0). */
function tally(cards: Record<string, Partial<CardCounters>>, games = 12): PlayerStats {
  return {
    games,
    wins: 7,
    losses: 4,
    draws: 1,
    cards: Object.fromEntries(Object.entries(cards).map(([id, counts]) => [id, { ...NO_COUNTS, ...counts }])),
  };
}

const PICK: PocketPick = { id: "core-005", playerRate: 0.8, baseRate: 0.2 };

const ALL = tally({
  "core-001": { defeated: 5, allured: 2, played: 9, playedAgainst: 8, destroyed: 6, seen: 12, lethal: 3, sprung: 4, survived: 7 },
  "core-002": { defeated: 1, played: 2 },
});

function keys(stats: PlayerStats, pocket: PocketPick | null = null): ShowcaseKey[] {
  return showcaseStats(stats, pocket).map((stat) => stat.key);
}

describe("R1584000 showcaseStats", () => {
  it("R1584000 the stats come in table order, each with its top card, title and line", () => {
    const stats = showcaseStats(ALL, PICK);
    expect(stats.map((stat) => stat.key)).toEqual([
      "toppled",
      "coveted",
      "pocketPick",
      "faithful",
      "nemesis",
      "fallen",
      "record",
      "lastLaugh",
      "springloaded",
      "survivor",
    ]);
    expect(stats.map((stat) => stat.title())).toEqual([
      "Most Toppled",
      "Most Coveted",
      "Pocket Pick",
      "Old Faithful",
      "Arch-Nemesis",
      "Fallen Hero",
      "The Record",
      "Last Laugh",
      "Springloaded",
      "Survivor",
    ]);
    expect(stats.map((stat) => stat.line())).toEqual([
      "taken down 5×",
      "stolen 2×",
      "you play it 4× as often as everyone else",
      "played 9×",
      "played against you 8×",
      "lost 6×",
      "12 games: 7 won, 4 lost, 1 drawn (58% won)",
      "dealt the winning blow 3×",
      "sprung 4×",
      "survived to the end 7×",
    ]);
    expect(stats.map((stat) => stat.cardId)).toEqual(Array.from({ length: 10 }, (_unused, i) => (i === 2 ? "core-005" : "core-001")));
    expect(stats.map((stat) => [stat.animation, stat.side])).toEqual([
      ["death", "opponent"],
      ["allure", "you"],
      ["trap", "you"],
      ["entrance", "you"],
      ["lunge", "opponent"],
      ["death", "you"],
      ["radiant", "you"],
      ["lunge", "you"],
      ["trap", "you"],
      ["shield", "you"],
    ]);
  });

  it("R1584000 a stat with no data is skipped, and no games show nothing", () => {
    expect(showcaseStats(EMPTY_STATS, PICK)).toEqual([]);
    expect(showcaseStats(tally({ "core-001": { defeated: 3 } }, 0), PICK)).toEqual([]);
    expect(keys(tally({ "core-001": { played: 2, seen: 5 }, "core-002": { played: 0 } }))).toEqual(["faithful", "record"]);
    expect(keys(tally({}))).toEqual([]);
    // The Pocket Pick is left out until its request answers, and when its card is not in the catalog.
    expect(keys(tally({ "core-001": { played: 2 } }), null)).toEqual(["faithful"]);
    expect(keys(tally({ "core-001": { played: 2 } }), { ...PICK, id: "gone-001" })).toEqual(["faithful"]);
  });

  it("R1584000 the card of a counter is the one with its most, a tie going to the lower id", () => {
    const stats = showcaseStats(tally({ "core-009": { played: 4 }, "core-003": { played: 4 }, "core-001": { played: 2 } }), null);
    expect(stats[0]?.cardId).toBe("core-003");
    expect(stats[0]?.line()).toBe("played 4×");
  });

  it("R1584000 a card the catalog no longer holds is passed over for the next", () => {
    const stats = showcaseStats(tally({ "gone-001": { played: 9 }, "core-002": { played: 3 } }), null);
    expect(stats.map((stat) => [stat.key, stat.cardId, stat.line()])).toEqual([["faithful", "core-002", "played 3×"]]);
    expect(keys(tally({ "gone-001": { played: 9 } }))).toEqual([]);
  });

  it("R1584000 the Pocket Pick line is the ratio to a tenth, or nobody else at a base rate of 0", () => {
    expect(pocketLine({ id: "core-005", playerRate: 0.8, baseRate: 0.2 })).toBe("you play it 4× as often as everyone else");
    expect(pocketLine({ id: "core-005", playerRate: 0.7, baseRate: 0.5 })).toBe("you play it 1.4× as often as everyone else");
    expect(pocketLine({ id: "core-005", playerRate: 0.3, baseRate: 0 })).toBe("nobody else plays it");
  });

  it("R1584000 the words are looked up when called, so a language change reaches a stat already made", async () => {
    const [first] = showcaseStats(tally({ "core-001": { defeated: 5 } }), null);
    expect(first?.title()).toBe("Most Toppled");
    await setLocale("zh-CN");
    expect(first?.title()).toBe("最常击倒");
    expect(first?.line()).toBe("被击败 5 次");
  });
});
