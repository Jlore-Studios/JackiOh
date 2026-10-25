// R1943000: the Sets tab's figures (stats/sets.ts), read off `GET /api/stats/cards`'s card rows.

import { describe, expect, it } from "vitest";

import type { PublicCardStat } from "../net/api.ts";
import { SET_STATS_CARDS_LISTED } from "./config.ts";
import { changes, leaders, outliers, setOf, setRows } from "./sets.ts";

const MIN_SAMPLE = 20;

function card(id: string, set: string, wins: number, games: number): PublicCardStat {
  return {
    id,
    name: id,
    cost: 1,
    rarity: "Common",
    set,
    games,
    winRate: games > 0 ? wins / games : null,
    drawnGames: 0,
    drawnWinRate: null,
    playedGames: 0,
    playedWinRate: null,
    drawnNotPlayedGames: 0,
    drawnNotPlayedWinRate: null,
    playedDelta: null,
    playRate: 0,
    hasEnoughGames: games >= MIN_SAMPLE,
  };
}

const CARDS = [
  card("core-001", "Core", 700, 1200), // 58.3%, 5.8 standard errors over
  card("core-002", "Core", 450, 1000), // 45%, 3.2 under
  card("core-003", "Core", 9, 10), // under the floor: pooled, never ranked
  card("classic-001", "Classic", 520, 1000), // 52%
  card("classic-002", "Classic", 480, 1000), // 48%
  card("classic-003", "Classic", 500, 1000), // 50%
  card("classicplus-001", "classic+", 0, 0), // no games: no row
];

describe("R1943000 the Sets tab's figures", () => {
  it("R1943000 a set pools every card's games and wins, and takes its mean, median, best and worst over the cards with the floor's games", () => {
    const rows = setRows(CARDS, MIN_SAMPLE);
    expect(rows.map((row) => row.set)).toEqual(["Core", "Classic"]); // highest pooled first
    const [core, classic] = rows;
    expect(core).toMatchObject({ cards: 3, rated: 2, games: 2210, wins: 1159, strong: 1, weak: 1 });
    expect(core!.pooled).toBeCloseTo(1159 / 2210);
    expect(core!.error).toBeCloseTo(Math.sqrt(((1159 / 2210) * (1 - 1159 / 2210)) / 2210));
    expect(core!.mean).toBeCloseTo((0.7 / 1.2 + 0.45) / 2);
    expect(core!.median).toBeCloseTo((0.7 / 1.2 + 0.45) / 2);
    expect(core!.best?.id).toBe("core-001");
    expect(core!.worst?.id).toBe("core-002");
    expect(classic).toMatchObject({ cards: 3, rated: 3, games: 3000, wins: 1500, strong: 0, weak: 0 });
    expect(classic!.median).toBeCloseTo(0.5);
    expect(classic!.best?.id).toBe("classic-001");
    expect(classic!.worst?.id).toBe("classic-002");
  });

  it("R1943000 names each set as SHIPPED_SETS spells it, whatever the case of the row", () => {
    expect(setOf({ set: "classic+" })).toBe("Classic+");
    expect(setOf({ set: "MEDITATIVE" })).toBe("Meditative");
    expect(setOf({ set: "Stinkyscape" })).toBe("Stinkyscape");
  });

  it("R1943000 names the highest set by each measure, and the change since the previous patch where both have games", () => {
    const rows = setRows(CARDS, MIN_SAMPLE);
    // Core leads all three: pooled 52.4% to 50%, and mean and median 51.7% to 50%.
    expect(leaders(rows)).toEqual({ pooled: "Core", mean: "Core", median: "Core" });
    const before = setRows([card("core-001", "Core", 500, 1000), card("classic-001", "Classic", 550, 1000)], MIN_SAMPLE);
    const delta = changes(rows, before);
    expect(delta.get("Core")).toBeCloseTo(1159 / 2210 - 0.5);
    expect(delta.get("Classic")).toBeCloseTo(0.5 - 0.55);
    expect(changes(rows, []).size).toBe(0);
    expect(leaders([])).toEqual({ pooled: null, mean: null, median: null });
  });

  it("R1943000 lists the cards 3 or more standard errors from 50%, furthest first, at most SET_STATS_CARDS_LISTED a side", () => {
    const { strongest, weakest } = outliers(CARDS, MIN_SAMPLE);
    expect(strongest.map((c) => c.id)).toEqual(["core-001"]);
    expect(strongest[0]!.z).toBeCloseTo((0.7 / 1.2 - 0.5) / Math.sqrt(0.25 / 1200));
    expect(weakest.map((c) => c.id)).toEqual(["core-002"]);
    const many = Array.from({ length: SET_STATS_CARDS_LISTED + 5 }, (_, n) => card(`core-${100 + n}`, "Core", 600 + n, 1000));
    const listed = outliers(many, MIN_SAMPLE).strongest;
    expect(listed).toHaveLength(SET_STATS_CARDS_LISTED);
    expect(listed[0]!.id).toBe(`core-${100 + SET_STATS_CARDS_LISTED + 4}`);
  });
});
