// The device's copy of the statistics (SPEC R639): kept like the tutorial's progress, tolerant of
// anything in storage, and never throwing.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PLAYER_STATS_KEY, PLAYER_STATS_VERSION } from "./config.ts";
import { EMPTY_LOG, EMPTY_STATS, NO_COUNTS, logMax, logSeen, logWith } from "./model.ts";
import {
  dropPlayerStatsCache,
  parsePlayerStats,
  readPlayerStats,
  recordGame,
  resetPlayerStats,
  subscribePlayerStats,
} from "./store.ts";

beforeEach(() => {
  window.localStorage.clear();
  dropPlayerStatsCache();
});

afterEach(() => {
  vi.restoreAllMocks();
  dropPlayerStatsCache();
});

const log = logWith(logSeen(EMPTY_LOG, ["core-001"]), "played", "core-001");

describe("R639 the stored statistics", () => {
  it("R639 an empty device has no statistics", () => {
    expect(readPlayerStats()).toBe(EMPTY_STATS);
  });

  it("R639 a game is added to the totals and written to localStorage in the stored shape", () => {
    const stats = recordGame(log, "win");
    expect(stats).toMatchObject({ games: 1, wins: 1 });
    expect(readPlayerStats()).toBe(stats);
    const stored = JSON.parse(window.localStorage.getItem(PLAYER_STATS_KEY) ?? "null") as Record<string, unknown>;
    expect(stored).toMatchObject({ v: PLAYER_STATS_VERSION, games: 1, wins: 1, losses: 0, draws: 0 });
    expect(stored.cards).toEqual({ "core-001": { ...NO_COUNTS, seen: 1, played: 1, playedInWin: 1 } });
  });

  it("R639 a game is added to the totals as stored, so two tabs finishing a game each add theirs", () => {
    recordGame(log, "win");
    // Another tab's write the first never saw: it stands in storage with a game of its own.
    window.localStorage.setItem(
      PLAYER_STATS_KEY,
      JSON.stringify({ v: PLAYER_STATS_VERSION, games: 5, wins: 2, losses: 3, draws: 0, cards: {} }),
    );
    expect(recordGame(log, "loss")).toMatchObject({ games: 6, wins: 2, losses: 4 });
  });

  it("R639 subscribers hear of a finished game and of a reset, once each", () => {
    const heard = vi.fn();
    const stop = subscribePlayerStats(heard);
    recordGame(log, "draw");
    expect(heard).toHaveBeenCalledTimes(1);
    resetPlayerStats();
    expect(heard).toHaveBeenCalledTimes(2);
    expect(readPlayerStats()).toBe(EMPTY_STATS);
    stop();
    recordGame(log, "win");
    expect(heard).toHaveBeenCalledTimes(2);
  });

  it("R639 another tab's write is read back and announced", () => {
    const heard = vi.fn();
    const stop = subscribePlayerStats(heard);
    const value = JSON.stringify({ v: PLAYER_STATS_VERSION, games: 7, wins: 7, losses: 0, draws: 0, cards: {} });
    window.dispatchEvent(new StorageEvent("storage", { key: PLAYER_STATS_KEY, newValue: value }));
    expect(heard).toHaveBeenCalledTimes(1);
    expect(readPlayerStats().games).toBe(7);
    // A different key is none of its business.
    window.dispatchEvent(new StorageEvent("storage", { key: "other", newValue: "x" }));
    expect(heard).toHaveBeenCalledTimes(1);
    stop();
  });
});

