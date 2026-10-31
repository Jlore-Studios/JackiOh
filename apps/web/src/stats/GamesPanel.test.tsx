// Tests for the Games panel of /stats (SPEC §9.11, R1994000).

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { expectSkeleton } from "../test/skeleton.ts";
import { GamesPanel } from "./GamesPanel.tsx";
import { statsTestid } from "./testids.ts";

const SLOW = { timeout: 10_000 } as const;

const FIGURES = {
  games: 400,
  draws: 4,
  firstWins: 204,
  firstShare: 0.51,
  firstLow: 0.46,
  firstHigh: 0.56,
  reasons: { "hero-death": 390, "turn-cap": 10 },
  turnQuartiles: [12, 17, 23],
  counted: 400,
  fatigueGames: 48,
  fatigueShare: 0.12,
  fatigueDamageMean: 3.5,
  trapsSet: 300,
  trapsFired: 120,
  trapsFiredPerSet: 0.4,
  positionSwitches: 80,
  positionSwitchesMean: 0.2,
  radiantUpgrades: 60,
  radiantUpgradesMean: 0.15,
};

function gamesResponse(figures: Record<string, unknown> | null, totalGames = 400, minSample = 20) {
  return {
    patch: "v0.5.0b",
    source: "live" as const,
    sourceLabel: "Live games",
    mode: "bo1" as const,
    modeLabel: "Best of 1",
    minSample,
    totalGames,
    figures,
    pairs: figures === null ? [] : [{ pilots: ["human", "ai"], figures }],
  };
}

function answer(body: unknown): Response {
  return new Response(JSON.stringify(body), { status: 200 });
}

const FIGURE_LINES = [/First player wins/, /Median .* turns/, /reach fatigue/];

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R1994000 the Games panel", () => {
  it("R1994000 the panel shows the first player's wins with their margin, the median turns and fatigue", async () => {
    const fetchMock = vi.fn((_input: RequestInfo | URL) => Promise.resolve(answer(gamesResponse(FIGURES))));
    vi.stubGlobal("fetch", fetchMock);

    render(<GamesPanel patch="v0.5.0b" mode="bo1" />);

    expect(await screen.findByText("First player wins 51% of decided games (±5)", undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByText("Median 17 turns")).toBeInTheDocument();
    expect(screen.getByText("12% of games reach fatigue")).toBeInTheDocument();
    expect(screen.queryByTestId(statsTestid.gamesFew)).toBeNull();

    const url = new URL(String(fetchMock.mock.calls[0]?.[0]), window.location.origin);
    expect(`${url.pathname}${url.search}`).toBe("/api/stats/games?patch=v0.5.0b&mode=bo1");

    // No data source, no gate and no word of AI games (R661); every game counted, so no note.
    const panel = screen.getByTestId(statsTestid.gamesPanel);
    expect(panel.textContent).not.toMatch(/\blive\b|AI/);
    expect(panel.textContent).not.toContain("Counted over");
  });

  it("R2446000 the panel shows no set traps line, whatever traps the response counts", async () => {
    vi.stubGlobal("fetch", vi.fn((_input: RequestInfo | URL) => Promise.resolve(answer(gamesResponse(FIGURES)))));

    render(<GamesPanel patch="v0.5.0b" mode="bo1" />);

    expect(await screen.findByText("First player wins 51% of decided games (±5)", undefined, SLOW)).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
    expect(screen.getByTestId(statsTestid.gamesPanel).textContent).not.toMatch(/trap/i);
  });

  it("#2115 a rows skeleton stands in for the figures while the read is pending", async () => {
    vi.stubGlobal("fetch", vi.fn((_input: RequestInfo | URL) => Promise.resolve(answer(gamesResponse(FIGURES)))));

    render(<GamesPanel patch="v0.5.0b" mode="bo1" />);

    // Right after render the read has not answered.
    expectSkeleton(screen.getByText("Loading game figures…"), "rows");

    expect(await screen.findByText("First player wins 51% of decided games (±5)", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("Loading game figures…")).toBeNull();
    expect(screen.queryByTestId("skeleton")).toBeNull();
  });

  it("R1994000 a short sample reads as not enough games", async () => {
    for (const figures of [{ ...FIGURES, games: 12 }, null]) {
      vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(answer(gamesResponse(figures, 12)))));

      render(<GamesPanel patch="" mode="" />);

      const few = await screen.findByTestId(statsTestid.gamesFew, undefined, SLOW);
      expect(few).toHaveTextContent("Not enough games on this patch yet.");
      const panel = screen.getByTestId(statsTestid.gamesPanel);
      for (const line of FIGURE_LINES) expect(panel.textContent).not.toMatch(line);
      cleanup();
    }
  });

  it("R1994000 the panel says how many of the patch's games it counts", async () => {
    vi.stubGlobal("fetch", vi.fn(() => Promise.resolve(answer(gamesResponse({ ...FIGURES, games: 600 }, 1500)))));

    render(<GamesPanel patch="" mode="" />);

    expect(
      await screen.findByText(
        "Counted over 600 of the patch's 1,500 games: games recorded before these figures were kept are not in them.",
        undefined,
        SLOW,
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("First player wins 51% of decided games (±5)")).toBeInTheDocument();
  });

  it("R1994000 a failed load offers Try again", async () => {
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValueOnce(answer(gamesResponse(FIGURES)));
    vi.stubGlobal("fetch", fetchMock);

    render(<GamesPanel patch="" mode="" />);

    await screen.findByText("Game figures unavailable", undefined, SLOW);
    const retry = screen.getByTestId(statsTestid.gamesRetry);
    expect(retry).toHaveTextContent("Try again");
    fireEvent.click(retry);

    expect(await screen.findByText("First player wins 51% of decided games (±5)", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("Game figures unavailable")).toBeNull();
    expect(screen.queryByTestId(statsTestid.gamesRetry)).toBeNull();
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("R2567000 the first player's share reads over the decided games, with no draw figure", async () => {
    vi.stubGlobal("fetch", vi.fn((_input: RequestInfo | URL) => Promise.resolve(answer(gamesResponse(FIGURES)))));

    render(<GamesPanel patch="v0.5.0b" mode="bo1" />);

    expect(await screen.findByText("First player wins 51% of decided games (±5)", undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByTestId(statsTestid.gamesPanel).textContent).not.toMatch(/draw/i);
  });

  it("R2567000 every game drawn reads as an unknown share, not 0%", async () => {
    const allDrawn = { ...FIGURES, draws: 400, firstWins: 0, firstShare: null, firstLow: 0, firstHigh: 1 };
    vi.stubGlobal("fetch", vi.fn((_input: RequestInfo | URL) => Promise.resolve(answer(gamesResponse(allDrawn)))));

    render(<GamesPanel patch="v0.5.0b" mode="bo1" />);

    expect(await screen.findByText("First player wins: unknown, no game was won", undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByTestId(statsTestid.gamesPanel).textContent).not.toMatch(/First player wins \d/);
    expect(screen.getAllByRole("listitem")).toHaveLength(3);
  });
});
