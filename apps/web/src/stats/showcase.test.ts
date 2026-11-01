// R1584000: which stats the floor's showcase shows, and the words for each. R1743000: the other
// twenty-five, the tokens every stat passes over, and the bag the showcase draws its order from.

import catalogJson from "@jackioh/cards/catalog.json";
import type { CardDefs } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { ROTATION_POOL, type RandomSource } from "../routes/landingFan.ts";
import { seeded } from "../test/random.ts";
import { SHOWCASE_MIN_PLAYS } from "./config.ts";
import { EMPTY_STATS, NO_COUNTS, type CardCounters, type PlayerStats } from "./model.ts";
import type { PocketPick } from "./pocketPick.ts";
import {
  EMPTY_BAG,
  SHOWCASE_KEYS,
  nextInBag,
  pocketLine,
  pocketStat,
  showcaseStats,
  type ShowcaseBag,
  type ShowcaseKey,
  type ShowcaseStat,
} from "./showcase.ts";

const CATALOG = catalogJson as unknown as CardDefs;

beforeEach(() => {
  resetLocaleForTests();
});
afterEach(() => {
  resetLocaleForTests();
});

/** A tally of 12 games with each card's counters as given (the rest 0). */
function tally(cards: Record<string, Partial<CardCounters>>, games = 12, over: Partial<PlayerStats> = {}): PlayerStats {
  return {
    ...EMPTY_STATS,
    games,
    wins: 7,
    losses: 4,
    draws: 1,
    ...over,
    cards: Object.fromEntries(Object.entries(cards).map(([id, counts]) => [id, { ...NO_COUNTS, ...counts }])),
  };
}

const PICK: PocketPick = { id: "core-005", playerRate: 0.8, baseRate: 0.2 };

const ALL = tally({
  "core-001": { defeated: 5, allured: 2, played: 9, playedAgainst: 8, destroyed: 6, seen: 12, lethal: 3, sprung: 4, survived: 7 },
  "core-002": { defeated: 1, played: 2 },
});

function keys(stats: PlayerStats, pocket: PocketPick | null = null): ShowcaseKey[] {
  return showcaseStats(stats, pocket, seeded(1)).map((stat) => stat.key);
}

describe("R1584000 showcaseStats", () => {
  it("R1584000 the stats come in table order, each with its top card, title and line", () => {
    const stats = showcaseStats(ALL, PICK, seeded(1)).slice(0, 10);
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
    expect(showcaseStats(EMPTY_STATS, PICK, seeded(1))).toEqual([]);
    expect(showcaseStats(tally({ "core-001": { defeated: 3 } }, 0), PICK, seeded(1))).toEqual([]);
    expect(keys(tally({ "core-001": { played: 2, seen: 5 }, "core-002": { played: 0 } }))).toEqual([
      "faithful",
      "record",
      "unbreakable",
      "familiarFace",
      "explorer",
      "collector",
    ]);
    expect(keys(tally({}))).toEqual([]);
    // The Pocket Pick is left out until its request answers, and when its card is not in the catalog.
    expect(keys(tally({ "core-001": { played: 2 } }), null)).toEqual(["faithful", "unbreakable", "explorer"]);
    expect(keys(tally({ "core-001": { played: 2 } }), { ...PICK, id: "gone-001" })).toEqual(["faithful", "unbreakable", "explorer"]);
  });

  it("R1584000 the card of a counter is the one with its most, a tie going to the lower id", () => {
    const stats = showcaseStats(tally({ "core-009": { played: 4 }, "core-003": { played: 4 }, "core-001": { played: 2 } }), null, seeded(1));
    expect(stats[0]?.cardId).toBe("core-003");
    expect(stats[0]?.line()).toBe("played 4×");
  });

  it("R1584000 a card the catalog no longer holds is passed over for the next", () => {
    const stats = showcaseStats(tally({ "gone-001": { played: 9 }, "core-002": { played: 3 } }), null, seeded(1));
    expect(stats.map((stat) => [stat.key, stat.cardId, stat.line()])).toEqual([
      ["faithful", "core-002", "played 3×"],
      ["unbreakable", "core-002", "played 3×, never destroyed"],
      ["explorer", "core-002", "1 different card played"],
    ]);
    expect(keys(tally({ "gone-001": { played: 9 } }))).toEqual([]);
  });

  it("R1584000 the Pocket Pick line is the ratio to a tenth, or nobody else at a base rate of 0", () => {
    expect(pocketLine({ id: "core-005", playerRate: 0.8, baseRate: 0.2 })).toBe("you play it 4× as often as everyone else");
    expect(pocketLine({ id: "core-005", playerRate: 0.7, baseRate: 0.5 })).toBe("you play it 1.4× as often as everyone else");
    expect(pocketLine({ id: "core-005", playerRate: 0.3, baseRate: 0 })).toBe("nobody else plays it");
  });

  it("R1584000 the words are looked up when called, so a language change reaches a stat already made", async () => {
    const [first] = showcaseStats(tally({ "core-001": { defeated: 5 } }), null, seeded(1));
    expect(first?.title()).toBe("Most Toppled");
    await setLocale("zh-CN");
    expect(first?.title()).toBe("最常击倒");
    expect(first?.line()).toBe("被击败 5 次");
  });
});

