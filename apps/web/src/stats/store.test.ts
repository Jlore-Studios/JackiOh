// The device's copy of the statistics (SPEC R639): kept like the tutorial's progress, tolerant of
// anything in storage, and never throwing.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PLAYER_STATS_KEY, PLAYER_STATS_VERSION } from "./config.ts";
import { EMPTY_LOG, EMPTY_STATS, logSeen, logWith } from "./model.ts";
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
    expect(stored.cards).toEqual({ "core-001": { seen: 1, played: 1, playedAgainst: 0, destroyed: 0, defeated: 0 } });
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
    expect(parsed.cards).toEqual({ "core-001": { seen: 2, played: 0, playedAgainst: 0, destroyed: 0, defeated: 4 } });
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
