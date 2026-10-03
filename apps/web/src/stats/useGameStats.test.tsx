// Logging a game as the board is handed its views (SPEC R639): once, at the end, from the player's
// own seat, and only when the route says the game is the player's own.

import { renderHook } from "@testing-library/react";
import type { GameEvent, PlayerView } from "@jackioh/shared";
import { beforeEach, describe, expect, it } from "vitest";

import { baseView, card, emptySide, resetIds } from "../test/fixtures.ts";
import { PLAYER_STATS_KEY } from "./config.ts";
import { dropPlayerStatsCache, readPlayerStats } from "./store.ts";
import { useGameStats } from "./useGameStats.ts";

beforeEach(() => {
  resetIds();
  window.localStorage.clear();
  dropPlayerStatsCache();
});

function play(defId: string, player: "p1" | "p2" = "p1"): GameEvent {
  return { type: "cardPlayed", player, instanceId: `i-${defId}`, defId, costPaid: 1 };
}

function viewWith(events: GameEvent[], over: Partial<PlayerView> = {}): PlayerView {
  return baseView({
    you: emptySide("p1", { hand: [card({ defId: "core-001" })] }),
    events,
    ...over,
  });
}

describe("R639 useGameStats", () => {
  it("R639 logs a finished game once: what was seen, what was played, and the result", () => {
    const first = viewWith([]);
    const { rerender } = renderHook(({ view }) => useGameStats(view, true), { initialProps: { view: first } });
    expect(readPlayerStats().games).toBe(0);

    const second = viewWith([play("core-001"), play("core-002", "p2")]);
    rerender({ view: second });
    expect(readPlayerStats().games).toBe(0);

    const over = viewWith([play("core-001"), play("core-002", "p2")], { result: { winner: "p1", reason: "hero-death" } });
    rerender({ view: over });
    const stats = readPlayerStats();
    expect(stats).toMatchObject({ games: 1, wins: 1, losses: 0 });
    expect(stats.cards["core-001"]).toMatchObject({ seen: 1, played: 1 });
    expect(stats.cards["core-002"]).toMatchObject({ playedAgainst: 1 });

    // The same finished view again, or a re-render, adds nothing.
    rerender({ view: { ...over } });
    expect(readPlayerStats().games).toBe(1);
  });

  it("R639 an events window that slides is counted only for what is new", () => {
    const a = viewWith([play("core-001")]);
    const { rerender } = renderHook(({ view }) => useGameStats(view, true), { initialProps: { view: a } });
    // The window now holds the old play and a new one: the old is not played twice.
    rerender({ view: viewWith([play("core-001"), play("core-003")]) });
    rerender({
      view: viewWith([play("core-001"), play("core-003")], { result: { winner: "p2", reason: "concede" } }),
    });
    const stats = readPlayerStats();
    expect(stats.losses).toBe(1);
    expect(stats.cards["core-003"]?.played).toBe(1);
    expect(stats.cards["core-001"]?.played ?? 0).toBe(0);
  });

  it("R639 a board the route does not track logs nothing", () => {
    const { rerender } = renderHook(({ view }) => useGameStats(view, false), { initialProps: { view: viewWith([]) } });
    rerender({ view: viewWith([], { result: { winner: "p1", reason: "hero-death" } }) });
    expect(readPlayerStats().games).toBe(0);
    expect(window.localStorage.getItem(PLAYER_STATS_KEY)).toBeNull();
  });

  it("R639 a new game on the same board starts a log of its own", () => {
    const { rerender } = renderHook(({ view }) => useGameStats(view, true), { initialProps: { view: viewWith([]) } });
    rerender({ view: viewWith([], { result: { winner: "p1", reason: "hero-death" } }) });
    expect(readPlayerStats().games).toBe(1);

    rerender({ view: baseView({ you: emptySide("p1", { hand: [card({ defId: "core-007" })] }) }) });
    rerender({ view: baseView({ result: { winner: "draw", reason: "turn-cap" } }) });
    const stats = readPlayerStats();
    expect(stats).toMatchObject({ games: 2, draws: 1 });
    expect(stats.cards["core-007"]?.seen).toBe(1);
    // The first game's card is not carried into the second game's log.
    expect(stats.cards["core-001"]?.seen).toBe(1);
  });
});
