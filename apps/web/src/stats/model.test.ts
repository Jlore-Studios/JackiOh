// The statistics' arithmetic (SPEC R639): pure functions over a game's log and the device's totals.

import { describe, expect, it } from "vitest";

import {
  EMPTY_LOG,
  EMPTY_STATS,
  addGame,
  logIsEmpty,
  logSeen,
  logWith,
  outcomeFor,
  topCards,
  winPercent,
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
    expect(once.cards["core-001"]).toEqual({ seen: 1, played: 1, playedAgainst: 0, destroyed: 1, defeated: 0 });
    expect(once.cards["core-002"]).toEqual({ seen: 1, played: 0, playedAgainst: 0, destroyed: 0, defeated: 1 });

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
        "core-003": { seen: 1, played: 2, playedAgainst: 0, destroyed: 0, defeated: 0 },
        "core-001": { seen: 1, played: 2, playedAgainst: 0, destroyed: 0, defeated: 0 },
        "core-002": { seen: 1, played: 5, playedAgainst: 0, destroyed: 0, defeated: 0 },
        "core-004": { seen: 9, played: 0, playedAgainst: 0, destroyed: 0, defeated: 0 },
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
