// Tests for CardStatsBlock (SPEC §9.11, R377, R654).

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_STATS } from "../cards/inspect/testids.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { CardStatsBlock } from "./CardStatsBlock.tsx";

const SLOW = { timeout: 10_000 } as const;

function mockStatsResponse(card: Record<string, unknown>, minSample = 20) {
  return {
    patch: "0.2.0",
    previousPatch: null,
    gate: { cleared: true, liveGames: 1200, minLiveGames: 1000 },
    source: "live" as const,
    sourceLabel: "Live games",
    pilot: "merged" as const,
    pilotLabel: "All pilots",
    minSample,
    totalGames: 1200,
    cards: [card],
    summary: {
      totalGames: 1200,
      liveGames: 1200,
      activePatch: "0.2.0",
      source: "live" as const,
      pilot: "merged" as const,
      bestCard: null,
      worstCard: null,
    },
  };
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const SPARK_PUP = {
  id: "core-001",
  name: "Spark Pup",
  cost: 1,
  rarity: "Common",
  set: "core",
  games: 60,
  winRate: 0.6,
  drawnGames: 40,
  drawnWinRate: 0.65,
  playedGames: 30,
  playedWinRate: 0.7,
  drawnNotPlayedGames: 10,
  drawnNotPlayedWinRate: 0.5,
  playedDelta: 0.2,
  notDrawnGames: 20,
  notDrawnWinRate: 0.5,
  drawnDelta: 0.15,
  playRate: 0.5,
  hasEnoughGames: true,
};

describe("R377 CardStatsBlock", () => {
  it("R2427000 renders Drawn vs not drawn with both sides' games above the sample floor", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify(mockStatsResponse(SPARK_PUP)),
          { status: 200 },
        ),
      ),
    );

    render(<CardStatsBlock cardId="core-001" />);

    expect(await screen.findByText("Drawn vs not drawn", undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByText("+15%")).toBeInTheDocument();
    expect(screen.getByText("(40 / 20)")).toBeInTheDocument();
    expect(screen.queryByText("Played vs held")).not.toBeInTheDocument();
  });

  it("#2115 a panel skeleton stands in for the figures while the read is pending", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify(mockStatsResponse(SPARK_PUP)),
          { status: 200 },
        ),
      ),
    );

    render(<CardStatsBlock cardId="core-001" />);

    // Right after render the read has not answered: the block keeps its frame and its title.
    const block = screen.getByTestId(INSPECT_STATS);
    expectSkeleton(within(block).getByText("Loading stats…"), "panel");

    expect(await screen.findByText("Drawn vs not drawn", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("Loading stats…")).toBeNull();
    expect(screen.queryByTestId("skeleton")).toBeNull();
  });

  it("R654 renders not enough data for Drawn vs not drawn when below sample floor", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify(
            mockStatsResponse({
              id: "core-002",
              name: "Lava Hound",
              cost: 4,
              rarity: "Rare",
              set: "core",
              games: 10,
              winRate: 0.9,
              drawnGames: 8,
              drawnWinRate: 0.9,
              playedGames: 5,
              playedWinRate: 1.0,
              drawnNotPlayedGames: 3,
              drawnNotPlayedWinRate: 0.733,
              playedDelta: 0.267,
              notDrawnGames: 2,
              notDrawnWinRate: 1.0,
              drawnDelta: -0.1,
              playRate: 0.1,
              hasEnoughGames: false,
            }),
          ),
          { status: 200 },
        ),
      ),
    );

    render(<CardStatsBlock cardId="core-002" />);

    expect(await screen.findByText("Drawn vs not drawn", undefined, SLOW)).toBeInTheDocument();
    // Delta should display "not enough data" and not any misleading percentage
    expect(screen.getByText("not enough data")).toBeInTheDocument();
    expect(screen.queryByText("-10%")).not.toBeInTheDocument();
    expect(screen.queryByText("(8 / 2)")).not.toBeInTheDocument();
  });

  it("R654 renders not enough data for Drawn vs not drawn when the server withholds the delta", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        new Response(
          JSON.stringify(
            mockStatsResponse({
              id: "core-003",
              name: "Aqua Warden",
              cost: 2,
              rarity: "Common",
              set: "core",
              games: 30,
              winRate: 0.5,
              drawnGames: 30,
              drawnWinRate: 0.5,
              playedGames: 0,
              playedWinRate: null,
              drawnNotPlayedGames: 30,
              drawnNotPlayedWinRate: 0.5,
              playedDelta: null,
              notDrawnGames: 0,
              notDrawnWinRate: null,
              drawnDelta: null,
              playRate: 0.3,
              hasEnoughGames: true,
            }),
          ),
          { status: 200 },
        ),
      ),
    );

    render(<CardStatsBlock cardId="core-003" />);

    expect(await screen.findByText("Drawn vs not drawn", undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByText("not enough data")).toBeInTheDocument();
    expect(screen.queryByText("%")).not.toBeInTheDocument();
  });
});

describe("CardStatsBlock after a load that failed", () => {
  it("offers Try again, which loads the card's statistics again", async () => {
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValueOnce(new Response(JSON.stringify(mockStatsResponse(SPARK_PUP)), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    render(<CardStatsBlock cardId="core-001" />);

    await screen.findByText("Statistics unavailable", undefined, SLOW);
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    await screen.findByText("Drawn vs not drawn", undefined, SLOW);
    expect(screen.getByText("+15%")).toBeInTheDocument();
    expect(screen.queryByText("Statistics unavailable")).toBeNull();
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("offers no Try again when the server has no statistics for the card", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response(JSON.stringify(mockStatsResponse({ ...SPARK_PUP, id: "core-002" })), { status: 200 })),
    );

    render(<CardStatsBlock cardId="core-001" />);

    await screen.findByText("Statistics unavailable", undefined, SLOW);
    expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
  });
});
