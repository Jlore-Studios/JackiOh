// Tests for the public statistics page (SPEC §9.11, R654).

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { paths } from "../net/navigate.ts";
import { SESSION_STORAGE_KEY } from "../net/session.ts";
import { recordGame, resetPlayerStats } from "../stats/store.ts";
import { statsTestid } from "../stats/testids.ts";
import { SiteFooter, siteFooterTestid } from "./SiteFooter.tsx";
import StatsRoute from "./stats.tsx";

const { documentTitleFor } = await import("../main.tsx");

const SLOW = { timeout: 10_000 } as const;

const MOCK_CARD_STATS_PROVISIONAL = {
  patch: "0.2.0",
  previousPatch: "0.1.0",
  gate: {
    cleared: false,
    liveGames: 412,
    minLiveGames: 1000,
  },
  source: "provisional",
  sourceLabel: "AI games + live games (provisional)",
  minSample: 20,
  totalGames: 1200,
  cards: [
    {
      id: "core-001",
      name: "Spark Pup",
      cost: 1,
      rarity: "Common",
      set: "core",
      games: 50,
      winRate: 0.6,
      drawnGames: 40,
      drawnWinRate: 0.65,
      playedGames: 30,
      playedWinRate: 0.7,
      playRate: 0.5,
      hasEnoughGames: true,
    },
    {
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
      playRate: 0.1,
      hasEnoughGames: false, // Below sample threshold (20)
    },
  ],
  summary: {
    totalGames: 1200,
    liveGames: 412,
    activePatch: "0.2.0",
    source: "provisional",
    bestCard: { id: "core-001", name: "Spark Pup", winRate: 0.6, games: 50 },
    worstCard: { id: "core-001", name: "Spark Pup", winRate: 0.6, games: 50 },
  },
};

const MOCK_CARD_STATS_CLEARED = {
  ...MOCK_CARD_STATS_PROVISIONAL,
  patch: "0.1.0",
  gate: {
    cleared: true,
    liveGames: 1050,
    minLiveGames: 1000,
  },
  source: "live",
  sourceLabel: "Live games",
  summary: {
    ...MOCK_CARD_STATS_PROVISIONAL.summary,
    totalGames: 1050,
    liveGames: 1050,
    activePatch: "0.1.0",
    source: "live",
  },
};

const MOCK_PLAYERS = {
  players: [
    {
      profileId: "profile-alpha",
      displayName: "AlphaPlayer",
      games: 45,
      wins: 30,
      losses: 15,
      draws: 0,
      winRate: 0.67,
      favouriteCards: [{ id: "core-001", count: 20 }],
      funStats: {
        nemesisCardId: "core-002",
        totalDestroyed: 40,
        totalDefeated: 35,
      },
      updatedAt: 1234567890,
    },
  ],
  page: 1,
  limit: 50,
};

const MOCK_DRILL_DOWN = {
  card: { id: "core-001", name: "Spark Pup", cost: 1, rarity: "Common" },
  patches: [{ patch: "0.1.0", games: 100, winRate: 0.58 }],
  byTurn: [
    { turn: 1, games: 25, winRate: 0.64 },
    { turn: 2, games: 15, winRate: 0.53 },
  ],
  coPlayed: [{ id: "core-002", name: "Lava Hound", games: 20, winRate: 0.7 }],
};

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

beforeEach(() => {
  window.localStorage.clear();
  resetPlayerStats();
  at("/stats");

  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/stats/cards/core-001")) {
        return Promise.resolve(new Response(JSON.stringify(MOCK_DRILL_DOWN), { status: 200 }));
      }
      if (url.includes("/api/stats/cards?patch=0.1.0")) {
        return Promise.resolve(
          new Response(JSON.stringify(MOCK_CARD_STATS_CLEARED), { status: 200 }),
        );
      }
      if (url.includes("/api/stats/cards")) {
        return Promise.resolve(
          new Response(JSON.stringify(MOCK_CARD_STATS_PROVISIONAL), { status: 200 }),
        );
      }
      if (url.includes("/api/stats/players")) {
        return Promise.resolve(new Response(JSON.stringify(MOCK_PLAYERS), { status: 200 }));
      }
      return Promise.resolve(new Response("{}", { status: 200 }));
    }),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  document.head.querySelector('link[rel="canonical"]')?.remove();
});