const TOKENS = ["core-t-sheep", "core-t-rush", "core-051-1"] as const;

/** Data for every one of the 35 stats, none of it on a token. */
const RICH = tally(
  {
    "core-001": {
      seen: 34,
      played: 12,
      playedAgainst: 9,
      destroyed: 8,
      defeated: 9,
      allured: 2,
      lethal: 3,
      survived: 5,
      damageDealt: 312,
      biggestHit: 14,
      overkill: 9,
      attacks: 41,
      healing: 96,
      drawn: 19,
      radiant: 6,
      playedInWin: 9,
      playedInLoss: 3,
    },
    "core-002": { seen: 20, played: 7, playedAgainst: 11, defeated: 2 },
    "core-003": { seen: 5, playedAgainst: 12 },
    "core-004": { seen: 2, played: 1 },
    "core-041": { seen: 9, played: 7, sprung: 6, playedInWin: 2, playedInLoss: 5 },
  },
  50,
  { bestStreak: 6, longestGame: 23, longestGamePlayed: { "core-002": 3 } },
);

/** RICH, plus three tokens topping every counter a stat reads (one never destroyed, one always, one never played) and one in the longest game. */
const WITH_TOKENS: PlayerStats = {
  ...RICH,
  cards: {
    ...RICH.cards,
    [TOKENS[0]]: { ...NO_COUNTS, seen: 999, played: 999, playedAgainst: 999, defeated: 999, allured: 999, lethal: 999, sprung: 999, survived: 999, damageDealt: 999, biggestHit: 999, overkill: 999, attacks: 999, healing: 999, drawn: 999, radiant: 999, playedInWin: 999 },
    [TOKENS[1]]: { ...NO_COUNTS, seen: 999, played: 999, playedAgainst: 999, destroyed: 999, defeated: 999, sprung: 999, survived: 999, playedInLoss: 999 },
    [TOKENS[2]]: { ...NO_COUNTS, seen: 999, playedAgainst: 999 },
  },
  longestGamePlayed: { [TOKENS[0]]: 9, "core-002": 3 },
};

const seen = (stats: ShowcaseStat[]): [string, string, string][] => stats.map((stat) => [stat.key, stat.cardId, stat.line()]);

function statOf(stats: PlayerStats, key: ShowcaseKey, seed = 1): ShowcaseStat | undefined {
  return showcaseStats(stats, null, seeded(seed)).find((stat) => stat.key === key);
}