describe("R639 a tolerant reader", () => {
  it.each([
    ["not JSON", "{nope"],
    ["the wrong version", JSON.stringify({ v: 99, games: 3 })],
    ["an array", "[]"],
    ["null", "null"],
    ["a number", "7"],
  ])("R639 %s reads as no statistics", (_name, raw) => {
    expect(parsePlayerStats(raw)).toBe(EMPTY_STATS);
  });

  it("R639 a count that is not a whole, positive number reads as 0, and an all-zero card is dropped", () => {
    const parsed = parsePlayerStats({
      v: PLAYER_STATS_VERSION,
      games: -2,
      wins: 1.5,
      losses: "3",
      draws: 2,
      cards: {
        "core-001": { seen: 2, played: "x", playedAgainst: -1, destroyed: 1.2, defeated: 4 },
        "core-002": { seen: 0 },
        "core-003": "nope",
      },
    });
    expect(parsed).toMatchObject({ games: 0, wins: 0, losses: 0, draws: 2 });
    expect(parsed.cards).toEqual({ "core-001": { ...NO_COUNTS, seen: 2, defeated: 4 } });
  });

  it("R1582000 statistics stored before the new counters load with them at 0, and a game adds to them", () => {
    window.localStorage.setItem(
      PLAYER_STATS_KEY,
      JSON.stringify({
        v: PLAYER_STATS_VERSION,
        games: 3,
        wins: 2,
        losses: 1,
        draws: 0,
        cards: { "core-001": { seen: 3, played: 4, playedAgainst: 0, destroyed: 1, defeated: 2 } },
      }),
    );
    expect(readPlayerStats().cards["core-001"]).toEqual({ ...NO_COUNTS, seen: 3, played: 4, destroyed: 1, defeated: 2 });
    let log = logWith(EMPTY_LOG, "survived", "core-001");
    log = logWith(log, "sprung", "core-020");
    expect(recordGame(log, "win")).toMatchObject({ games: 4, wins: 3 });
    const kept = readPlayerStats().cards["core-001"];
    expect(kept).toEqual({ ...NO_COUNTS, seen: 3, played: 4, destroyed: 1, defeated: 2, survived: 1 });
    const stored = JSON.parse(window.localStorage.getItem(PLAYER_STATS_KEY) ?? "null") as {
      cards: Record<string, unknown>;
    };
    expect(stored.cards["core-020"]).toEqual({ ...NO_COUNTS, sprung: 1 });
  });

  it("R1582000 a hand-edited new counter that is not a whole positive number reads as 0, and a card with only a new counter is kept", () => {
    const parsed = parsePlayerStats({
      v: PLAYER_STATS_VERSION,
      games: 1,
      wins: 1,
      losses: 0,
      draws: 0,
      cards: {
        "core-001": { allured: "2", lethal: -1, sprung: 1.5, survived: 3 },
        "core-002": { sprung: 2 },
        "core-003": { allured: 0, lethal: null },
      },
    });
    expect(parsed.cards).toEqual({
      "core-001": { ...NO_COUNTS, survived: 3 },
      "core-002": { ...NO_COUNTS, sprung: 2 },
    });
  });

  it("R1582000 the clear empties the new counters", () => {
    recordGame(logWith(EMPTY_LOG, "allured", "core-050"), "loss");
    expect(resetPlayerStats()).toBe(EMPTY_STATS);
    expect(readPlayerStats()).toBe(EMPTY_STATS);
    expect(parsePlayerStats(window.localStorage.getItem(PLAYER_STATS_KEY))).toEqual(EMPTY_STATS);
  });

  it("R1743000 an older value reads the new counters and totals as 0, a bad one as 0, and the clear empties them", () => {
    window.localStorage.setItem(
      PLAYER_STATS_KEY,
      JSON.stringify({ v: PLAYER_STATS_VERSION, games: 3, wins: 2, losses: 1, draws: 0, cards: { "core-001": { seen: 3, played: 4 } } }),
    );
    expect(readPlayerStats()).toMatchObject({ streak: 0, bestStreak: 0, longestGame: 0, longestGamePlayed: {} });
    expect(readPlayerStats().cards["core-001"]).toEqual({ ...NO_COUNTS, seen: 3, played: 4 });

    const bad = parsePlayerStats({
      v: PLAYER_STATS_VERSION,
      games: 1,
      wins: 1,
      losses: 0,
      draws: 0,
      cards: { "core-001": { damageDealt: "9", biggestHit: -1, overkill: 1.5, attacks: 4, healing: null, playedInWin: 2 } },
      streak: -1,
      bestStreak: "7",
      longestGame: 2.5,
      longestGamePlayed: { "core-001": 2, "core-002": "x", "core-003": -4, "core-004": 0 },
    });
    expect(bad).toMatchObject({ streak: 0, bestStreak: 0, longestGame: 0, longestGamePlayed: { "core-001": 2 } });
    expect(bad.cards["core-001"]).toEqual({ ...NO_COUNTS, attacks: 4, playedInWin: 2 });
    expect(parsePlayerStats({ v: PLAYER_STATS_VERSION, games: 1, longestGamePlayed: [3] }).longestGamePlayed).toEqual({});

    // A won game stores every new counter and total under the same key, and reads them back.
    const log = { ...logMax(logWith(EMPTY_LOG, "drawn", "core-001"), "overkill", "core-001", 5), turns: 12 };
    recordGame({ ...log, played: { "core-001": 2 } }, "win");
    dropPlayerStatsCache();
    expect(readPlayerStats()).toMatchObject({ streak: 1, bestStreak: 1, longestGame: 12, longestGamePlayed: { "core-001": 2 } });
    expect(readPlayerStats().cards["core-001"]).toMatchObject({ drawn: 1, overkill: 5, played: 6, playedInWin: 1 });

    expect(resetPlayerStats()).toBe(EMPTY_STATS);
    dropPlayerStatsCache();
    expect(readPlayerStats()).toEqual(EMPTY_STATS);
    expect(readPlayerStats()).toMatchObject({ streak: 0, bestStreak: 0, longestGame: 0, longestGamePlayed: {} });
  });

  it("R2649000 an older value reads the new counters as 0, a bad one as 0, a game stores them and the clear empties them", () => {
    window.localStorage.setItem(
      PLAYER_STATS_KEY,
      JSON.stringify({ v: PLAYER_STATS_VERSION, games: 3, wins: 2, losses: 1, draws: 0, cards: { "core-001": { seen: 3, played: 4, drawn: 2 } } }),
    );
    expect(readPlayerStats().cards["core-001"]).toEqual({ ...NO_COUNTS, seen: 3, played: 4, drawn: 2 });

    const bad = parsePlayerStats({
      v: PLAYER_STATS_VERSION,
      games: 1,
      cards: { "core-001": { tempExpired: "9", kills: -1, biggestKill: 1.5, manaSaved: null, created: 3, launched: 2, froze: {} } },
    });
    expect(bad.cards["core-001"]).toEqual({ ...NO_COUNTS, created: 3, launched: 2 });
    // A card with only a new counter is kept.
    expect(parsePlayerStats({ v: PLAYER_STATS_VERSION, games: 1, cards: { "core-002": { shattered: 1 } } }).cards["core-002"]).toEqual({
      ...NO_COUNTS,
      shattered: 1,
    });

    // A game stores each new counter under the same key and version, and reads it back; biggestKill keeps its max.
    dropPlayerStatsCache();
    window.localStorage.clear();
    const game = logMax(logWith(logWith(EMPTY_LOG, "kills", "core-001", 2), "counterspells", "core-041"), "biggestKill", "core-001", 6);
    recordGame({ ...game, createdIds: ["m1"], attackers: { mine: 2 } }, "win");
    recordGame(logMax(logWith(EMPTY_LOG, "kills", "core-001"), "biggestKill", "core-001", 3), "loss");
    dropPlayerStatsCache();
    const stored = JSON.parse(window.localStorage.getItem(PLAYER_STATS_KEY) ?? "null") as { v: number; cards: Record<string, Record<string, number>> };
    expect(stored.v).toBe(PLAYER_STATS_VERSION);
    expect(stored.cards["core-001"]).toMatchObject({ kills: 3, biggestKill: 6 });
    expect(window.localStorage.getItem(PLAYER_STATS_KEY)).not.toContain("createdIds");
    expect(window.localStorage.getItem(PLAYER_STATS_KEY)).not.toContain("attackers");
    expect(readPlayerStats().cards["core-001"]).toMatchObject({ kills: 3, biggestKill: 6 });
    expect(readPlayerStats().cards["core-041"]).toMatchObject({ counterspells: 1 });

    expect(resetPlayerStats()).toBe(EMPTY_STATS);
    dropPlayerStatsCache();
    expect(readPlayerStats()).toEqual(EMPTY_STATS);
  });

  it("R639 storage that throws leaves the in-memory totals in force", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(readPlayerStats()).toBe(EMPTY_STATS);
    expect(recordGame(log, "win").games).toBe(1);
    expect(readPlayerStats().games).toBe(1);
  });
});
