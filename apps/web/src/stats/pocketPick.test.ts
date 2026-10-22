// The Pocket Pick (SPEC R639, R1583000).

import { afterEach, describe, expect, it, vi } from "vitest";

import type { PlayRatesResponse } from "../net/api.ts";
import { POCKET_PICK_MIN_GAMES, POCKET_PICK_MIN_PLAYS } from "./config.ts";
import { NO_COUNTS, type PlayerStats } from "./model.ts";
import { loadPocketPick, pocketPick } from "./pocketPick.ts";

afterEach(() => vi.unstubAllGlobals());

/** A device tally of `games` games, each card played as many times as `played` says. */
function tally(games: number, played: Record<string, number>): PlayerStats {
  const cards = Object.fromEntries(Object.entries(played).map(([id, n]) => [id, { ...NO_COUNTS, played: n }]));
  return { games, wins: 0, losses: 0, draws: 0, cards };
}

/** The server's play rates, each card at the rate given, in key order. */
function rates(byCard: Record<string, number>): PlayRatesResponse {
  return { decks: 1000, cards: Object.entries(byCard).map(([id, rate]) => ({ id, rate })) };
}

/** Enough games that the game floor never decides a case. */
const G = 2 * POCKET_PICK_MIN_GAMES;

describe("R1583000 pocketPick", () => {
  it("R1583000 picks the card played most often next to the player base, with both rates", () => {
    const stats = tally(G, { "core-001": 10, "core-002": 20, "core-003": 4 });
    // 0.5 against 0.1 is five times the base; 1 against 0.5 twice; 0.2 against 0.05 four times.
    expect(pocketPick(stats, rates({ "core-001": 0.1, "core-002": 0.5, "core-003": 0.05 }))).toEqual({
      id: "core-001",
      playerRate: 0.5,
      baseRate: 0.1,
    });
  });

  it("R1583000 names none below POCKET_PICK_MIN_GAMES games", () => {
    const base = rates({ "core-001": 0.1 });
    expect(pocketPick(tally(POCKET_PICK_MIN_GAMES - 1, { "core-001": 5 }), base)).toBeNull();
    expect(pocketPick(tally(POCKET_PICK_MIN_GAMES, { "core-001": 5 }), base)?.id).toBe("core-001");
  });

  it("R1583000 a card needs POCKET_PICK_MIN_PLAYS plays", () => {
    const base = rates({ "core-001": 0.001, "core-002": 0.1 });
    expect(pocketPick(tally(G, { "core-001": POCKET_PICK_MIN_PLAYS - 1, "core-002": 10 }), base)?.id).toBe("core-002");
    expect(pocketPick(tally(G, { "core-001": POCKET_PICK_MIN_PLAYS, "core-002": 10 }), base)?.id).toBe("core-001");
  });

  it("R1583000 never picks an unlisted card or one played no more often than the base", () => {
    const stats = tally(G, { "core-001": 10, "core-002": 10, "core-003": 4 });
    expect(pocketPick(stats, rates({ "core-002": 0.5, "core-003": 0.6 }))).toBeNull();
  });

  it("R1583000 a base rate of 0 ranks first, then the higher player rate, then the lower id", () => {
    expect(pocketPick(tally(G, { "core-001": 4, "core-002": 20 }), rates({ "core-001": 0, "core-002": 0.01 }))?.id).toBe(
      "core-001",
    );
    // Both twice the base: the card played more often wins.
    expect(pocketPick(tally(G, { "core-001": 5, "core-002": 10 }), rates({ "core-001": 0.125, "core-002": 0.25 }))?.id).toBe(
      "core-002",
    );
    // A tie throughout: the lower id, whatever order the server lists them in.
    expect(pocketPick(tally(G, { "core-001": 10, "core-002": 10 }), rates({ "core-002": 0.25, "core-001": 0.25 }))?.id).toBe(
      "core-001",
    );
  });

  it("R1583000 the player's rate is at most 1", () => {
    expect(pocketPick(tally(G, { "core-001": 30 }), rates({ "core-001": 0.5 }))?.playerRate).toBe(1);
  });
});

describe("R1583000 loadPocketPick", () => {
  it("R1583000 reads the play rates and picks from them", async () => {
    const requests = vi.fn((_input: unknown) =>
      Promise.resolve(new Response(JSON.stringify(rates({ "core-001": 0.1 })), { status: 200 })),
    );
    vi.stubGlobal("fetch", requests);
    await expect(loadPocketPick(tally(G, { "core-001": 10 }))).resolves.toEqual({
      id: "core-001",
      playerRate: 0.5,
      baseRate: 0.1,
    });
    expect(String(requests.mock.calls[0]?.[0])).toContain("/api/stats/play-rates");
  });

  it("R1583000 names none when the request fails", async () => {
    const stats = tally(G, { "core-001": 10 });
    vi.stubGlobal(
      "fetch",
      vi.fn((_input: unknown) => Promise.reject(new TypeError("offline"))),
    );
    await expect(loadPocketPick(stats)).resolves.toBeNull();
    vi.stubGlobal(
      "fetch",
      vi.fn((_input: unknown) => Promise.resolve(new Response("{}", { status: 500 }))),
    );
    await expect(loadPocketPick(stats)).resolves.toBeNull();
  });

  it("R1583000 asks nothing below POCKET_PICK_MIN_GAMES games", async () => {
    const requests = vi.fn((_input: unknown) =>
      Promise.resolve(new Response(JSON.stringify(rates({ "core-001": 0.1 })), { status: 200 })),
    );
    vi.stubGlobal("fetch", requests);
    await expect(loadPocketPick(tally(POCKET_PICK_MIN_GAMES - 1, { "core-001": 9 }))).resolves.toBeNull();
    expect(requests).not.toHaveBeenCalled();
  });
});