describe("R1743000 a token is never a stat's card", () => {
  it("R1743000 the catalog's tokens are tokens, and the test tally's others are not", () => {
    for (const id of TOKENS) expect(CATALOG[id]?.token).toBe(true);
    for (const id of Object.keys(RICH.cards)) expect(CATALOG[id]?.token).toBe(false);
  });

  it("R1743000 a token topping every counter names no stat: each falls through to the top non-token card, and a pick on it is dropped", () => {
    const tokenPick: PocketPick = { id: TOKENS[0], playerRate: 0.9, baseRate: 0.1 };
    const stats = showcaseStats(WITH_TOKENS, tokenPick, seeded(4));
    const without = showcaseStats(RICH, null, seeded(4));
    expect(seen(stats)).toEqual(seen(without));
    expect(stats.map((stat) => stat.key)).toEqual(SHOWCASE_KEYS.filter((key) => key !== "pocketPick"));
    for (const stat of stats) expect(CATALOG[stat.cardId]?.token).toBe(false);
    for (const key of ["faithful", "fallen", "survivor", "record", "toppled", "lastLaugh", "workhorse"] as const) {
      expect(statOf(WITH_TOKENS, key)?.cardId).toBe("core-001");
    }
    expect(pocketStat(WITH_TOKENS, tokenPick)).toEqual([]);
    expect(showcaseStats(WITH_TOKENS, tokenPick, seeded(4)).some((stat) => stat.key === "pocketPick")).toBe(false);
    expect(pocketStat(WITH_TOKENS, PICK).map((stat) => [stat.key, stat.cardId])).toEqual([["pocketPick", "core-005"]]);
    expect(pocketStat(EMPTY_STATS, PICK)).toEqual([]);
    expect(pocketStat(WITH_TOKENS, null)).toEqual([]);
  });

  it("R1743000 a tally of nothing but tokens shows no stat but never throws", () => {
    const only = tally({ [TOKENS[0]]: { seen: 5, played: 5, damageDealt: 9, playedInWin: 6, drawn: 1 }, [TOKENS[1]]: { played: 1 } }, 12, {
      bestStreak: 3,
      longestGame: 9,
      longestGamePlayed: { [TOKENS[0]]: 2 },
    });
    expect(showcaseStats(only, null, seeded(1))).toEqual([]);
  });
});

