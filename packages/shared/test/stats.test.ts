// SPEC §9.11, R376–R378: card win rates read off game records. The records here are written by hand,
// so each figure can be counted on the fingers; the engine's `summarizeGame` (packages/engine
// test/game-summary.test.ts) and the two writers (apps/server game-records.test.ts, packages/ai
// dev-run.test.ts) prove the records themselves.

import { describe, expect, it } from "vitest";

import {
  BREAKDOWN_TITLES,
  BREAKDOWNS,
  cardStats,
  DEFAULT_CARD_STATS_FILTER,
  describeFilter,
  formatCardStats,
  formatDelta,
  formatTally,
  parseGameRecord,
  parseGameRecordLines,
  playedDelta,
  winRate,
  type CardStats,
  type CardStatsFilter,
  type GameRecord,
  type SeatSummary,
} from "../src/index";

function seat(partial: Partial<SeatSummary> = {}): SeatSummary {
  return { deck: ["a", "b", "c"], opening: [], drawn: [], played: [], ...partial };
}

function record(partial: Partial<GameRecord> & { winner?: GameRecord["game"]["winner"] } = {}): GameRecord {
  const { winner = "p1", ...rest } = partial;
  return {
    id: "m1",
    source: "live",
    mode: "bo1",
    patch: "v0.1.1",
    pilots: { p1: "human", p2: "human" },
    game: { first: "p1", winner, reason: winner === "draw" ? "turn-cap" : "hero-death", turns: 12, seats: { p1: seat(), p2: seat() } },
    ...rest,
  };
}

function statsOf(report: ReturnType<typeof cardStats>, card: string): CardStats {
  const found = report.cards.find((stats) => stats.card === card);
  if (found === undefined) throw new Error(`no entry for ${card}`);
  return found;
}

const live: CardStatsFilter = { ...DEFAULT_CARD_STATS_FILTER };

