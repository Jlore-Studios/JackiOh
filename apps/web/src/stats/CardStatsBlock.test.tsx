// Tests for CardStatsBlock (SPEC §9.11, R654, R3185000, R3447000, R4082000, R4083000).

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { INSPECT_STATS } from "../cards/inspect/testids.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { CardStatsBlock } from "./CardStatsBlock.tsx";
import { statsTestid } from "./testids.ts";

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
  decisiveness: 0.142,
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
  it("R3185000 shows Games, Tier, Adjusted win rate, Decisiveness, Win rate and Play rate, and nothing else of the server's", async () => {
    serve(SPARK_PUP);
    render(<CardStatsBlock cardId="core-001" />);

    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures()).toEqual([
      ["Games", "60"],
      ["Tier", "A"],
      // To a tenth of a point, as /stats prints it (R2777000).
      ["Adjusted win rate", "56.1%"],
      ["Decisiveness", "±14.2%"],
      ["Win rate", "60%"],
      ["Play rate", "50%"],
    ]);
    // The tier is /stats' chip.
    const chip = screen.getByTestId(statsTestid.cardStatsTier);
    expect(chip).toHaveClass("stats-tier");
    expect(chip).toHaveAttribute("data-tier", "A");
    expect(screen.getByTestId(statsTestid.cardStatsAdjusted)).toHaveTextContent("56.1%");
    for (const gone of ["When drawn", "Drawn vs not drawn", "not enough data"]) {
      expect(screen.queryByText(gone)).toBeNull();
    }
  });

  it("R4552003 the panel shows Decisiveness between Adjusted win rate and Win rate, its tooltip naming both values", async () => {
    const body = { ...mockStatsResponse(SPARK_PUP), tiers: { center: 0, spread: 0.03, cards: 100, bandLow: 0.45, bandHigh: 0.55, decisiveness: 0.137 } };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status: 200 })));
    render(<CardStatsBlock cardId="core-001" />);

    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures().map(([label]) => label)).toEqual(["Games", "Tier", "Adjusted win rate", "Decisiveness", "Win rate", "Play rate"]);
    expect(screen.getByTestId(statsTestid.cardStatsDecisiveness)).toHaveTextContent("±14.2%");
    const title = screen.getByText("Decisiveness").getAttribute("title") ?? "";
    expect(title).toContain("by ±14.2%");
    expect(title).toContain("A typical card: ±13.7%.");
    expect(title).toContain("not a margin of error");
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
      ["Decisiveness", "not enough games"],
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
    expect(screen.getByTestId(statsTestid.cardStatsTier)).toHaveTextContent("—");
    expect(screen.getByTestId(statsTestid.cardStatsTier)).not.toHaveClass("stats-tier");
    expect(screen.getByTestId(statsTestid.cardStatsAdjusted)).toHaveTextContent("—");
    expect(screen.getByText("60%")).toBeInTheDocument();
  });
});

describe("R4082000 a card no deck holds reads its Created figures", () => {
  /** A token's Created row: 50 seats made one, read against the other Created cards (R4083000: nothing else is sent). */
  const TOKEN = {
    id: "core-t-sheep",
    name: "Sheep",
    cost: 1,
    rarity: "token",
    set: "core",
    games: 50,
    adjustedWinRate: 0.4871,
    adjustedLow: 0.43,
    adjustedHigh: 0.54,
    decisiveness: 0.088,
    tier: "C",
    hasEnoughGames: true,
    window: { from: "0.2.0", liveGames: 1200, source: "live", counted: ["0.2.0"] },
  };

  function serveCreated(created: Record<string, unknown>, cards: Record<string, unknown>[] = []): void {
    const body = { ...mockStatsResponse(SPARK_PUP), cards, created: [created] };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status: 200 })));
  }

  it("R4083000 a token shows its games, tier, adjusted win rate and decisiveness alone, no Win rate or Play rate", async () => {
    serveCreated(TOKEN);
    render(<CardStatsBlock cardId="core-t-sheep" />);
    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures()).toEqual([
      ["Games", "50"],
      ["Tier", "C"],
      ["Adjusted win rate", "48.7%"],
      ["Decisiveness", "±8.8%"],
    ]);
    expect(screen.getByTestId(INSPECT_STATS)).not.toHaveTextContent(/Win rate|Play rate/);
  });

  it("R4552003 a card no deck holds shows Games, Tier, Adjusted win rate and Decisiveness", async () => {
    const created = { center: 0, spread: 0.03, cards: 4, baseline: 0.5, decisiveness: 0.1 };
    const body = {
      ...mockStatsResponse(SPARK_PUP),
      cards: [],
      created: [TOKEN],
      tiers: { center: 0, spread: 0.03, cards: 100, bandLow: 0.45, bandHigh: 0.55, decisiveness: 0.137, created },
    };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status: 200 })));
    render(<CardStatsBlock cardId="core-t-sheep" />);
    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures().map(([label]) => label)).toEqual(["Games", "Tier", "Adjusted win rate", "Decisiveness"]);
    // The typical value is the Created cards', not the deck cards'.
    expect(screen.getByText("Decisiveness").getAttribute("title")).toContain("A typical card: ±10.0%.");
  });

  it("R4083000 below the floor a token reads its games, — for tier and adjusted and not enough games for decisiveness", async () => {
    serveCreated({ ...TOKEN, games: 12, tier: null, hasEnoughGames: false });
    render(<CardStatsBlock cardId="core-t-sheep" />);
    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures()).toEqual([
      ["Games", "12"],
      ["Tier", "—"],
      ["Adjusted win rate", "—"],
      ["Decisiveness", "not enough games"],
    ]);
    expect(screen.getByTestId(INSPECT_STATS)).not.toHaveTextContent(/Win rate|Play rate/);
  });

  it("R4082000 a card a deck holds keeps its deck figures, Created copies or not", async () => {
    serveCreated({ ...TOKEN, id: "core-001" }, [SPARK_PUP]);
    render(<CardStatsBlock cardId="core-001" />);
    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(figures()[0]).toEqual(["Games", "60"]);
    expect(figures()[1]).toEqual(["Tier", "A"]);
    // R4083000: a deck card keeps its Win rate and Play rate.
    expect(figures().slice(4)).toEqual([
      ["Win rate", "60%"],
      ["Play rate", "50%"],
    ]);
  });
});