describe("R1743000 the new stats", () => {
  it("R1743000 all 35 keys show given data, in the order of SHOWCASE_KEYS, each with a title and an animation", () => {
    expect(SHOWCASE_KEYS).toHaveLength(35);
    expect(new Set(SHOWCASE_KEYS).size).toBe(35);
    const stats = showcaseStats(RICH, PICK, seeded(2));
    expect(stats.map((stat) => stat.key)).toEqual([...SHOWCASE_KEYS]);
    expect(stats.slice(10).map((stat) => stat.title())).toEqual([
      "Glass Cannon",
      "Unbreakable",
      "Grudge Match",
      "Never Tried It",
      "Easy Target",
      "The Closer",
      "Hair Trigger",
      "Ride or Die",
      "Familiar Face",
      "One-Hit Wonder",
      "Second Fiddle",
      "Bronze Medal",
      "Explorer",
      "Collector",
      "Workhorse",
      "Heavy Hitter",
      "Overkill",
      "Brawler",
      "Medic",
      "Card Shark",
      "Shine On",
      "Comeback Kid",
      "Jinx",
      "Hot Streak",
      "Marathon",
    ]);
    expect(stats.slice(10).map((stat) => [stat.animation, stat.side])).toEqual([
      ["death", "you"], // Glass Cannon
      ["shield", "you"], // Unbreakable
      ["lunge", "you"], // Grudge Match
      ["lunge", "opponent"], // Never Tried It
      ["death", "opponent"], // Easy Target
      ["lunge", "you"], // The Closer
      ["trap", "you"], // Hair Trigger
      ["shield", "you"], // Ride or Die
      ["radiant", "you"], // Familiar Face
      ["entrance", "you"], // One-Hit Wonder
      ["entrance", "you"], // Second Fiddle
      ["entrance", "you"], // Bronze Medal
      ["entrance", "you"], // Explorer
      ["radiant", "you"], // Collector
      ["lunge", "you"], // Workhorse
      ["lunge", "you"], // Heavy Hitter
      ["lunge", "you"], // Overkill
      ["lunge", "you"], // Brawler
      ["shield", "you"], // Medic
      ["entrance", "you"], // Card Shark
      ["radiant", "you"], // Shine On
      ["radiant", "you"], // Comeback Kid
      ["death", "you"], // Jinx
      ["radiant", "you"], // Hot Streak
      ["entrance", "you"], // Marathon
    ]);
  });

  it("R1743000 a tally with only a seen card shows The Record, Familiar Face and Collector, and a new total alone shows its stat", () => {
    expect(keys(tally({ "core-001": { seen: 1 } }))).toEqual(["record", "familiarFace", "collector"]);
    expect(keys(tally({ "core-001": { seen: 1 } }, 12, { bestStreak: 2 }))).toEqual(["record", "familiarFace", "collector"]);
    expect(keys(tally({ "core-001": { played: 3 } }, 12, { bestStreak: 2, longestGame: 4 }))).toEqual([
      "faithful",
      "unbreakable",
      "explorer",
      "hotStreak",
      "marathon",
    ]);
  });

  it("R1743000 Glass Cannon is the card destroyed most per play of at least SHOWCASE_MIN_PLAYS, a tie to the lower id", () => {
    const base = { "core-009": { played: 10, destroyed: 8 }, "core-003": { played: 10, destroyed: 8 }, "core-002": { played: SHOWCASE_MIN_PLAYS - 1, destroyed: SHOWCASE_MIN_PLAYS - 1 } };
    const found = statOf(tally(base), "glassCannon");
    expect([found?.cardId, found?.line()]).toEqual(["core-003", "destroyed 8 of the 10 times you played it"]);
    // A better share beats a bigger sample, and a count past the plays is capped at them.
    expect(statOf(tally({ ...base, "core-005": { played: SHOWCASE_MIN_PLAYS, destroyed: SHOWCASE_MIN_PLAYS } }), "glassCannon")?.cardId).toBe("core-005");
    expect(statOf(tally({ "core-001": { played: 5, destroyed: 9 } }), "glassCannon")?.line()).toBe("destroyed 5 of the 5 times you played it");
    expect(statOf(tally({ "core-002": base["core-002"] }), "glassCannon")).toBeUndefined();
    expect(statOf(tally({ "core-001": { played: 10 }, "core-002": { destroyed: 4 } }), "glassCannon")).toBeUndefined();
  });

  it("R1743000 Unbreakable is the most played card that was never destroyed", () => {
    const found = statOf(tally({ "core-009": { played: 12 }, "core-003": { played: 12 }, "core-004": { played: 20, destroyed: 1 } }), "unbreakable");
    expect([found?.cardId, found?.line()]).toEqual(["core-003", "played 12×, never destroyed"]);
    expect(statOf(tally({ "core-004": { played: 3, destroyed: 1 }, "core-005": { seen: 4 } }), "unbreakable")).toBeUndefined();
  });

  it("R1743000 Grudge Match is the card with the most of the lesser of the plays by you and by them", () => {
    const found = statOf(
      tally({ "core-009": { played: 7, playedAgainst: 9 }, "core-003": { played: 7, playedAgainst: 12 }, "core-004": { played: 20, playedAgainst: 2 } }),
      "grudgeMatch",
    );
    expect([found?.cardId, found?.line()]).toEqual(["core-003", "you 7×, them 12×"]);
    expect(statOf(tally({ "core-001": { played: 4 }, "core-002": { playedAgainst: 6 } }), "grudgeMatch")).toBeUndefined();
  });

  it("R1743000 Never Tried It is the card faced most that you never played, on the opponent's side", () => {
    const found = statOf(
      tally({ "core-009": { playedAgainst: 11 }, "core-003": { playedAgainst: 11 }, "core-004": { playedAgainst: 30, played: 1 } }),
      "neverTried",
    );
    expect([found?.cardId, found?.line(), found?.side]).toEqual(["core-003", "faced 11×, never played it yourself", "opponent"]);
    expect(statOf(tally({ "core-004": { playedAgainst: 30, played: 1 }, "core-005": { played: 3 } }), "neverTried")).toBeUndefined();
  });

  it("R1743000 Easy Target is the card taken down most per time you faced it, at least SHOWCASE_MIN_PLAYS", () => {
    const base = { "core-009": { playedAgainst: 10, defeated: 9 }, "core-003": { playedAgainst: 10, defeated: 9 }, "core-004": { playedAgainst: SHOWCASE_MIN_PLAYS - 1, defeated: SHOWCASE_MIN_PLAYS - 1 } };
    const found = statOf(tally(base), "easyTarget");
    expect([found?.cardId, found?.line(), found?.side]).toEqual(["core-003", "taken down 9 of the 10 times you faced it", "opponent"]);
    expect(statOf(tally({ "core-004": base["core-004"] }), "easyTarget")).toBeUndefined();
    expect(statOf(tally({ "core-004": { playedAgainst: 12 } }), "easyTarget")).toBeUndefined();
  });

  it("R1743000 The Closer is the card with the most winning blows per play, at least SHOWCASE_MIN_PLAYS", () => {
    const base = { "core-009": { played: 5, lethal: 3 }, "core-003": { played: 5, lethal: 3 }, "core-004": { played: SHOWCASE_MIN_PLAYS - 1, lethal: SHOWCASE_MIN_PLAYS - 1 } };
    const found = statOf(tally(base), "closer");
    expect([found?.cardId, found?.line()]).toEqual(["core-003", "the winning blow in 3 of 5 plays"]);
    expect(statOf(tally({ "core-004": base["core-004"] }), "closer")).toBeUndefined();
    expect(statOf(tally({ "core-004": { played: 9 } }), "closer")).toBeUndefined();
  });

  it("R1743000 Hair Trigger is the Trap that sprang most per play, at least SHOWCASE_MIN_PLAYS", () => {
    const base = { "core-060": { played: 7, sprung: 6 }, "core-041": { played: 7, sprung: 6 }, "core-085": { played: SHOWCASE_MIN_PLAYS - 1, sprung: SHOWCASE_MIN_PLAYS - 1 } };
    const found = statOf(tally(base), "hairTrigger");
    expect([found?.cardId, found?.line(), found?.animation]).toEqual(["core-041", "sprung 6 of the 7 times you set it", "trap"]);
    expect(statOf(tally({ "core-085": base["core-085"] }), "hairTrigger")).toBeUndefined();
    expect(statOf(tally({ "core-041": { played: 7 } }), "hairTrigger")).toBeUndefined();
  });

  it("R1743000 Ride or Die is the card that was still standing most per play, at least SHOWCASE_MIN_PLAYS", () => {
    const base = { "core-009": { played: 6, survived: 5 }, "core-003": { played: 6, survived: 5 }, "core-004": { played: SHOWCASE_MIN_PLAYS - 1, survived: SHOWCASE_MIN_PLAYS - 1 } };
    const found = statOf(tally(base), "rideOrDie");
    expect([found?.cardId, found?.line()]).toEqual(["core-003", "still standing 5 of the 6 times you played it"]);
    expect(statOf(tally({ "core-004": base["core-004"] }), "rideOrDie")).toBeUndefined();
    expect(statOf(tally({ "core-004": { played: 9 } }), "rideOrDie")).toBeUndefined();
  });

  it("R1743000 Familiar Face is the card seen in most games, worded with the games, and singular for one", () => {
    const found = statOf(tally({ "core-009": { seen: 34 }, "core-003": { seen: 34 }, "core-004": { seen: 2 } }, 50), "familiarFace");
    expect([found?.cardId, found?.line()]).toEqual(["core-003", "in 34 of your 50 games"]);
    expect(statOf(tally({ "core-004": { seen: 1 } }, 1), "familiarFace")?.line()).toBe("in 1 of your 1 game");
    expect(statOf(tally({ "core-004": { played: 1 } }), "familiarFace")).toBeUndefined();
  });

  it("R1743000 One-Hit Wonder is a card played exactly once, drawn with the showcase's random source, and no draw with none", () => {
    const stats = tally({ "core-009": { played: 1 }, "core-003": { played: 1 }, "core-004": { played: 2 }, "core-006": { playedAgainst: 1 }, [TOKENS[0]]: { played: 1 } });
    for (const seed of [1, 2, 3, 4, 5, 6]) {
      const mirror = seeded(seed);
      const ids = ["core-003", "core-009"];
      const found = statOf(stats, "oneHitWonder", seed);
      expect([found?.cardId, found?.line()]).toEqual([ids[Math.floor(mirror() * ids.length)], "played once. Just once."]);
    }
    expect(new Set([1, 2, 3, 4, 5, 6, 7, 8].map((seed) => statOf(stats, "oneHitWonder", seed)?.cardId))).toEqual(new Set(["core-003", "core-009"]));
    expect(statOf(tally({ "core-004": { played: 2 }, [TOKENS[0]]: { played: 1 } }), "oneHitWonder")).toBeUndefined();
  });

  it("R1743000 Second Fiddle and Bronze Medal are the #2 and #3 most played cards, a tie to the lower id, a token passed over", () => {
    const stats = tally({
      "core-009": { played: 12 },
      "core-004": { played: 7 },
      "core-003": { played: 7 },
      "core-005": { played: 5 },
      [TOKENS[0]]: { played: 50 },
    });
    const second = statOf(stats, "secondFiddle");
    const third = statOf(stats, "bronzeMedal");
    expect([second?.cardId, second?.line()]).toEqual(["core-003", "your #2, played 7×"]);
    expect([third?.cardId, third?.line()]).toEqual(["core-004", "your #3, played 7×"]);
    expect(statOf(tally({ "core-009": { played: 12 } }), "secondFiddle")).toBeUndefined();
    expect(statOf(tally({ "core-009": { played: 12 }, "core-003": { played: 7 } }), "bronzeMedal")).toBeUndefined();
    expect(statOf(tally({ "core-009": { played: 12 }, "core-003": { played: 7 } }), "secondFiddle")?.cardId).toBe("core-003");
    expect(statOf(tally({ "core-009": { played: 12 }, [TOKENS[0]]: { played: 7 } }), "secondFiddle")).toBeUndefined();
  });

  it("R1743000 Explorer is a card played, drawn with the random source, worded with how many different cards were played", () => {
    const stats = tally({ "core-001": { played: 2 }, "core-002": { played: 1 }, "core-003": { seen: 4 }, [TOKENS[0]]: { played: 3 } });
    for (const seed of [1, 2, 3, 4, 5, 6]) {
      const mirror = seeded(seed);
      mirror(); // One-Hit Wonder's draw: core-002 was played exactly once.
      const ids = ["core-001", "core-002"];
      const found = statOf(stats, "explorer", seed);
      expect([found?.cardId, found?.line()]).toEqual([ids[Math.floor(mirror() * ids.length)], "2 different cards played"]);
    }
    expect(statOf(tally({ "core-001": { played: 3 } }), "explorer")?.line()).toBe("1 different card played");
    expect(statOf(tally({ "core-001": { seen: 3 }, [TOKENS[0]]: { played: 3 } }), "explorer")).toBeUndefined();
  });

  it("R1743000 Collector is a card seen from the shipped sets, drawn with the random source, worded out of their non-token cards", () => {
    const stats = tally({ "core-001": { seen: 2 }, "core-002": { seen: 1 }, "core-003": { played: 4 }, "gone-001": { seen: 3 }, [TOKENS[0]]: { seen: 5 } });
    for (const seed of [1, 2, 3, 4, 5, 6]) {
      const mirror = seeded(seed);
      mirror(); // Explorer's draw: core-003 is the one card played.
      const ids = ["core-001", "core-002"];
      const found = statOf(stats, "collector", seed);
      expect([found?.cardId, found?.line()]).toEqual([ids[Math.floor(mirror() * ids.length)], `seen 2 of ${String(ROTATION_POOL.length)} cards`]);
    }
    expect(ROTATION_POOL.every((def) => !def.token)).toBe(true);
    expect(statOf(tally({ [TOKENS[0]]: { seen: 5 }, "gone-001": { seen: 3 } }), "collector")).toBeUndefined();
  });

  it("R1743000 the random stats draw once each and only with a candidate, in the order One-Hit Wonder, Explorer, Collector", () => {
    const counting = (): { random: RandomSource; calls: () => number } => {
      const inner = seeded(9);
      let n = 0;
      return {
        random: () => {
          n += 1;
          return inner();
        },
        calls: () => n,
      };
    };
    const none = counting();
    showcaseStats(tally({ "core-001": { defeated: 4 } }), null, none.random);
    expect(none.calls()).toBe(0);
    const some = counting();
    showcaseStats(tally({ "core-001": { played: 2, seen: 3 } }), null, some.random);
    expect(some.calls()).toBe(2);
    const all = counting();
    showcaseStats(tally({ "core-001": { played: 1, seen: 3 } }), null, all.random);
    expect(all.calls()).toBe(3);
    expect(seen(showcaseStats(RICH, null, seeded(5)))).toEqual(seen(showcaseStats(RICH, null, seeded(5))));
  });

  it.each([
    ["workhorse", "damageDealt", 312, "dealt 312 damage"],
    ["heavyHitter", "biggestHit", 14, "hit for 14 once"],
    ["overkill", "overkill", 9, "hit 9 harder than it needed to"],
    ["brawler", "attacks", 41, "attacked 41×"],
    ["medic", "healing", 96, "healed 96"],
    ["cardShark", "drawn", 19, "drawn 19×"],
    ["shineOn", "radiant", 6, "made Radiant 6×"],
  ] as const)("R1743000 %s is the card with the most %s, a tie to the lower id, and left out with none", (key, counter, n, line) => {
    const found = statOf(tally({ "core-009": { [counter]: n }, "core-003": { [counter]: n }, "core-004": { [counter]: n - 1 } }), key);
    expect([found?.cardId, found?.line()]).toEqual(["core-003", line]);
    expect(statOf(tally({ "core-004": { seen: 2, played: 3 } }), key)).toBeUndefined();
    expect(statOf(tally({ [TOKENS[0]]: { [counter]: n } }), key)).toBeUndefined();
  });

  it("R1743000 Comeback Kid and Jinx are the highest win and loss shares of the games a card was played in, at least SHOWCASE_MIN_PLAYS", () => {
    const base = {
      "core-009": { playedInWin: 9, playedInLoss: 2 },
      "core-003": { playedInWin: 9, playedInLoss: 2 },
      "core-004": { playedInWin: SHOWCASE_MIN_PLAYS - 1 },
      "core-005": { playedInWin: 2, playedInLoss: 7 },
      "core-006": { playedInWin: 2, playedInLoss: 7 },
      "core-007": { playedInLoss: SHOWCASE_MIN_PLAYS - 1 },
    };
    const comeback = statOf(tally(base), "comebackKid");
    const jinx = statOf(tally(base), "jinx");
    expect([comeback?.cardId, comeback?.line()]).toEqual(["core-003", "won 9 of the 11 games you played it in"]);
    expect([jinx?.cardId, jinx?.line()]).toEqual(["core-005", "lost 7 of the 9 games you played it in"]);
    expect([comeback?.animation, jinx?.animation]).toEqual(["radiant", "death"]);
    expect(statOf(tally({ "core-004": base["core-004"], "core-007": base["core-007"] }), "comebackKid")).toBeUndefined();
    expect(statOf(tally({ "core-004": base["core-004"], "core-007": base["core-007"] }), "jinx")).toBeUndefined();
    expect(statOf(tally({ "core-001": { playedInLoss: 8 } }), "comebackKid")).toBeUndefined();
    expect(statOf(tally({ "core-001": { playedInWin: 8 } }), "jinx")).toBeUndefined();
  });

  it("R1743000 Hot Streak is the most played card and the best streak, singular for one win, left out with no streak or no card", () => {
    const found = statOf(tally({ "core-009": { played: 4 }, "core-003": { played: 4 } }, 12, { bestStreak: 6 }), "hotStreak");
    expect([found?.cardId, found?.line()]).toEqual(["core-003", "best streak: 6 wins"]);
    expect(statOf(tally({ "core-003": { played: 4 } }, 12, { bestStreak: 1 }), "hotStreak")?.line()).toBe("best streak: 1 win");
    expect(statOf(tally({ "core-003": { played: 4 } }, 12, { streak: 3 }), "hotStreak")).toBeUndefined();
    expect(statOf(tally({ "core-003": { seen: 4 } }, 12, { bestStreak: 6 }), "hotStreak")).toBeUndefined();
  });

  it("R1743000 Marathon is the most played card of the longest game, else the most played overall, and the turns", () => {
    const longest = { longestGame: 23, longestGamePlayed: { "core-009": 4, "core-003": 4, "core-002": 1 } };
    const base = { "core-005": { played: 30 }, "core-009": { played: 8 } };
    const found = statOf(tally(base, 12, longest), "marathon");
    expect([found?.cardId, found?.line(), found?.animation]).toEqual(["core-003", "longest game: 23 turns", "entrance"]);
    // No plays in that game: the card played most overall. A token in it is passed over.
    expect(statOf(tally(base, 12, { longestGame: 23 }), "marathon")?.cardId).toBe("core-005");
    expect(statOf(tally(base, 12, { longestGame: 23, longestGamePlayed: { [TOKENS[0]]: 9 } }), "marathon")?.cardId).toBe("core-005");
    expect(statOf(tally(base, 12, { longestGame: 23, longestGamePlayed: { [TOKENS[0]]: 9, "core-002": 1 } }), "marathon")?.cardId).toBe("core-002");
    expect(statOf(tally(base, 12, { longestGame: 1 }), "marathon")?.line()).toBe("longest game: 1 turn");
    expect(statOf(tally(base, 12, { longestGamePlayed: { "core-002": 1 } }), "marathon")).toBeUndefined();
    expect(statOf(tally({ "core-005": { seen: 3 } }, 12, { longestGame: 9 }), "marathon")).toBeUndefined();
  });

  it("R1743000 the new stats' words are looked up when called, in Chinese too", async () => {
    const found = statOf(RICH, "glassCannon");
    expect(found?.title()).toBe("Glass Cannon");
    await setLocale("zh-CN");
    expect(found?.title()).toBe("玻璃大炮");
    expect(found?.line()).toBe("打出 12 次，被摧毁 8 次");
  });
});

