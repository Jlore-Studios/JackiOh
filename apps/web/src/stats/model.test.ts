// The statistics' arithmetic (SPEC R639): pure functions over a game's log and the device's totals.

import { describe, expect, it } from "vitest";

import {
  EMPTY_LOG,
  EMPTY_STATS,
  NO_COUNTS,
  addGame,
  logIsEmpty,
  logSeen,
  logWith,
  outcomeFor,
  topCards,
  winPercent,
  formatPlayedDelta,
} from "./model.ts";

describe("R639 a game's log", () => {
  it("R639 a card is seen once per game however many views showed it", () => {
    let log = logSeen(EMPTY_LOG, ["core-001", "core-002"]);
    log = logSeen(log, ["core-002", "core-003"]);
    expect(log.seen).toEqual(["core-001", "core-002", "core-003"]);
    // A view that shows nothing new is the very same log, so a re-render copies nothing.
    expect(logSeen(log, ["core-001"])).toBe(log);
  });

  it("R639 plays, losses and takedowns count each time they happen", () => {
    let log = logWith(EMPTY_LOG, "played", "core-001");
    log = logWith(log, "played", "core-001");
    log = logWith(log, "playedAgainst", "core-002");
    expect(log.played).toEqual({ "core-001": 2 });
    expect(log.playedAgainst).toEqual({ "core-002": 1 });
    expect(logIsEmpty(EMPTY_LOG)).toBe(true);
    expect(logIsEmpty(log)).toBe(false);
  });

  it("R639 a finished game reads win, loss or draw from the viewer's seat", () => {
    expect(outcomeFor("p1", "p1")).toBe("win");
    expect(outcomeFor("p2", "p1")).toBe("loss");
    expect(outcomeFor("draw", "p2")).toBe("draw");
  });
});

describe("R639 the device's totals", () => {
  it("R639 a game adds one to the games and to its outcome, and its log to the cards", () => {
    let log = logSeen(EMPTY_LOG, ["core-001", "core-002"]);
    log = logWith(log, "played", "core-001");
    log = logWith(log, "destroyed", "core-001");
    log = logWith(log, "defeated", "core-002");
    const once = addGame(EMPTY_STATS, log, "win");
    expect(once).toMatchObject({ games: 1, wins: 1, losses: 0, draws: 0 });
    expect(once.cards["core-001"]).toEqual({ seen: 1, played: 1, playedAgainst: 0, destroyed: 1, defeated: 0, allured: 0, lethal: 0, sprung: 0, survived: 0 });
    expect(once.cards["core-002"]).toEqual({ seen: 1, played: 0, playedAgainst: 0, destroyed: 0, defeated: 1, allured: 0, lethal: 0, sprung: 0, survived: 0 });

    const twice = addGame(once, log, "loss");
    expect(twice).toMatchObject({ games: 2, wins: 1, losses: 1 });
    expect(twice.cards["core-001"]?.seen).toBe(2);
    // The totals it started from are untouched: the fold returns a new value.
    expect(once.games).toBe(1);
    expect(addGame(twice, EMPTY_LOG, "draw")).toMatchObject({ games: 3, draws: 1 });
  });

  it("R639 the win percentage is whole, and absent before the first game", () => {
    expect(winPercent(EMPTY_STATS)).toBeNull();
    expect(winPercent({ ...EMPTY_STATS, games: 3, wins: 2 })).toBe(67);
  });

  it("R639 the favourites list is highest first, by id among equals, never lists a zero, and stops at the limit", () => {
    const stats = {
      ...EMPTY_STATS,
      cards: {
        "core-003": { ...NO_COUNTS, seen: 1, played: 2 },
        "core-001": { ...NO_COUNTS, seen: 1, played: 2 },
        "core-002": { ...NO_COUNTS, seen: 1, played: 5 },
        "core-004": { ...NO_COUNTS, seen: 9 },
      },
    };
    expect(topCards(stats, "played", 3)).toEqual([
      { id: "core-002", count: 5 },
      { id: "core-001", count: 2 },
      { id: "core-003", count: 2 },
    ]);
    expect(topCards(stats, "played", 1)).toEqual([{ id: "core-002", count: 5 }]);
    expect(topCards(stats, "destroyed", 3)).toEqual([]);
    expect(topCards(stats, "played", 0)).toEqual([]);
  });
});

describe("R1582000 the new counters", () => {
  it("R1582000 Allure's takings, lethal blows, Traps fired and survivors count in a game's log and fold into the card's totals", () => {
    let log = logWith(EMPTY_LOG, "allured", "core-050");
    log = logWith(log, "lethal", "core-001");
    log = logWith(log, "sprung", "core-020");
    log = logWith(log, "sprung", "core-020");
    log = logWith(log, "survived", "core-001");
    const stats = addGame(EMPTY_STATS, log, "win");
    expect(stats.cards["core-001"]).toEqual({ ...NO_COUNTS, lethal: 1, survived: 1 });
    expect(stats.cards["core-020"]).toEqual({ ...NO_COUNTS, sprung: 2 });
    expect(stats.cards["core-050"]).toEqual({ ...NO_COUNTS, allured: 1 });
    expect(topCards(stats, "sprung", 3)).toEqual([{ id: "core-020", count: 2 }]);
  });

  it("R1582000 each new counter alone makes a log non-empty, and every one starts at 0", () => {
    for (const counter of ["allured", "lethal", "sprung", "survived"] as const) {
      expect(logIsEmpty(logWith(EMPTY_LOG, counter, "core-001"))).toBe(false);
    }
    expect(NO_COUNTS).toEqual({
      seen: 0,
      played: 0,
      playedAgainst: 0,
      destroyed: 0,
      defeated: 0,
      allured: 0,
      lethal: 0,
      sprung: 0,
      survived: 0,
    });
    expect(EMPTY_LOG).toMatchObject({ allured: {}, lethal: {}, sprung: {}, survived: {} });
  });
});

describe("R377 formatPlayedDelta", () => {
  it("R377 formats signed percentage points and returns null for null", () => {
    expect(formatPlayedDelta(0.2)).toBe("+20%");
    expect(formatPlayedDelta(-0.082)).toBe("-8%");
    expect(formatPlayedDelta(0)).toBe("0%");
    expect(formatPlayedDelta(null)).toBeNull();
  });
});
