// Tests for CardStatsBlock (SPEC §9.11, R654, R3185000).

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_STATS, INSPECT_STATS_ADJUSTED, INSPECT_STATS_TIER } from "../cards/inspect/testids.ts";
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
  adjustedWinRate: 0.5612,
  adjustedLow: 0.51,
  adjustedHigh: 0.61,
  tier: "A",
  playRate: 0.5,
  hasEnoughGames: true,
};

/** Serves `card` as the card statistics' only row. */
function serve(card: Record<string, unknown>): void {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(mockStatsResponse(card)), { status: 200 })));
}

/** Each figure's label and what it reads, in the panel's order. */
function figures(): [string, string][] {
  return [...screen.getByTestId(INSPECT_STATS).querySelectorAll(".inspect-stats-metric")].map((metric) => [
    metric.querySelector(".inspect-stats-metric-label")?.textContent ?? "",
    metric.querySelector(".inspect-stats-metric-value")?.textContent ?? "",
  ]);
}

describe("R3185000 CardStatsBlock", () => {
  it("R3185000 shows Games, Tier, Adjusted win rate, Win rate and Play rate, and nothing else of the server's", async () => {
    serve(SPARK_PUP);
    render(<CardStatsBlock cardId="core-001" />);

    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures()).toEqual([
      ["Games", "60"],
      ["Tier", "A"],
      // To a tenth of a point, as /stats prints it (R2777000).
      ["Adjusted win rate", "56.1%"],
      ["Win rate", "60%"],
      ["Play rate", "50%"],
    ]);
    // The tier is /stats' chip.
    const chip = screen.getByTestId(INSPECT_STATS_TIER);
    expect(chip).toHaveClass("stats-tier");
    expect(chip).toHaveAttribute("data-tier", "A");
    expect(screen.getByTestId(INSPECT_STATS_ADJUSTED)).toHaveTextContent("56.1%");
    for (const gone of ["When drawn", "Drawn vs not drawn", "not enough data"]) {
      expect(screen.queryByText(gone)).toBeNull();
    }
  });

  it("#2115 a panel skeleton stands in for the figures while the read is pending", async () => {
    serve(SPARK_PUP);
    render(<CardStatsBlock cardId="core-001" />);

    // Right after render the read has not answered: the block keeps its frame and its title.
    const block = screen.getByTestId(INSPECT_STATS);
    expectSkeleton(within(block).getByText("Loading stats…"), "panel");

    expect(await screen.findByText("Adjusted win rate", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("Loading stats…")).toBeNull();
    expect(screen.queryByTestId("skeleton")).toBeNull();
  });

  it("R3185000 R654 below the floor, Win rate reads not enough games and the tier and the adjusted rate read —", async () => {
    serve({
      id: "core-002",
      name: "Lava Hound",
      cost: 4,
      rarity: "Rare",
      set: "core",
      games: 10,
      winRate: 0.9,
      adjustedWinRate: 0.497,
      tier: "F",
      playRate: 0.1,
      hasEnoughGames: false,
    });
    render(<CardStatsBlock cardId="core-002" />);

    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures()).toEqual([
      ["Games", "10"],
      ["Tier", "—"],
      ["Adjusted win rate", "—"],
      ["Win rate", "not enough games (< 20)"],
      ["Play rate", "10%"],
    ]);
    expect(screen.queryByText("90%")).toBeNull();
  });

  it("R3185000 a card with no tier or adjusted rate, as from an older server, reads — for both", async () => {
    const { adjustedWinRate: _rate, adjustedLow: _low, adjustedHigh: _high, tier: _tier, ...older } = SPARK_PUP;
    serve(older);
    render(<CardStatsBlock cardId="core-001" />);

    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(screen.getByTestId(INSPECT_STATS_TIER)).toHaveTextContent("—");
    expect(screen.getByTestId(INSPECT_STATS_TIER)).not.toHaveClass("stats-tier");
    expect(screen.getByTestId(INSPECT_STATS_ADJUSTED)).toHaveTextContent("—");
    expect(screen.getByText("60%")).toBeInTheDocument();
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

    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(screen.getByText("56.1%")).toBeInTheDocument();
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