describe("R1743000 the bag", () => {
  const A = SHOWCASE_KEYS.slice(0, 5);

  it("R1743000 a round is every key once, in the order the seed shuffles them, and each round again", () => {
    let bag: ShowcaseBag = EMPTY_BAG;
    const random = seeded(7);
    const mirror = seeded(7);
    const shown: ShowcaseKey[] = [];
    for (let i = 0; i < A.length * 4; i += 1) {
      bag = nextInBag(bag, A, random);
      expect(bag.shown).not.toBeNull();
      shown.push(bag.shown as ShowcaseKey);
    }
    for (let round = 0; round < 4; round += 1) {
      const one = shown.slice(round * A.length, (round + 1) * A.length);
      expect([...one].sort()).toEqual([...A].sort());
    }
    // The first round is the seed's own shuffle (a Fisher-Yates, as the fan's).
    const expected = [...A];
    for (let i = expected.length - 1; i > 0; i -= 1) {
      const j = Math.min(i, Math.floor(mirror() * (i + 1)));
      [expected[i], expected[j]] = [expected[j] as ShowcaseKey, expected[i] as ShowcaseKey];
    }
    expect(shown.slice(0, A.length)).toEqual(expected);
  });

  it("R1743000 a refill never starts with the stat shown, whatever the seed, and never repeats it twice in a row", () => {
    for (let seed = 1; seed <= 50; seed += 1) {
      const random = seeded(seed);
      let bag: ShowcaseBag = EMPTY_BAG;
      let last: ShowcaseKey | null = null;
      for (let i = 0; i < 40; i += 1) {
        bag = nextInBag(bag, A, random);
        expect(bag.shown).not.toBe(last);
        last = bag.shown;
      }
      const spent: ShowcaseBag = { shown: "toppled", left: [] };
      expect(nextInBag(spent, ["toppled", "coveted"], seeded(seed)).shown).toBe("coveted");
    }
  });

  it("R1743000 a bag keeps only the keys still held, refills when none are left, and a lone or no stat is the whole bag", () => {
    const bag: ShowcaseBag = { shown: "toppled", left: ["coveted", "faithful", "survivor"] };
    expect(nextInBag(bag, ["toppled", "faithful", "survivor"], seeded(1))).toEqual({ shown: "faithful", left: ["survivor"] });
    expect(nextInBag(bag, ["toppled", "faithful"], seeded(1))).toEqual({ shown: "faithful", left: [] });
    // Nothing left of the round held: a new round of those held, and the shown one not first.
    const refilled = nextInBag(bag, ["toppled", "record", "fallen"], seeded(1));
    expect(refilled.shown).not.toBe("toppled");
    expect([refilled.shown, ...refilled.left].sort()).toEqual(["fallen", "record", "toppled"]);
    // One stat is always itself, and none is nothing, with no draw either way.
    const never: RandomSource = () => {
      throw new Error("no draw wanted");
    };
    expect(nextInBag({ shown: "toppled", left: [] }, ["toppled"], never)).toEqual({ shown: "toppled", left: [] });
    expect(nextInBag(EMPTY_BAG, [], never)).toEqual({ shown: null, left: [] });
  });
});