describe("R654 public statistics route", () => {
  it("R654 is served at /stats signed out with document title", () => {
    expect(paths.stats).toBe("/stats");
    expect(documentTitleFor(paths.stats)).toBe("Statistics · JackiOh");
  });

  it("R654 renders summary tiles and provisional banner when below 1000 live games", async () => {
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.screen, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(statsTestid.summaryTiles, undefined, SLOW)).toBeInTheDocument();

    // R661: one tile counts the patch's games, never the gate's live games (412).
    const games = screen.getByTestId(statsTestid.summaryTotalGames);
    expect(games).toHaveTextContent("Games on patch 0.2.0");
    expect(games).toHaveTextContent("1,200");
    expect(games).not.toHaveTextContent("412");

    const banner = screen.getByTestId(statsTestid.provisionalBanner);
    expect(banner).toHaveTextContent("Provisional statistics");
  });

  it("R654 cards table displays 'not enough games' below sample floor and percentage when at or above sample floor", async () => {
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    // core-001 has 50 games (>= 20) -> displays 60%
    expect(screen.getByText("Spark Pup")).toBeInTheDocument();
    expect(screen.getByText("60%")).toBeInTheDocument();

    // core-002 has 10 games (< 20) -> displays "not enough games"
    expect(screen.getByText("Lava Hound")).toBeInTheDocument();
    expect(screen.getByText("not enough games")).toBeInTheDocument();
  });

  it("R654 sorts cards by column on header click and keyboard activation", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    const nameHeader = screen.getByRole("columnheader", { name: /Name/i });
    expect(nameHeader).toHaveAttribute("aria-sort", "none");

    // Click to sort ascending by name
    await user.click(nameHeader);
    expect(nameHeader).toHaveAttribute("aria-sort", "ascending");

    // Press Enter to toggle sort direction
    fireEvent.keyDown(nameHeader, { key: "Enter" });
    expect(nameHeader).toHaveAttribute("aria-sort", "descending");
  });

  it("R654 filters cards by search input", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByText("Spark Pup")).toBeInTheDocument();
    expect(screen.getByText("Lava Hound")).toBeInTheDocument();

    const searchInput = screen.getByTestId(statsTestid.searchInput);
    await user.type(searchInput, "Lava");

    expect(screen.queryByText("Spark Pup")).not.toBeInTheDocument();
    expect(screen.getByText("Lava Hound")).toBeInTheDocument();
  });

  it("R654 opens card drill-down modal on row click and closes on close button", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    // Click on Spark Pup row
    const sparkRow = screen.getByText("Spark Pup");
    await user.click(sparkRow);

    const modal = await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW);
    expect(modal).toBeInTheDocument();
    expect(await screen.findByText("Win rate by patch")).toBeInTheDocument();
    expect(await screen.findByText("Win rate by turn played")).toBeInTheDocument();
    expect(await screen.findByText("Co-played cards (synergy)")).toBeInTheDocument();

    // Close modal
    const closeBtn = screen.getByTestId(statsTestid.drillDownClose);
    await user.click(closeBtn);
    expect(screen.queryByTestId(statsTestid.drillDownModal)).not.toBeInTheDocument();
  });

  it("R654 players tab lists public player aggregates and keeps Elo separate", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    const playersTab = await screen.findByTestId(statsTestid.tabPlayers, undefined, SLOW);
    await user.click(playersTab);

    expect(await screen.findByTestId(statsTestid.playersTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByText("AlphaPlayer")).toBeInTheDocument();
    expect(screen.getByText("67%")).toBeInTheDocument();
    expect(screen.getByText("Big D-fender")).toBeInTheDocument();

    // Table does not expose Elo or rating
    const table = screen.getByTestId(statsTestid.playersTable);
    expect(within(table).queryByText(/elo/i)).not.toBeInTheDocument();
    expect(within(table).queryByText(/rating/i)).not.toBeInTheDocument();
  });

  it("R654 fallback button switches to previous patch live data when available", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    const fallbackBtn = await screen.findByTestId(statsTestid.fallbackToggle, undefined, SLOW);
    expect(fallbackBtn).toHaveTextContent("View previous patch (0.1.0) →");

    await user.click(fallbackBtn);

    await waitFor(() => {
      expect(screen.getByTestId(statsTestid.summaryTotalGames)).toHaveTextContent("Games on patch 0.1.0");
    });
    expect(screen.getByTestId(statsTestid.summaryTotalGames)).toHaveTextContent("1,050");
    expect(screen.getByTestId(statsTestid.summaryTotalGames)).not.toHaveTextContent("live");
  });

  it("R654 shows player's personal stats when signed in and local stats exist", async () => {
    // Set mock active session
    window.localStorage.setItem(
      SESSION_STORAGE_KEY,
      JSON.stringify({ accessToken: "active-token", refreshToken: "refresh", expiresAt: 9999999999 }),
    );
    // Record game with core-001 played
    recordGame(
      {
        seen: ["core-001"],
        played: { "core-001": 5 },
        playedAgainst: {},
        destroyed: {},
        defeated: {},
      },
      "win",
    );

    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByText(/5×/)).toBeInTheDocument();
  });

  it("R654 site footer links to stats page", () => {
    render(<SiteFooter />);
    const footerLink = screen.getByTestId(siteFooterTestid.stats);
    expect(footerLink).toHaveAttribute("href", "/stats");
    expect(footerLink).toHaveTextContent("Stats");
  });

  it("R661 shows no data source, no count towards the gate and no word of AI padding or a gate, in the banner, the notes and the drill-down", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    const page = await screen.findByTestId(statsTestid.screen, undefined, SLOW);
    const banner = await screen.findByTestId(statsTestid.provisionalBanner, undefined, SLOW);

    // The banner says only that the figures are provisional (R661).
    expect(banner).toHaveTextContent("Provisional statistics");
    expect(banner.textContent).not.toContain("1000");
    expect(banner.textContent).not.toMatch(/live games|AI/);
    expect(banner.querySelector(".stats-provisional-progress")).toBeNull();

    // No tile, badge or note names a source or the gate anywhere on the page (R661).
    expect(page.querySelector(".stats-summary-badge")).toBeNull();
    expect(page.textContent).not.toContain("Data source");
    expect(page.textContent).not.toContain("AI games + live games (provisional)");
    expect(page.textContent).not.toMatch(/AI development|publication gate|cleared gate|\blive\b/i);
    // The patch's games still show, once, with no live count beside them (R661).
    const tiles = screen.getByTestId(statsTestid.summaryTiles);
    expect(within(tiles).getAllByText("1,200")).toHaveLength(1);
    expect(tiles).toHaveTextContent("Games on patch 0.2.0");
    expect(page.textContent).not.toContain("412");

    // The drill-down names no gate either (R661).
    await user.click(await screen.findByText("Spark Pup"));
    const modal = await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW);
    expect(modal.textContent).not.toMatch(/gate/i);
    expect(within(modal).getByText("Win rate by patch")).toBeInTheDocument();
  });
});
