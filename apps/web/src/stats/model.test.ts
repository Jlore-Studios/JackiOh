// The statistics' arithmetic (SPEC R639): pure functions over a game's log and the device's totals.

import { describe, expect, it } from "vitest";

import {
  EMPTY_LOG,
  EMPTY_STATS,
  NO_COUNTS,
  addGame,
  logIsEmpty,
  logMax,
  logSeen,
  logWith,
  MAX_COUNTERS,
  SUMMED_COUNTERS,
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
    expect(once.cards["core-001"]).toEqual({ ...NO_COUNTS, seen: 1, played: 1, destroyed: 1, playedInWin: 1 });
    expect(once.cards["core-002"]).toEqual({ ...NO_COUNTS, seen: 1, defeated: 1 });

    const twice = addGame(once, log, "loss");
    expect(twice).toMatchObject({ games: 2, wins: 1, losses: 1 });
    expect(twice.cards["core-001"]?.seen).toBe(2);
    expect(once.games).toBe(1);
    expect(addGame(twice, EMPTY_LOG, "draw")).toMatchObject({ games: 3, draws: 1 });
  });

  it("R639 the win percentage is whole, and absent before the first game", () => {
    expect(winPercent(EMPTY_STATS)).toBeNull();
    expect(winPercent({ ...EMPTY_STATS, games: 3, wins: 2 })).toBe(67);
  });

  it("R639 the favorites list is highest first, by id among equals, never lists a zero, and stops at the limit", () => {
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
    expect(NO_COUNTS).toMatchObject({
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

describe("R1743000 the new counters and totals", () => {
  it("R1743000 the new counters sum across games, and biggestHit and overkill keep the max", () => {
    const first = logMax(logMax(logWith(logWith(EMPTY_LOG, "damageDealt", "core-001", 5), "attacks", "core-001"), "biggestHit", "core-001", 5), "overkill", "core-001", 2);
    const second = logMax(logMax(logWith(logWith(EMPTY_LOG, "damageDealt", "core-001", 3), "healing", "core-002", 4), "biggestHit", "core-001", 3), "overkill", "core-001", 6);
    const third = logWith(logWith(EMPTY_LOG, "drawn", "core-002"), "radiant", "core-002");
    let stats = addGame(EMPTY_STATS, first, "win");
    stats = addGame(addGame(stats, second, "win"), third, "win");
    expect(stats.cards["core-001"]).toEqual({ ...NO_COUNTS, damageDealt: 8, attacks: 1, biggestHit: 5, overkill: 6 });
    expect(stats.cards["core-002"]).toEqual({ ...NO_COUNTS, healing: 4, drawn: 1, radiant: 1 });
    expect(addGame(stats, logMax(EMPTY_LOG, "biggestHit", "core-001", 1), "loss").cards["core-001"]?.biggestHit).toBe(5);
  });

  it("R1743000 logMax raises a card's value only when the new one is larger, and else is the log itself", () => {
    const log = logMax(EMPTY_LOG, "biggestHit", "core-001", 4);
    expect(log.biggestHit).toEqual({ "core-001": 4 });
    expect(logMax(log, "biggestHit", "core-001", 4)).toBe(log);
    expect(logMax(log, "biggestHit", "core-001", 3)).toBe(log);
    expect(logMax(log, "overkill", "core-001", 0)).toBe(log);
    expect(logMax(log, "biggestHit", "core-001", 9).biggestHit).toEqual({ "core-001": 9 });
  });

  it("R1743000 playedInWin and playedInLoss count once for each card played in a won or lost game, a draw neither", () => {
    const log = logWith(logWith(logWith(EMPTY_LOG, "played", "core-001"), "played", "core-001"), "played", "core-002");
    let stats = addGame(EMPTY_STATS, log, "win");
    stats = addGame(stats, logWith(EMPTY_LOG, "played", "core-001"), "loss");
    stats = addGame(stats, log, "draw");
    // A card only seen or played against is in neither.
    stats = addGame(stats, logWith(logSeen(EMPTY_LOG, ["core-003"]), "playedAgainst", "core-003"), "win");
    expect(stats.cards["core-001"]).toMatchObject({ played: 5, playedInWin: 1, playedInLoss: 1 });
    expect(stats.cards["core-002"]).toMatchObject({ played: 2, playedInWin: 1, playedInLoss: 0 });
    expect(stats.cards["core-003"]).toMatchObject({ playedInWin: 0, playedInLoss: 0 });
  });

  it("R1743000 a loss or a draw resets the streak, and the best stays", () => {
    let stats = addGame(addGame(addGame(EMPTY_STATS, EMPTY_LOG, "win"), EMPTY_LOG, "win"), EMPTY_LOG, "win");
    expect(stats).toMatchObject({ streak: 3, bestStreak: 3 });
    stats = addGame(stats, EMPTY_LOG, "loss");
    expect(stats).toMatchObject({ streak: 0, bestStreak: 3 });
    stats = addGame(addGame(stats, EMPTY_LOG, "win"), EMPTY_LOG, "draw");
    expect(stats).toMatchObject({ streak: 0, bestStreak: 3 });
    stats = addGame(addGame(stats, EMPTY_LOG, "win"), EMPTY_LOG, "win");
    expect(stats).toMatchObject({ streak: 2, bestStreak: 3 });
    for (let i = 0; i < 2; i += 1) stats = addGame(stats, EMPTY_LOG, "win");
    expect(stats).toMatchObject({ streak: 4, bestStreak: 4 });
  });

  it("R1743000 the longest game keeps its turns and the plays in it, and a shorter or equal game leaves them", () => {
    const long = { ...logWith(logWith(EMPTY_LOG, "played", "core-001"), "played", "core-002"), turns: 23 };
    let stats = addGame(EMPTY_STATS, long, "win");
    expect(stats.longestGame).toBe(23);
    expect(stats.longestGamePlayed).toEqual({ "core-001": 1, "core-002": 1 });
    stats = addGame(stats, { ...logWith(EMPTY_LOG, "played", "core-009"), turns: 23 }, "win");
    stats = addGame(stats, { ...logWith(EMPTY_LOG, "played", "core-009"), turns: 5 }, "loss");
    expect(stats).toMatchObject({ longestGame: 23, longestGamePlayed: { "core-001": 1, "core-002": 1 } });
    stats = addGame(stats, { ...logWith(EMPTY_LOG, "played", "core-009"), turns: 24 }, "loss");
    expect(stats).toMatchObject({ longestGame: 24, longestGamePlayed: { "core-009": 1 } });
    expect(addGame(stats, EMPTY_LOG, "win").longestGame).toBe(24);
  });

  it("R1743000 each new record and the turns alone make a log non-empty, and every new counter starts at 0", () => {
    for (const counter of SUMMED_COUNTERS) expect(logIsEmpty(logWith(EMPTY_LOG, counter, "core-001"))).toBe(false);
    for (const counter of MAX_COUNTERS) expect(logIsEmpty(logMax(EMPTY_LOG, counter, "core-001", 1))).toBe(false);
    expect(logIsEmpty({ ...EMPTY_LOG, turns: 1 })).toBe(false);
    expect(NO_COUNTS).toMatchObject({ damageDealt: 0, biggestHit: 0, overkill: 0, attacks: 0, healing: 0, drawn: 0, radiant: 0, playedInWin: 0, playedInLoss: 0 });
    expect(EMPTY_LOG).toMatchObject({ damageDealt: {}, biggestHit: {}, overkill: {}, attacks: {}, healing: {}, drawn: {}, radiant: {}, turns: 0 });
    expect(EMPTY_STATS).toMatchObject({ streak: 0, bestStreak: 0, longestGame: 0, longestGamePlayed: {} });
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