describe("card win rates (§9.11)", () => {
  it("R377 counts every breakdown over the decks that held the card, a mirror once per deck", () => {
    const games = [
      // p1 wins going first. p1 opens with a, draws b and plays a; c stays in its library.
      record({
        id: "g1",
        game: {
          first: "p1",
          winner: "p1",
          reason: "hero-death",
          turns: 9,
          seats: {
            p1: seat({ deck: ["a", "b", "c"], opening: ["a"], drawn: ["b"], played: ["a"] }),
            // p2 holds a too: the mirror counts it once more, as a loss going second.
            p2: seat({ deck: ["a", "d"], opening: ["core-t-coin", "d"], drawn: ["a"], played: ["d"] }),
          },
        },
      }),
      // p2 wins going second with a in its opening hand, played.
      record({
        id: "g2",
        winner: "p2",
        game: {
          first: "p1",
          winner: "p2",
          reason: "concede",
          turns: 4,
          seats: {
            p1: seat({ deck: ["b", "c"], opening: ["b", "c"], played: ["c"] }),
            p2: seat({ deck: ["a", "b"], opening: ["a"], played: ["a", "a"] }),
          },
        },
      }),
      // A draw: both decks hold b, drawn and never played.
      record({
        id: "g3",
        game: {
          first: "p1",
          winner: "draw",
          reason: "turn-cap",
          turns: 30,
          seats: {
            p1: seat({ deck: ["b"], drawn: ["b"] }),
            p2: seat({ deck: ["b"], opening: ["b"] }),
          },
        },
      }),
    ];

    const report = cardStats(games, live);
    expect(report.games).toBe(3);
    expect(report.decks).toBe(6);
    expect(report.cards.map((stats) => stats.card)).toEqual(["a", "b", "c", "d"]);

    const a = statsOf(report, "a");
    expect(a.inDeck).toEqual({ games: 3, wins: 2, draws: 0 });
    expect(a.openingHand).toEqual({ games: 2, wins: 2, draws: 0 });
    expect(a.goingFirst).toEqual({ games: 1, wins: 1, draws: 0 });
    expect(a.goingSecond).toEqual({ games: 2, wins: 1, draws: 0 });
    // Played twice in g2 is still one game.
    expect(a.played).toEqual({ games: 2, wins: 2, draws: 0 });
    // g1's p2 drew a and never played it.
    expect(a.drawnNotPlayed).toEqual({ games: 1, wins: 0, draws: 0 });

    const b = statsOf(report, "b");
    expect(b.inDeck).toEqual({ games: 5, wins: 2, draws: 2 });
    expect(b.openingHand).toEqual({ games: 2, wins: 0, draws: 1 });
    expect(b.played).toEqual({ games: 0, wins: 0, draws: 0 });
    // g1 p1 (drawn), g2 p1 (opening), g3 both: never played. g2 p2 held b and never drew it.
    expect(b.drawnNotPlayed).toEqual({ games: 4, wins: 1, draws: 2 });

    // c never reached g1's hand, so it is in neither side of the delta there.
    const c = statsOf(report, "c");
    expect(c.inDeck.games).toBe(2);
    expect(c.played).toEqual({ games: 1, wins: 0, draws: 0 });
    expect(c.drawnNotPlayed).toEqual({ games: 0, wins: 0, draws: 0 });

    // The Coin was in an opening hand and is in no deck, so it has no entry.
    expect(report.cards.some((stats) => stats.card === "core-t-coin")).toBe(false);
  });

  it("R377 puts a card played without being drawn on the played side, never the baseline", () => {
    const made = record({
      game: {
        first: "p1",
        winner: "p1",
        reason: "hero-death",
        turns: 8,
        seats: { p1: seat({ deck: ["a"], played: ["a"] }), p2: seat({ deck: ["z"] }) },
      },
    });
    const a = statsOf(cardStats([made], live), "a");
    expect(a.played.games).toBe(1);
    expect(a.drawnNotPlayed.games).toBe(0);
  });

  it("R377 defines the played delta against the drawn-but-not-played baseline, in points", () => {
    const stats: CardStats = {
      card: "a",
      inDeck: { games: 10, wins: 5, draws: 0 },
      openingHand: { games: 0, wins: 0, draws: 0 },
      goingFirst: { games: 5, wins: 3, draws: 0 },
      goingSecond: { games: 5, wins: 2, draws: 0 },
      played: { games: 4, wins: 3, draws: 0 },
      drawnNotPlayed: { games: 5, wins: 2, draws: 1 },
    };
    expect(winRate(stats.played)).toBe(0.75);
    expect(winRate(stats.drawnNotPlayed)).toBe(0.4);
    expect(playedDelta(stats)).toBeCloseTo(0.35);
    expect(formatDelta(playedDelta(stats))).toBe("+35.0");
    expect(formatDelta(-0.082)).toBe("-8.2");
    expect(winRate(stats.openingHand)).toBeNull();
    expect(playedDelta({ ...stats, drawnNotPlayed: { games: 0, wins: 0, draws: 0 } })).toBeNull();
    expect(formatDelta(null)).toBe("—");
  });

  it("R378 reads live games only unless a development run is asked for by name", () => {
    const records = [
      record({ id: "live-1", source: "live", winner: "p1" }),
      record({ id: "dev-1", source: "dev", pilots: { p1: "ai", p2: "ai" }, winner: "p2" }),
      record({ id: "dev-2", source: "dev", pilots: { p1: "ai", p2: "ai" }, winner: "p2" }),
    ];
    expect(DEFAULT_CARD_STATS_FILTER.source).toBe("live");
    expect(cardStats(records).games).toBe(1);
    expect(statsOf(cardStats(records), "a").inDeck.wins).toBe(1);
    expect(cardStats(records, { ...live, source: "dev" }).games).toBe(2);
    expect(cardStats(records, { ...live, source: "all" }).games).toBe(3);
  });

  it("R377 filters by any combination of match type, patch and pilot", () => {
    const records = [
      record({ id: "1", mode: "bo1", patch: "v0.1.1" }),
      record({ id: "2", mode: "random", patch: "v0.1.1" }),
      record({ id: "3", mode: "bo3", patch: "v0.2.5" }),
      // A practice-shaped record: one human seat, one AI seat. The AI's deck holds only z.
      record({
        id: "4",
        mode: "random",
        patch: "v0.2.5",
        pilots: { p1: "human", p2: "ai" },
        game: { first: "p1", winner: "p2", reason: "hero-death", turns: 10, seats: { p1: seat(), p2: seat({ deck: ["z"] }) } },
      }),
    ];
    const games = (filter: Partial<CardStatsFilter>): number => cardStats(records, { ...live, ...filter }).games;

    expect(games({})).toBe(4);
    expect(games({ mode: "random" })).toBe(2);
    expect(games({ patch: "v0.2.5" })).toBe(2);
    expect(games({ mode: "random", patch: "v0.2.5" })).toBe(1);
    expect(games({ mode: "bo1", patch: "v0.2.5" })).toBe(0);

    // Pilots count seats: record 4's human seat holds a, its AI seat z.
    const human = cardStats(records, { ...live, pilot: "human" });
    expect(human.decks).toBe(7);
    expect(human.cards.map((stats) => stats.card)).toEqual(["a", "b", "c"]);
    const ai = cardStats(records, { ...live, pilot: "ai" });
    expect(ai.games).toBe(1);
    expect(ai.decks).toBe(1);
    expect(ai.cards.map((stats) => stats.card)).toEqual(["z"]);
    expect(statsOf(ai, "z").inDeck).toEqual({ games: 1, wins: 1, draws: 0 });
    const unified = cardStats(records, { ...live, pilot: "unified" });
    expect(unified.decks).toBe(8);
    expect(statsOf(unified, "a").inDeck.games).toBe(7);
  });

  it("R377 shows every breakdown for each card with its games beside every win rate", () => {
    const report = cardStats(
      [
        record({
          game: {
            first: "p1",
            winner: "p1",
            reason: "hero-death",
            turns: 9,
            seats: { p1: seat({ deck: ["a"], opening: ["a"], played: ["a"] }), p2: seat({ deck: ["b"], drawn: ["b"] }) },
          },
        }),
      ],
      { ...live, mode: "bo1", patch: "v0.1.1", pilot: "human" },
    );
    const names: Record<string, string> = { a: "Alpha" };
    const text = formatCardStats(report, (card) => names[card]);
    const lines = text.split("\n");

    expect(lines[0]).toBe("Card win rates: live games, Best of 1, patch v0.1.1, human pilots.");
    expect(lines[1]).toBe("1 games, 2 decks.");
    // The table starts after the blank line under the legend.
    const heading = lines[lines.indexOf("") + 1];
    expect(heading?.startsWith("Card ")).toBe(true);
    for (const breakdown of BREAKDOWNS) expect(heading).toContain(BREAKDOWN_TITLES[breakdown]);
    expect(heading).toContain("Played Δ");

    const alpha = lines.find((line) => line.startsWith("a Alpha"));
    // In deck, opening hand, going first, going second, played, drawn-not-played; then the delta.
    expect(alpha?.split(/\s{2,}/).slice(1)).toEqual(["100.0% (1)", "100.0% (1)", "100.0% (1)", "— (0)", "100.0% (1)", "— (0)", "—"]);
    const beta = lines.find((line) => line.startsWith("b "));
    expect(beta?.split(/\s{2,}/).slice(1)).toEqual(["0.0% (1)", "— (0)", "— (0)", "0.0% (1)", "— (0)", "0.0% (1)", "—"]);

    expect(formatTally({ games: 3, wins: 1, draws: 1 })).toBe("33.3% (3)");
    expect(describeFilter({ source: "all", mode: null, patch: null, pilot: "unified" })).toBe(
      "live games and AI development runs, every mode, every patch, human and AI pilots (unified)",
    );
    expect(describeFilter({ source: "dev", mode: "random", patch: "v0.2.5", pilot: "ai" })).toBe(
      "AI development runs, All Random, patch v0.2.5, AI pilots",
    );
  });
});