describe("R3447000 the panel names the patch its figures were collected on", () => {
  /** Newest first: 0.7.0, still gathering its games, then 0.6.0 and its micro. */
  const PATCHES = [
    { versions: ["0.7.0"], games: 300, liveGames: 300 },
    { versions: ["0.6.0", "0.6.0b"], games: 4000, liveGames: 2000 },
  ];

  /** Serves Spark Pup at the newest patch with `cardWindow` as its window, and the patch list when given. */
  function serveWindow(cardWindow: Record<string, unknown> | undefined, patches?: readonly unknown[]): void {
    const body = { ...mockStatsResponse({ ...SPARK_PUP, window: cardWindow }), patch: "0.7.0", patches };
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status: 200 })));
  }

  it("R3447000 while a newer patch gathers its games, the previous window's figures name the patch they were collected on", async () => {
    serveWindow({ from: "0.7.0", liveGames: 300, source: "previous", counted: ["0.6.0", "0.6.0b"] }, PATCHES);
    render(<CardStatsBlock cardId="core-001" />);

    const patch = await screen.findByTestId(statsTestid.cardStatsPatch, undefined, SLOW);
    expect(patch).toHaveTextContent("Collected on patch 0.6.0");
    // Beside the title, above the same six figures, and with no word of where they come from (R661).
    expect(patch.closest(".inspect-stats-header")).toHaveTextContent("Statistics");
    expect(figures().map(([label]) => label)).toEqual(["Games", "Tier", "Adjusted win rate", "Decisiveness", "Win rate", "Play rate"]);
    expect(screen.getByTestId(INSPECT_STATS).textContent).not.toMatch(/previous|live games/i);
  });

  it("R3447000 a card's own window names the patch shown", async () => {
    serveWindow({ from: "0.7.0", liveGames: 1200, source: "live", counted: ["0.7.0"] }, PATCHES);
    render(<CardStatsBlock cardId="core-001" />);

    expect(await screen.findByTestId(statsTestid.cardStatsPatch, undefined, SLOW)).toHaveTextContent("Collected on patch 0.7.0");
  });

  it("R3447000 a window a micro opened names the patch the micro is part of", async () => {
    serveWindow({ from: "0.6.0b", liveGames: 1500, source: "live", counted: ["0.6.0b"] }, PATCHES);
    render(<CardStatsBlock cardId="core-001" />);

    expect(await screen.findByTestId(statsTestid.cardStatsPatch, undefined, SLOW)).toHaveTextContent("Collected on patch 0.6.0");
  });

  it("R3447000 from a server with no patch list, the first version the figures count", async () => {
    serveWindow({ from: "0.7.0", liveGames: 300, source: "previous", counted: ["0.6.0b"] });
    render(<CardStatsBlock cardId="core-001" />);

    expect(await screen.findByTestId(statsTestid.cardStatsPatch, undefined, SLOW)).toHaveTextContent("Collected on patch 0.6.0b");
  });

  it("R3447000 a card with no window, as from an older server, names no patch", async () => {
    serveWindow(undefined, PATCHES);
    render(<CardStatsBlock cardId="core-001" />);

    await screen.findByText("Adjusted win rate", undefined, SLOW);
    expect(screen.queryByTestId(statsTestid.cardStatsPatch)).toBeNull();
    expect(screen.queryByText(/Collected on patch/)).toBeNull();
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