describe("reading a record back (§9.11)", () => {
  it("R376 reads back what was written and refuses a record with a bad field", () => {
    const written = record({ source: "dev", pilots: { p1: "ai", p2: "ai" } });
    expect(parseGameRecord(JSON.parse(JSON.stringify(written)))).toEqual(written);

    expect(() => parseGameRecord({ ...written, source: "practice" })).toThrow("source is not one of live, dev");
    expect(() => parseGameRecord({ ...written, mode: "bo5" })).toThrow("mode");
    expect(() => parseGameRecord({ ...written, patch: "" })).toThrow("patch");
    expect(() => parseGameRecord({ ...written, pilots: { p1: "ai", p2: "robot" } })).toThrow("pilots.p2");
    expect(() => parseGameRecord({ ...written, game: { ...written.game, reason: "rage-quit" } })).toThrow("game.reason");
    expect(() => parseGameRecord({ ...written, game: { ...written.game, turns: -1 } })).toThrow("game.turns");
    expect(() =>
      parseGameRecord({ ...written, game: { ...written.game, seats: { ...written.game.seats, p2: { ...seat(), played: [7] } } } }),
    ).toThrow("game.seats.p2.played");
    expect(() => parseGameRecord([written])).toThrow("an object");
  });

  it("reads one record per line, skipping blank lines and naming the line it cannot read", () => {
    const one = record({ id: "x" });
    const two = record({ id: "y" });
    expect(parseGameRecordLines(`${JSON.stringify(one)}\n\n${JSON.stringify(two)}\n`)).toEqual([one, two]);
    expect(() => parseGameRecordLines(`${JSON.stringify(one)}\n{"id":"z"}\n`)).toThrow(/^line 2: /);
    expect(() => parseGameRecordLines("not json")).toThrow(/^line 1: /);
  });
});
