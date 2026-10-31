// Tests for the public statistics page (SPEC §9.11, R654).

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SHIPPED_SETS } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { cardStatsPath } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { rememberRead } from "../net/readCache.ts";
import { SESSION_STORAGE_KEY } from "../net/session.ts";
import { PLAYER_SEARCH_DEBOUNCE_MS } from "../stats/config.ts";
import { EMPTY_LOG } from "../stats/model.ts";
import { recordGame, resetPlayerStats } from "../stats/store.ts";
import { statsTestid } from "../stats/testids.ts";
import { expectEmptyState } from "../test/empty.ts";
import { pressBack } from "../test/history.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { at } from "../test/stubServer.ts";
import { SiteFooter, siteFooterTestid } from "./SiteFooter.tsx";
import StatsRoute from "./stats.tsx";

const { documentTitleFor } = await import("../main.tsx");

const SLOW = { timeout: 10_000 } as const;
const HERE = dirname(fileURLToPath(import.meta.url));

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
      drawnNotPlayedGames: 10,
      drawnNotPlayedWinRate: 0.5,
      playedDelta: 0.2,
      notDrawnGames: 10,
      notDrawnWinRate: 0.4,
      drawnDelta: 0.25,
      playRate: 0.5,
      hasEnoughGames: true,
      playedRadiantGames: 30,
      playedRadiantWinRate: 0.7,
      playedNormalGames: 20,
      playedNormalWinRate: 0.4,
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
      drawnNotPlayedGames: 3,
      drawnNotPlayedWinRate: 0.733,
      playedDelta: 0.267,
      notDrawnGames: 2,
      notDrawnWinRate: 1.0,
      drawnDelta: -0.1,
      playRate: 0.1,
      hasEnoughGames: false, // Below sample threshold (20)
      playedRadiantGames: 5,
      playedRadiantWinRate: 1.0,
      playedNormalGames: 5,
      playedNormalWinRate: 1.0,
    },
    {
      id: "core-003",
      name: "Aqua Warden",
      cost: 2,
      rarity: "Common",
      set: "core",
      games: 30,
      winRate: 0.5,
      drawnGames: 25,
      drawnWinRate: 0.45,
      playedGames: 20,
      playedWinRate: 0.4,
      drawnNotPlayedGames: 5,
      drawnNotPlayedWinRate: 0.5,
      playedDelta: -0.1,
      notDrawnGames: 5,
      notDrawnWinRate: 0.6,
      drawnDelta: -0.15,
      playRate: 0.3,
      hasEnoughGames: true,
      playedRadiantGames: 20,
      playedRadiantWinRate: 0.4,
      playedNormalGames: 10,
      playedNormalWinRate: 0.5,
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

const MOCK_KEYWORD_CARDS = {
  ...MOCK_CARD_STATS_PROVISIONAL,
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
      drawnNotPlayedGames: 10,
      drawnNotPlayedWinRate: 0.5,
      playedDelta: 0.2,
      playRate: 0.5,
      hasEnoughGames: true,
      playedRadiantGames: 30,
      playedRadiantWinRate: 0.7,
      playedNormalGames: 20,
      playedNormalWinRate: 0.4,
    },
    {
      id: "core-019",
      name: "Midrange Menace",
      cost: 3,
      rarity: "Common",
      set: "core",
      games: 40,
      winRate: 0.55,
      drawnGames: 35,
      drawnWinRate: 0.5,
      playedGames: 25,
      playedWinRate: 0.6,
      drawnNotPlayedGames: 10,
      drawnNotPlayedWinRate: 0.5,
      playedDelta: 0.1,
      playRate: 0.4,
      hasEnoughGames: true,
      playedRadiantGames: 25,
      playedRadiantWinRate: 0.6,
      playedNormalGames: 15,
      playedNormalWinRate: 0.45,
    },
    {
      id: "core-032",
      name: "Prem Panther",
      cost: 4,
      rarity: "Rare",
      set: "core",
      games: 30,
      winRate: 0.5,
      drawnGames: 25,
      drawnWinRate: 0.45,
      playedGames: 20,
      playedWinRate: 0.5,
      drawnNotPlayedGames: 5,
      drawnNotPlayedWinRate: 0.5,
      playedDelta: 0.0,
      playRate: 0.3,
      hasEnoughGames: true,
      playedRadiantGames: 10,
      playedRadiantWinRate: 0.8,
      playedNormalGames: 20,
      playedNormalWinRate: 0.5,
    },
    {
      id: "core-055",
      name: "Lava Golem",
      cost: 5,
      rarity: "Epic",
      set: "core",
      games: 25,
      winRate: 0.45,
      drawnGames: 20,
      drawnWinRate: 0.4,
      playedGames: 15,
      playedWinRate: 0.45,
      drawnNotPlayedGames: 5,
      drawnNotPlayedWinRate: 0.4,
      playedDelta: 0.05,
      playRate: 0.2,
      hasEnoughGames: true,
      playedRadiantGames: 0,
      playedRadiantWinRate: null,
      playedNormalGames: 25,
      playedNormalWinRate: 0.45,
    },
  ],
};

const MOCK_CARD_STATS_EMPTY = {
  ...MOCK_CARD_STATS_PROVISIONAL,
  patch: "0.2.0",
  totalGames: 0,
  cards: MOCK_CARD_STATS_PROVISIONAL.cards.map((card) => ({
    ...card,
    games: 0,
    winRate: null,
    drawnGames: 0,
    drawnWinRate: null,
    playedGames: 0,
    playedWinRate: null,
    drawnNotPlayedGames: 0,
    drawnNotPlayedWinRate: null,
    playedDelta: null,
    notDrawnGames: 0,
    notDrawnWinRate: null,
    drawnDelta: null,
    playRate: 0,
    hasEnoughGames: false,
    playedRadiantGames: 0,
    playedRadiantWinRate: null,
    playedNormalGames: 0,
    playedNormalWinRate: null,
  })),
  summary: {
    ...MOCK_CARD_STATS_PROVISIONAL.summary,
    totalGames: 0,
    bestCard: null,
    worstCard: null,
  },
};

const MOCK_PLAYERS = {
  players: [
    {
      profileId: "profile-alpha",
      username: "AlphaPlayer",
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

type HeldRequest = {
  url: URL;
  signal: AbortSignal | undefined;
  answer: (body: unknown, status?: number) => void;
};

/**
 * Stubs `fetch`: a request selected by `holds` waits until the test answers it and, as `fetch` does, rejects
 * the moment its signal aborts; any other is answered at once with the provisional card statistics.
 */
function holdFetch(holds: (url: URL) => boolean): HeldRequest[] {
  const held: HeldRequest[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = new URL(String(input), window.location.origin);
      if (!holds(url)) {
        return Promise.resolve(new Response(JSON.stringify(MOCK_CARD_STATS_PROVISIONAL), { status: 200 }));
      }
      return new Promise<Response>((resolve, reject) => {
        const signal = init?.signal ?? undefined;
        signal?.addEventListener(
          "abort",
          () => reject(new DOMException("The operation was aborted.", "AbortError")),
          { once: true },
        );
        held.push({
          url,
          signal,
          answer: (body, status = 200) => resolve(new Response(JSON.stringify(body), { status })),
        });
      });
    }),
  );
  return held;
}

function requested(pathname: string): URL[] {
  return vi
    .mocked(fetch)
    .mock.calls.map(([input]) => new URL(String(input), window.location.origin))
    .filter((url) => url.pathname === pathname);
}

/** A key pressed wherever the focus is: the drill-down reads Escape and Tab at the window (ConfirmConcede.test.tsx). */
function press(key: string, options: { shiftKey?: boolean } = {}): void {
  fireEvent.keyDown(document.activeElement ?? document.body, { key, ...options });
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
      if (url.includes("/api/stats/cards") && url.includes("mode=random")) {
        return Promise.resolve(
          new Response(JSON.stringify(MOCK_CARD_STATS_EMPTY), { status: 200 }),
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
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.head.querySelector('link[rel="canonical"]')?.remove();
});

describe("R654 public statistics route", () => {
  it("R654 is served at /stats signed out with document title", () => {
    expect(paths.stats).toBe("/stats");
    expect(documentTitleFor(paths.stats)).toBe("Statistics · JackiOh");
  });

  it("R2325000 R654 shows no provisional banner and no reading notes while the source is provisional", async () => {
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.screen, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(statsTestid.summaryTiles, undefined, SLOW)).toBeInTheDocument();

    // R661: one tile counts the patch's games, never the gate's live games (412).
    const games = screen.getByTestId(statsTestid.summaryTotalGames);
    expect(games).toHaveTextContent("Games on patch 0.2.0");
    expect(games).toHaveTextContent("1,200");
    expect(games).not.toHaveTextContent("412");

    // R2325000: no banner and no reading notes even while the source is provisional.
    expect(screen.queryByTestId("stats-provisional-banner")).toBeNull();
    expect(screen.queryByText("Provisional statistics")).toBeNull();
    expect(screen.queryByText("How to read these statistics")).toBeNull();
    expect(document.body.textContent).not.toContain("AI games + live games (provisional)");
  });

  it("R1646000 the fixed top keeps one summary tile, the patch's games, and no top or lowest card", async () => {
    render(<StatsRoute />);

    const tiles = await screen.findByTestId(statsTestid.summaryTiles, undefined, SLOW);
    expect(tiles.children).toHaveLength(1);
    expect(within(tiles).getByTestId(statsTestid.summaryTotalGames)).toHaveTextContent("Games on patch 0.2.0");
    expect(tiles).not.toHaveTextContent("Top card");
    expect(tiles).not.toHaveTextContent("Lowest card");
    expect(tiles).not.toHaveTextContent("Spark Pup");
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

  it("#2115 a table skeleton stands in for the players while the read is pending", async () => {
    const held = holdFetch((url) => url.pathname === "/api/stats/players");
    render(<StatsRoute />);

    fireEvent.click(await screen.findByTestId(statsTestid.tabPlayers, undefined, SLOW));
    expectSkeleton(await screen.findByText("Loading players…", undefined, SLOW), "table");
    await waitFor(() => expect(held).toHaveLength(1), SLOW);

    await act(async () => {
      held[0]!.answer(MOCK_PLAYERS);
    });
    expect(await screen.findByTestId(statsTestid.playersTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("Loading players…")).toBeNull();
  });

  it("R1436 players tab shows usernames, isolated, never a profile id nor any slice of one, and searches by username", async () => {
    const ids = ["7c1d2e3f-4a5b-4c6d-8e9f-0a1b2c3d4e5f", "0f9e8d7c-6b5a-4f4e-9d3c-2b1a0f9e8d7c"];
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/stats/players")) {
        const players = [
          { ...MOCK_PLAYERS.players[0], profileId: ids[0], username: "Max#3" },
          { ...MOCK_PLAYERS.players[0], profileId: ids[1], username: "محمد" },
        ];
        return Promise.resolve(new Response(JSON.stringify({ ...MOCK_PLAYERS, players }), { status: 200 }));
      }
      return Promise.resolve(new Response(JSON.stringify(MOCK_CARD_STATS_PROVISIONAL), { status: 200 }));
    });
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();
    const { container } = render(<StatsRoute />);

    await user.click(await screen.findByTestId(statsTestid.tabPlayers, undefined, SLOW));
    const table = await screen.findByTestId(statsTestid.playersTable, undefined, SLOW);
    const names = [...table.querySelectorAll("bdi.username")].map((name) => name.getAttribute("title"));
    expect(names).toEqual(["Max#3", "محمد"]);
    const text = container.textContent;
    for (const id of ids) {
      expect(text).not.toContain(id);
      expect(text).not.toContain(id.slice(0, 8));
    }

    // The search box asks the server by username; the folding is the server's (R654, R1436).
    const search = screen.getByPlaceholderText("Search player by username…");
    fireEvent.change(search, { target: { value: "max" } });
    await waitFor(() => {
      expect(fetchMock.mock.calls.some(([input]) => String(input).includes("/api/stats/players?search=max"))).toBe(true);
    });
  });

  it("R654 the set filter lists every shipped set and asks the server for each one's own value", async () => {
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);

    const select = screen.getByLabelText("Filter by set") as HTMLSelectElement;
    expect([...select.options].map((option) => [option.value, option.textContent])).toEqual([
      ["", "All sets"],
      ...SHIPPED_SETS.map((set) => [set.toLowerCase(), set]),
    ]);

    const asked = (): (string | null)[] =>
      vi
        .mocked(fetch)
        .mock.calls.map(([input]) => new URL(String(input), window.location.origin))
        .filter((url) => url.pathname === "/api/stats/cards")
        .map((url) => url.searchParams.get("set"));
    for (const set of SHIPPED_SETS) {
      fireEvent.change(select, { target: { value: set.toLowerCase() } });
      await waitFor(() => expect(asked()).toContain(set.toLowerCase()));
    }
  });

  it("R654 the player search asks the server once per pause in the typing, not once per keystroke", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);
    await user.click(await screen.findByTestId(statsTestid.tabPlayers, undefined, SLOW));
    await screen.findByTestId(statsTestid.playersTable, undefined, SLOW);

    const searched = (): (string | null)[] =>
      vi
        .mocked(fetch)
        .mock.calls.map(([input]) => new URL(String(input), window.location.origin))
        .filter((url) => url.pathname === "/api/stats/players" && url.searchParams.has("search"))
        .map((url) => url.searchParams.get("search"));
    const box = screen.getByPlaceholderText("Search player by username…");
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    for (const text of ["a", "al", "ali", "alic", "alice"]) {
      fireEvent.change(box, { target: { value: text } });
    }
    expect(searched()).toEqual([]);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(PLAYER_SEARCH_DEBOUNCE_MS - 1);
    });
    expect(searched()).toEqual([]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(searched()).toEqual(["alice"]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(PLAYER_SEARCH_DEBOUNCE_MS + 1);
    });
    expect(searched()).toEqual(["alice"]);
  });

  it("R654 the players tab pages: a full page offers the next, which asks the server for page 2, and Previous goes back", async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = new URL(String(input), window.location.origin);
      if (url.pathname === "/api/stats/players") {
        const page = Number(url.searchParams.get("page") ?? "1");
        const players =
          page === 1
            ? [
                { ...MOCK_PLAYERS.players[0], profileId: "profile-alpha", username: "AlphaPlayer" },
                { ...MOCK_PLAYERS.players[0], profileId: "profile-beta", username: "BetaPlayer" },
              ]
            : [{ ...MOCK_PLAYERS.players[0], profileId: "profile-gamma", username: "GammaPlayer" }];
        return Promise.resolve(new Response(JSON.stringify({ players, page, limit: 2 }), { status: 200 }));
      }
      return Promise.resolve(new Response(JSON.stringify(MOCK_CARD_STATS_PROVISIONAL), { status: 200 }));
    });
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();
    render(<StatsRoute />);

    const asked = (): { search: string | null; page: string | null }[] =>
      fetchMock.mock.calls
        .map(([input]) => new URL(String(input), window.location.origin))
        .filter((url) => url.pathname === "/api/stats/players")
        .map((url) => ({ search: url.searchParams.get("search"), page: url.searchParams.get("page") }));

    await user.click(await screen.findByTestId(statsTestid.tabPlayers, undefined, SLOW));
    await screen.findByText("BetaPlayer", undefined, SLOW);
    expect(screen.getByTestId(statsTestid.playersPager)).toHaveTextContent("Page 1");
    expect(screen.getByTestId(statsTestid.playersPrev)).toBeDisabled();
    expect(screen.getByTestId(statsTestid.playersNext)).toBeEnabled();

    await user.click(screen.getByTestId(statsTestid.playersNext));
    await screen.findByText("GammaPlayer", undefined, SLOW);
    expect(screen.queryByText("AlphaPlayer")).not.toBeInTheDocument();
    expect(asked().some((call) => call.page === "2")).toBe(true);
    expect(screen.getByTestId(statsTestid.playersPager)).toHaveTextContent("Page 2");
    expect(screen.getByTestId(statsTestid.playersNext)).toBeDisabled();

    await user.click(screen.getByTestId(statsTestid.playersPrev));
    await screen.findByText("AlphaPlayer", undefined, SLOW);
    expect(screen.getByTestId(statsTestid.playersPager)).toHaveTextContent("Page 1");

    // A new search starts again at page 1.
    await user.click(screen.getByTestId(statsTestid.playersNext));
    await screen.findByText("GammaPlayer", undefined, SLOW);
    fireEvent.change(screen.getByPlaceholderText("Search player by username…"), { target: { value: "alp" } });
    await waitFor(() => expect(asked()).toContainEqual({ search: "alp", page: null }));
    expect(asked()).not.toContainEqual({ search: "alp", page: "2" });
  });

  it("R654 site footer links to stats page", () => {
    render(<SiteFooter />);
    const footerLink = screen.getByTestId(siteFooterTestid.stats);
    expect(footerLink).toHaveAttribute("href", "/stats");
    expect(footerLink).toHaveTextContent("Stats");
  });

  it("R2325000 R661 shows no banner, no reading notes, no source and no live-game count", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    const page = await screen.findByTestId(statsTestid.screen, undefined, SLOW);
    await screen.findByTestId(statsTestid.summaryTiles, undefined, SLOW);

    // R2325000: no banner and no reading notes even while the source is provisional.
    expect(screen.queryByTestId("stats-provisional-banner")).toBeNull();
    expect(screen.queryByText("Provisional statistics")).toBeNull();
    expect(screen.queryByText("How to read these statistics")).toBeNull();

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

  it("R2427000 shows Drawn vs not drawn beside When drawn with signed points and both sides' games", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    // Drawn vs not drawn column header is shown by default, right after When drawn's
    const headers = screen.getAllByRole("columnheader").map((th) => th.textContent ?? "");
    const drawnAt = headers.findIndex((text) => /When drawn/i.test(text));
    expect(headers[drawnAt + 1]).toMatch(/Drawn vs not drawn/i);
    expect(screen.queryByRole("columnheader", { name: /Played vs held/i })).not.toBeInTheDocument();

    // Spark Pup (eligible, delta = +0.25): renders +25% and (40 / 10)
    expect(screen.getByText("+25%")).toBeInTheDocument();
    expect(screen.getByText("(40 / 10)")).toBeInTheDocument();

    // Aqua Warden (eligible, delta = -0.15): renders -15% and (25 / 5)
    expect(screen.getByText("-15%")).toBeInTheDocument();
    expect(screen.getByText("(25 / 5)")).toBeInTheDocument();

    // Lava Hound (below sample floor: 10 games < 20): its When drawn renders "—"
    const houndRow = screen.getByText("Lava Hound").closest("tr")!;
    expect(within(houndRow).getAllByText("—")).toHaveLength(3);

    // Toggle Drawn vs not drawn off: the column header leaves
    await user.click(screen.getByLabelText(/Drawn vs not drawn/i));
    expect(screen.queryByRole("columnheader", { name: /Drawn vs not drawn/i })).not.toBeInTheDocument();

    // Toggle it back on: the column header returns
    await user.click(screen.getByLabelText(/Drawn vs not drawn/i));
    expect(screen.getByRole("columnheader", { name: /Drawn vs not drawn/i })).toBeInTheDocument();
  });

  it("R654 renders not enough data for a Drawn vs not drawn on a card below the floor", async () => {
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    // Lava Hound is below the floor (10 games < 20): no delta and no cohort sizes.
    const houndRow = screen.getByText("Lava Hound").closest("tr")!;
    expect(within(houndRow).getByText("not enough data")).toBeInTheDocument();
    expect(within(houndRow).queryByText("-10%")).not.toBeInTheDocument();
    expect(within(houndRow).queryByText("(8 / 2)")).not.toBeInTheDocument();
  });

  it("R2427000 sorts cards by Drawn vs not drawn descending and ascending", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    const deltaHeader = screen.getByRole("columnheader", { name: /Drawn vs not drawn/i });
    expect(deltaHeader).toHaveAttribute("aria-sort", "none");

    // Click to sort descending: Spark Pup (+25%) first, then Aqua Warden (-15%), then Lava Hound (< minSample)
    await user.click(deltaHeader);
    expect(deltaHeader).toHaveAttribute("aria-sort", "descending");

    let rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Spark Pup");
    expect(rows[1]).toHaveTextContent("Aqua Warden");
    expect(rows[2]).toHaveTextContent("Lava Hound");

    // Click again to toggle to ascending: Aqua Warden (-15%) first, then Spark Pup (+25%), then Lava Hound
    await user.click(deltaHeader);
    expect(deltaHeader).toHaveAttribute("aria-sort", "ascending");

    rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Aqua Warden");
    expect(rows[1]).toHaveTextContent("Spark Pup");
    expect(rows[2]).toHaveTextContent("Lava Hound");
  });

  it("R1147000 face rates + Normal/Radiant click-sorts", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    const radiantHeader = screen.getByRole("columnheader", { name: /Radiant/i });
    const normalHeader = screen.getByRole("columnheader", { name: /Normal/i });

    expect(radiantHeader).toBeInTheDocument();
    expect(normalHeader).toBeInTheDocument();

    // Spark Pup: 30 radiant (70%), 20 normal (40%)
    const pupRow = screen.getByText("Spark Pup").closest("tr")!;
    expect(within(pupRow).getByText("70%")).toBeInTheDocument();
    expect(within(pupRow).getByText("40%")).toBeInTheDocument();

    // Aqua Warden: 20 radiant (40%), 10 normal (<20 => —)
    const wardenRow = screen.getByText("Aqua Warden").closest("tr")!;
    expect(within(wardenRow).getByText("40%")).toBeInTheDocument();

    // Lava Hound (<20 games total => — for both)
    const houndRow = screen.getByText("Lava Hound").closest("tr")!;
    expect(within(houndRow).getAllByText("—")).toHaveLength(3);

    // Sort by Radiant descending: Spark Pup (70%) > Aqua Warden (40%) > Lava Hound (—)
    await user.click(radiantHeader);
    expect(radiantHeader).toHaveAttribute("aria-sort", "descending");

    let rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Spark Pup");
    expect(rows[1]).toHaveTextContent("Aqua Warden");
    expect(rows[2]).toHaveTextContent("Lava Hound");

    // Sort by Radiant ascending: Aqua Warden (40%) > Spark Pup (70%) > Lava Hound (—)
    await user.click(radiantHeader);
    expect(radiantHeader).toHaveAttribute("aria-sort", "ascending");

    rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Aqua Warden");
    expect(rows[1]).toHaveTextContent("Spark Pup");
    expect(rows[2]).toHaveTextContent("Lava Hound");

    // Sort by Normal descending: Spark Pup (40%) > Aqua Warden (—) / Lava Hound (—)
    await user.click(normalHeader);
    expect(normalHeader).toHaveAttribute("aria-sort", "descending");

    rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Spark Pup");
  });

  it("R654 keyword filter", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = String(input);
        if (url.includes("/api/stats/cards")) {
          return Promise.resolve(new Response(JSON.stringify(MOCK_KEYWORD_CARDS), { status: 200 }));
        }
        return Promise.resolve(new Response("{}", { status: 200 }));
      }),
    );

    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    // Filter by keyword: "Taunt"
    const keywordSelect = screen.getByLabelText(/Filter by keyword/i);
    await user.selectOptions(keywordSelect, "Taunt");

    let rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows).toHaveLength(2);
    expect(screen.getByText("Midrange Menace")).toBeInTheDocument();
    expect(screen.getByText("Lava Golem")).toBeInTheDocument();
    expect(screen.queryByText("Spark Pup")).not.toBeInTheDocument();
    expect(screen.queryByText("Prem Panther")).not.toBeInTheDocument();

    // Reset filter to All keywords
    await user.selectOptions(keywordSelect, "");
    rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows).toHaveLength(4);
  });

  it("R654 sort control mirrors headers", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    const sortSelect = screen.getByTestId(statsTestid.sortSelect);
    const sortDirBtn = screen.getByTestId(statsTestid.sortDir);

    expect(sortSelect).toHaveValue("winRate");
    expect(sortDirBtn).toHaveTextContent("↓");

    // Change sort select to "name"
    await user.selectOptions(sortSelect, "name");
    expect(sortSelect).toHaveValue("name");
    // defaultDirection for name is asc
    expect(sortDirBtn).toHaveTextContent("↑");
    const nameHeader = screen.getByRole("columnheader", { name: /Name/i });
    expect(nameHeader).toHaveAttribute("aria-sort", "ascending");

    // Toggle direction button
    await user.click(sortDirBtn);
    expect(sortDirBtn).toHaveTextContent("↓");
    expect(nameHeader).toHaveAttribute("aria-sort", "descending");

    // Clicking a table header updates the select control
    const gamesHeader = within(screen.getByTestId(statsTestid.cardsTable)).getByRole("columnheader", { name: /^Games/ });
    await user.click(gamesHeader);
    expect(sortSelect).toHaveValue("games");
    expect(gamesHeader).toHaveAttribute("aria-sort", "descending");
  });

  it("R654 grid view and toggle", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(statsTestid.cardsGrid)).not.toBeInTheDocument();

    // Column toggles are visible in table view
    expect(screen.getByText("Columns:")).toBeInTheDocument();

    const viewGridBtn = screen.getByTestId(statsTestid.viewGrid);
    const viewTableBtn = screen.getByTestId(statsTestid.viewTable);

    expect(viewTableBtn).toHaveAttribute("aria-pressed", "true");
    expect(viewGridBtn).toHaveAttribute("aria-pressed", "false");

    // Switch to Grid view
    await user.click(viewGridBtn);

    expect(viewGridBtn).toHaveAttribute("aria-pressed", "true");
    expect(viewTableBtn).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByTestId(statsTestid.cardsGrid)).toBeInTheDocument();
    expect(screen.queryByTestId(statsTestid.cardsTable)).not.toBeInTheDocument();

    // Column toggles are hidden in grid view
    expect(screen.queryByText("Columns:")).not.toBeInTheDocument();

    // Verify grid cards render figures
    const gridItems = screen.getByTestId(statsTestid.cardsGrid).querySelectorAll("li.stats-grid-item");
    expect(gridItems).toHaveLength(3);

    // Switch back to Table view
    await user.click(viewTableBtn);
    expect(screen.getByTestId(statsTestid.cardsTable)).toBeInTheDocument();
    expect(screen.queryByTestId(statsTestid.cardsGrid)).not.toBeInTheDocument();
    expect(screen.getByText("Columns:")).toBeInTheDocument();
  });

  it("shows Drawn vs not drawn and every other column by default", async () => {
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    expect(screen.getByLabelText("Drawn vs not drawn")).toBeChecked();
    expect(screen.getByLabelText("Games")).toBeChecked();
    expect(screen.getByLabelText("When drawn")).toBeChecked();
    expect(screen.getByLabelText("Radiant")).toBeChecked();
    expect(screen.getByLabelText("Normal")).toBeChecked();
    expect(screen.getByLabelText("Play rate")).toBeChecked();
    expect(screen.getByRole("columnheader", { name: /Drawn vs not drawn/i })).toBeInTheDocument();
  });
});

describe("R377 public statistics match type", () => {
  function cardsCalls(): URL[] {
    return vi
      .mocked(fetch)
      .mock.calls.map(([input]) => new URL(String(input), window.location.origin))
      .filter((url) => url.pathname === "/api/stats/cards");
  }

  it("R377 mode picker asks the server for each match type and keeps it in the URL", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    const select = screen.getByTestId(statsTestid.modeSelect) as HTMLSelectElement;
    expect(select).toHaveValue("");

    await user.selectOptions(select, "bo1");
    await waitFor(() => expect(cardsCalls().some((url) => url.searchParams.get("mode") === "bo1")).toBe(true));
    expect(window.location.search).toContain("mode=bo1");

    await user.selectOptions(select, "random");
    await waitFor(() =>
      expect(cardsCalls().some((url) => url.searchParams.get("mode") === "random")).toBe(true),
    );
    expect(window.location.search).toContain("mode=random");

    vi.mocked(fetch).mockClear();
    await user.selectOptions(select, "");
    await waitFor(() => expect(cardsCalls().length).toBeGreaterThan(0));
    expect(cardsCalls().every((url) => !url.searchParams.has("mode"))).toBe(true);
    expect(window.location.search).not.toContain("mode=");
  });

  it("R377 stats page starts with the mode the URL names", async () => {
    at("/stats?mode=bo3");
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByTestId(statsTestid.modeSelect)).toHaveValue("bo3");
    await waitFor(() => expect(cardsCalls().some((url) => url.searchParams.get("mode") === "bo3")).toBe(true));
  });

  it("R377 drill-down carries the selected mode", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    await user.selectOptions(screen.getByTestId(statsTestid.modeSelect), "bo1");
    await waitFor(() => expect(cardsCalls().some((url) => url.searchParams.get("mode") === "bo1")).toBe(true));

    await user.click(screen.getByText("Spark Pup"));
    await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW);
    await waitFor(() =>
      expect(
        vi
          .mocked(fetch)
          .mock.calls.map(([input]) => new URL(String(input), window.location.origin))
          .some(
            (url) =>
              url.pathname === "/api/stats/cards/core-001" && url.searchParams.get("mode") === "bo1",
          ),
      ).toBe(true),
    );
  });

  it("R377 a mode with no games shows not enough games instead of a percentage", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.getByText("60%")).toBeInTheDocument();

    await user.selectOptions(screen.getByTestId(statsTestid.modeSelect), "random");
    await waitFor(() => expect(screen.getAllByText("not enough games").length).toBeGreaterThan(0));
    expect(screen.queryByText("60%")).not.toBeInTheDocument();
    expect(screen.queryByText("50%")).not.toBeInTheDocument();
  });
});

describe("card statistics requests", () => {
  it("a filter changed while the table loads ends on the newer figures, never on the canceled request's failure", async () => {
    const held = holdFetch((url) => url.pathname === "/api/stats/cards");
    render(<StatsRoute />);

    await waitFor(() => expect(held).toHaveLength(1), SLOW);
    fireEvent.change(screen.getByTestId(statsTestid.modeSelect), { target: { value: "bo1" } });
    await waitFor(() => expect(held).toHaveLength(2), SLOW);
    expect(held[0]!.signal?.aborted).toBe(true);
    expect(held[1]!.url.searchParams.get("mode")).toBe("bo1");

    await act(() => new Promise((resolve) => setTimeout(resolve, 0)));
    expect(screen.getByText("Loading card statistics…")).toBeInTheDocument();
    expectSkeleton(screen.getByText("Loading card statistics…"), "table");
    expect(screen.queryByText("The card statistics didn't load.")).not.toBeInTheDocument();

    await act(async () => {
      held[1]!.answer(MOCK_KEYWORD_CARDS);
    });
    expect(await screen.findByText("Midrange Menace", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("The card statistics didn't load.")).not.toBeInTheDocument();
  });

  it("the table's Retry asks again, and a filter changed while the retry loads cancels it", async () => {
    const held = holdFetch((url) => url.pathname === "/api/stats/cards");
    render(<StatsRoute />);

    await waitFor(() => expect(held).toHaveLength(1), SLOW);
    await act(async () => {
      held[0]!.answer({}, 500);
    });
    expect(await screen.findByText("The card statistics didn't load.", undefined, SLOW)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(held).toHaveLength(2), SLOW);
    fireEvent.change(screen.getByTestId(statsTestid.modeSelect), { target: { value: "bo1" } });
    await waitFor(() => expect(held).toHaveLength(3), SLOW);
    expect(held[1]!.signal?.aborted).toBe(true);

    await act(() => new Promise((resolve) => setTimeout(resolve, 0)));
    await act(async () => {
      held[2]!.answer(MOCK_KEYWORD_CARDS);
    });
    expect(await screen.findByText("Midrange Menace", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("The card statistics didn't load.")).not.toBeInTheDocument();
  });

  it("a card's details opened while another's load end on that card's, never Details unavailable", async () => {
    const held = holdFetch((url) => url.pathname.startsWith("/api/stats/cards/"));
    render(<StatsRoute />);

    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    fireEvent.click(screen.getByText("Spark Pup"));
    await waitFor(() => expect(held).toHaveLength(1), SLOW);
    fireEvent.click(screen.getByText("Aqua Warden"));
    await waitFor(() => expect(held).toHaveLength(2), SLOW);
    expect(held[1]!.url.pathname).toBe("/api/stats/cards/core-003");
    expect(held[0]!.signal?.aborted).toBe(true);

    await act(() => new Promise((resolve) => setTimeout(resolve, 0)));
    const modal = screen.getByTestId(statsTestid.drillDownModal);
    expect(within(modal).getByText("Loading this card's details…")).toBeInTheDocument();
    expectSkeleton(within(modal).getByText("Loading this card's details…"), "rows");
    expect(within(modal).queryByText("Details unavailable.")).not.toBeInTheDocument();

    await act(async () => {
      held[1]!.answer({
        ...MOCK_DRILL_DOWN,
        card: { id: "core-003", name: "Aqua Warden", cost: 2, rarity: "Common" },
        byTurn: [{ turn: 7, games: 12, winRate: 0.5 }],
      });
    });
    expect(await within(modal).findByText("Turn 7:", undefined, SLOW)).toBeInTheDocument();
    expect(within(modal).queryByText("Details unavailable.")).not.toBeInTheDocument();
  });

  it("a card's details that fail to load offer Retry, which asks again", async () => {
    let asked = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = new URL(String(input), window.location.origin);
        if (url.pathname === "/api/stats/cards/core-001") {
          asked += 1;
          const body = asked === 1 ? {} : MOCK_DRILL_DOWN;
          return Promise.resolve(new Response(JSON.stringify(body), { status: asked === 1 ? 500 : 200 }));
        }
        return Promise.resolve(new Response(JSON.stringify(MOCK_CARD_STATS_PROVISIONAL), { status: 200 }));
      }),
    );
    render(<StatsRoute />);

    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    fireEvent.click(screen.getByText("Spark Pup"));
    const modal = await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW);
    expect(await within(modal).findByText("Details unavailable.", undefined, SLOW)).toBeInTheDocument();

    fireEvent.click(within(modal).getByText("Retry"));
    expect(await within(modal).findByText("Win rate by patch", undefined, SLOW)).toBeInTheDocument();
    expect(asked).toBe(2);
  });

});

describe("the card drill-down dialog", () => {
  it("is a modal dialog assistive tech can find, named by its title", async () => {
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    fireEvent.click(screen.getByText("Spark Pup"));

    const dialog = await screen.findByRole("dialog", undefined, SLOW);
    expect(dialog).toBe(screen.getByTestId(statsTestid.drillDownModal));
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAccessibleName();
    expect(dialog.closest('[aria-hidden="true"]')).toBeNull();
  });

  it("takes the focus to Close when it opens, and Escape closes it and gives the focus back to the grid card that opened it", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    await user.click(screen.getByTestId(statsTestid.viewGrid));
    const card = within(screen.getByTestId(statsTestid.cardsGrid)).getByRole("button", { name: "Spark Pup" });

    // A click that leaves the focus where it was, as a click on a button does in Safari.
    fireEvent.click(card);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByTestId(statsTestid.drillDownClose)).toHaveFocus();

    press("Escape");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(card).toHaveFocus();
  });

  it("closes on a click on the scrim, never on one inside it, and on Close, each time giving the focus back to the table row", async () => {
    const { container } = render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    const row = screen.getByText("Spark Pup").closest("tr");
    if (row === null) throw new Error("no row");

    fireEvent.keyDown(row, { key: "Enter" });
    fireEvent.click(screen.getByRole("dialog"));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    const scrim = container.querySelector(".stats-modal-scrim");
    if (scrim === null) throw new Error("no scrim");
    fireEvent.click(scrim);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(row).toHaveFocus();

    fireEvent.click(row);
    expect(screen.getByTestId(statsTestid.drillDownClose)).toHaveFocus();
    fireEvent.click(screen.getByTestId(statsTestid.drillDownClose));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(row).toHaveFocus();
  });

  it("keeps Tab and Shift+Tab between Close, the face's controls and Retry, and brings them back inside from the page behind", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = new URL(String(input), window.location.origin);
        if (url.pathname === "/api/stats/cards/core-001") {
          return Promise.resolve(new Response("{}", { status: 500 }));
        }
        return Promise.resolve(new Response(JSON.stringify(MOCK_CARD_STATS_PROVISIONAL), { status: 200 }));
      }),
    );
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    fireEvent.click(screen.getByText("Spark Pup"));
    const dialog = screen.getByRole("dialog");
    const retry = await within(dialog).findByRole("button", { name: "Retry" }, SLOW);
    const close = screen.getByTestId(statsTestid.drillDownClose);
    const radiant = screen.getByTestId(statsTestid.drillDownRadiantToggle);
    const almanac = screen.getByTestId(statsTestid.drillDownAlmanac);
    expect(close).toHaveFocus();

    for (const next of [radiant, almanac, retry, close]) {
      press("Tab");
      expect(next).toHaveFocus();
    }
    for (const previous of [retry, almanac, radiant, close]) {
      press("Tab", { shiftKey: true });
      expect(previous).toHaveFocus();
    }

    const behind = screen.getByTestId(statsTestid.searchInput);
    behind.focus();
    press("Tab");
    expect(close).toHaveFocus();
    behind.focus();
    press("Tab", { shiftKey: true });
    expect(retry).toHaveFocus();
  });

  it("R2127000 Back closes the breakdown, gives the focus back to its row and keeps /stats with its tab and filters, a filter changed while it was open among them; with none open Back leaves", async () => {
    at("/decks");
    window.history.pushState(null, "", "/stats?set=core");
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    const row = screen.getByText("Spark Pup").closest("tr");
    if (row === null) throw new Error("no row");

    fireEvent.click(row);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(new URLSearchParams(window.location.search).get("card")).toBe("core-001");
    // A filter changed under the open breakdown replaces the address, over the entry Back is held by.
    fireEvent.change(screen.getByTestId(statsTestid.searchInput), { target: { value: "Spark" } });
    expect(new URLSearchParams(window.location.search).get("search")).toBe("Spark");

    await pressBack();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(row).toHaveFocus();
    expect(window.location.pathname).toBe(paths.stats);
    const params = new URLSearchParams(window.location.search);
    expect(params.get("set")).toBe("core");
    expect(params.get("search")).toBe("Spark");
    expect(params.has("card")).toBe(false);
    expect(screen.getByTestId(statsTestid.cardsTable)).toBeInTheDocument();
    expect(screen.getByTestId(statsTestid.searchInput)).toHaveValue("Spark");

    // With none open, Back leaves the page as it did.
    await pressBack();
    expect(window.location.pathname).toBe("/decks");
  });

  it("R2127000 with the breakdown closed by its Close, one Back still leaves /stats", async () => {
    at("/decks");
    window.history.pushState(null, "", "/stats");
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);

    fireEvent.click(screen.getByText("Spark Pup"));
    fireEvent.click(screen.getByTestId(statsTestid.drillDownClose));
    expect(screen.queryByRole("dialog")).toBeNull();

    await pressBack();
    expect(window.location.pathname).toBe("/decks");
  });
});

describe("the drill-down's card preview", () => {
  // jsdom lays nothing out, so the stylesheet is read: the face is `height: 100%` (cards.css), and a
  // preview stretched to the details column beside it drew the card tall and thin.
  it("keeps the face at 5:7 however long the details column runs", () => {
    const css = readFileSync(resolve(HERE, "stats.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    const body = /\.stats-drilldown-body\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(body).toMatch(/align-items:\s*flex-start/);
    const preview = /\.stats-drilldown-card-preview\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(preview).toMatch(/aspect-ratio:\s*5\s*\/\s*7/);
  });

  it("shows the Radiant face beside the normal one on Show Radiant, and only the normal one again on a second press", async () => {
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    fireEvent.click(screen.getByText("Spark Pup"));
    const dialog = screen.getByRole("dialog");
    const toggle = screen.getByTestId(statsTestid.drillDownRadiantToggle);
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    expect(dialog.querySelectorAll(".cf")).toHaveLength(1);
    expect(dialog.querySelector('[data-radiant-face="true"]')).toBeNull();

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "true");
    expect(dialog.querySelectorAll(".cf")).toHaveLength(2);
    const radiant = screen.getByTestId(statsTestid.drillDownRadiantFace);
    expect(radiant.querySelector('.cf[data-radiant-face="true"]')).not.toBeNull();
    expect(within(radiant).getByText("Radiant")).toBeInTheDocument();
    expect(within(dialog).getByText("Normal")).toBeInTheDocument();
    expect(dialog).toHaveAttribute("data-radiant", "true");

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    expect(screen.queryByTestId(statsTestid.drillDownRadiantFace)).toBeNull();
    expect(dialog.querySelectorAll(".cf")).toHaveLength(1);
  });

  it("links the card to its detail in the Almanac, and follows the link in place", async () => {
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    fireEvent.click(screen.getByText("Spark Pup"));
    const link = screen.getByTestId(statsTestid.drillDownAlmanac);
    expect(link).toHaveAttribute("href", "/almanac?card=core-001");
    expect(link).toHaveAttribute("href", paths.almanacCard("core-001"));

    fireEvent.click(link);
    expect(`${window.location.pathname}${window.location.search}`).toBe("/almanac?card=core-001");
  });

  it("shows the tokens a card's text names, each a link to its detail in the Almanac, and no Tokens section for a card that names none", async () => {
    holdFetch((url) => url.pathname.startsWith("/api/stats/cards/"));
    at("/stats?card=core-015");
    render(<StatsRoute />);
    const tokens = await screen.findByTestId(statsTestid.drillDownTokens, undefined, SLOW);
    const rush = within(tokens).getByRole("link", { name: "View Rush Token in the Almanac" });
    expect(rush).toHaveAttribute("href", paths.almanacCard("core-t-rush"));
    expect(within(tokens).getAllByRole("link")).toHaveLength(1);
    expect(rush.querySelector(".cf")).not.toBeNull();

    // Two tokens named, and a card that names only itself shows none.
    cleanup();
    at("/stats?card=core-098");
    render(<StatsRoute />);
    const two = await screen.findByTestId(statsTestid.drillDownTokens, undefined, SLOW);
    expect(within(two).getAllByRole("link").length).toBeGreaterThanOrEqual(2);

    cleanup();
    at("/stats?card=core-003");
    render(<StatsRoute />);
    await screen.findByRole("dialog", undefined, SLOW);
    expect(screen.queryByTestId(statsTestid.drillDownTokens)).toBeNull();
  });
});

describe("R1966000 the Sets tab", () => {
  /** A card row: `wins` of `games`, in `set`. */
  function row(id: string, name: string, set: string, wins: number, games: number) {
    return {
      ...MOCK_CARD_STATS_PROVISIONAL.cards[0]!,
      id,
      name,
      set,
      games,
      winRate: wins / games,
      hasEnoughGames: games >= 20,
    };
  }

  const NOW = {
    ...MOCK_CARD_STATS_PROVISIONAL,
    patch: "0.2.0",
    previousPatch: "0.1.0",
    cards: [
      row("core-001", "Spark Pup", "Core", 700, 1200), // 58.3%: 5.8 margins over 50%
      row("core-002", "Lava Hound", "Core", 450, 1000), // 45%: 3.2 under
      row("classic-001", "Palantir", "Classic", 520, 1000),
      row("classic-002", "Counterspell", "Classic", 480, 1000),
    ],
  };
  const BEFORE = {
    ...NOW,
    patch: "0.1.0",
    previousPatch: null,
    cards: [row("core-001", "Spark Pup", "Core", 500, 1000), row("classic-001", "Palantir", "Classic", 550, 1000)],
  };

  /** Answers the card statistics of patch 0.1.0 with `BEFORE`, any other with `NOW` (or `answer`'s), and a card's details with Spark Pup's. */
  function serveSets(answer: () => Response = () => new Response(JSON.stringify(NOW), { status: 200 })): void {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = new URL(String(input), window.location.origin);
        if (url.pathname.startsWith("/api/stats/cards/")) {
          return Promise.resolve(new Response(JSON.stringify(MOCK_DRILL_DOWN), { status: 200 }));
        }
        if (url.pathname !== "/api/stats/cards") return Promise.resolve(new Response("{}", { status: 200 }));
        if (url.searchParams.get("patch") === "0.1.0") {
          return Promise.resolve(new Response(JSON.stringify(BEFORE), { status: 200 }));
        }
        return Promise.resolve(answer());
      }),
    );
  }

  it("R1966000 ranks each set by its pooled win rate, with its mean, median, change since the previous patch, best and worst card", async () => {
    serveSets();
    const user = userEvent.setup();
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);

    await user.click(screen.getByTestId(statsTestid.tabSets));
    const table = await screen.findByTestId(statsTestid.setsTable, undefined, SLOW);
    expect(window.location.search).toBe("?tab=sets");
    const rows = within(table).getAllByRole("row").slice(1);
    expect(rows.map((r) => r.getAttribute("data-set"))).toEqual(["Core", "Classic"]);
    const [core, classic] = [rows[0]!, rows[1]!];
    // Core pools 1,150 wins of 2,200 card-games (52.3%); its two cards' mean and median are 51.7%.
    expect(core).toHaveTextContent("2,200");
    expect(core).toHaveTextContent("52.3%");
    expect(core).toHaveTextContent("51.7%");
    // Since 0.1.0: Core from 50% to 52.3%, Classic from 55% to 50%.
    expect(within(table).getByRole("columnheader", { name: "Since 0.1.0" })).toBeInTheDocument();
    expect(core).toHaveTextContent("+2.3%");
    expect(classic).toHaveTextContent("-5.0%");
    expect(within(core).getByRole("button", { name: "Spark Pup 58.3%" })).toBeInTheDocument();
    expect(within(core).getByRole("button", { name: "Lava Hound 45.0%" })).toBeInTheDocument();
    expect(within(classic).getByRole("button", { name: "Palantir 52.0%" })).toBeInTheDocument();

    const best = screen.getByTestId(statsTestid.setsLeaders);
    expect(best).toHaveTextContent("Highest win rateCore");
    expect(best).toHaveTextContent("Highest mean cardCore");
    expect(best).toHaveTextContent("Highest median cardCore");

    // Spark Pup is 5.8 margins over 50% and Lava Hound 3.2 under; the Classic cards are nearer.
    const far = screen.getByTestId(statsTestid.setsOutliers);
    const strongest = within(far).getByRole("region", { name: "Strongest cards" });
    const weakest = within(far).getByRole("region", { name: "Weakest cards" });
    expect(within(strongest).getAllByRole("listitem").map((li) => li.getAttribute("data-card-id"))).toEqual(["core-001"]);
    expect(strongest).toHaveTextContent("Core · 58.3% over 1,200 games · +5.8 margins");
    expect(within(weakest).getAllByRole("listitem").map((li) => li.getAttribute("data-card-id"))).toEqual(["core-002"]);

    // A card opens its details, as a row of the Cards tab does.
    await user.click(within(strongest).getByRole("button", { name: "Spark Pup" }));
    expect(await screen.findByTestId(statsTestid.drillDownModal, undefined, SLOW)).toBeInTheDocument();
  });

  it("#2115 a table skeleton stands in for the sets while the read is pending", async () => {
    serveSets();
    at("/stats?tab=sets");
    render(<StatsRoute />);

    // Right after render the read has not answered.
    expectSkeleton(screen.getByText("Loading set statistics…"), "table");

    expect(await screen.findByTestId(statsTestid.setsTable, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("Loading set statistics…")).toBeNull();
  });

  it("R1966000 reads every set's cards whatever the Cards tab filters, on the patch and match type it shows", async () => {
    serveSets();
    at("/stats?set=core&rarity=Rare&cost=2&mode=bo1");
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    fireEvent.click(screen.getByTestId(statsTestid.tabSets));
    await screen.findByTestId(statsTestid.setsTable, undefined, SLOW);
    const asked = requested("/api/stats/cards").filter((url) => !url.searchParams.has("set"));
    expect(asked.map((url) => [url.searchParams.get("patch"), url.searchParams.get("mode")])).toEqual([
      [null, "bo1"],
      ["0.1.0", "bo1"],
    ]);
    expect(asked.every((url) => !url.searchParams.has("rarity") && !url.searchParams.has("cost"))).toBe(true);
  });

  it("R1966000 a read that fails shows Retry, which asks again", async () => {
    let up = false;
    serveSets(() =>
      up
        ? new Response(JSON.stringify(NOW), { status: 200 })
        : new Response(JSON.stringify({ error: { code: "unavailable", message: "down" } }), { status: 503 }),
    );
    at("/stats?tab=sets");
    render(<StatsRoute />);
    const retry = await screen.findByTestId(statsTestid.setsRetry, undefined, SLOW);
    expect(screen.getByText("The set statistics didn't load.")).toBeInTheDocument();
    up = true;
    fireEvent.click(retry);
    expect(await screen.findByTestId(statsTestid.setsTable, undefined, SLOW)).toBeInTheDocument();
  });

  it("R1966000 says so when the patch has no games, and when no card is far from 50%", async () => {
    serveSets(() => new Response(JSON.stringify({ ...NOW, previousPatch: null, cards: [] }), { status: 200 }));
    at("/stats?tab=sets");
    render(<StatsRoute />);
    const noGames = await screen.findByText("No games on patch 0.2.0 yet.", undefined, SLOW);
    expect(noGames).toBeInTheDocument();
    expectEmptyState(noGames);
    cleanup();

    serveSets(() => new Response(JSON.stringify({ ...NOW, previousPatch: null, cards: NOW.cards.slice(2) }), { status: 200 }));
    render(<StatsRoute />);
    const table = await screen.findByTestId(statsTestid.setsTable, undefined, SLOW);
    expect(within(table).getByRole("columnheader", { name: "Change" })).toBeInTheDocument();
    expect(within(screen.getByTestId(statsTestid.setsOutliers)).getAllByText("No card is 3 margins or more from 50% yet.")).toHaveLength(2);
  });
});

describe("R1973002 the patch bar", () => {
  /** Newest first: 0.7.0 (newest, short of games), then the older patches with their games. */
  const PATCHES = [
    { versions: ["0.7.0", "0.7.0b"], games: 300, liveGames: 0 },
    { versions: ["0.6.0"], games: 1500, liveGames: 0 },
    { versions: ["0.5.0"], games: 999, liveGames: 0 },
    { versions: ["0.4.0", "0.4.0b"], games: 2000, liveGames: 0 },
    { versions: ["0.3.0"], games: 1000, liveGames: 0 },
    { versions: ["0.2.0"], games: 5000, liveGames: 0 },
    { versions: ["0.1.0"], games: 4000, liveGames: 0 },
    { versions: ["0.0.9"], games: 3000, liveGames: 0 },
  ];

  /** Answers the card statistics at the version asked for (the newest, 0.7.0b, when none is), with the patch list. */
  function servePatches(): void {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = new URL(String(input), window.location.origin);
        if (url.pathname !== "/api/stats/cards") return Promise.resolve(new Response("{}", { status: 200 }));
        const patch = url.searchParams.get("patch") ?? "0.7.0b";
        const body = { ...MOCK_CARD_STATS_PROVISIONAL, patch, previousPatch: null, patches: PATCHES };
        return Promise.resolve(new Response(JSON.stringify(body), { status: 200 }));
      }),
    );
  }

  it("R1973002 offers the newest five patches with 1,000 games and the one shown, and steps to the patch before and after", async () => {
    servePatches();
    const user = userEvent.setup();
    render(<StatsRoute />);
    const select = await screen.findByTestId(statsTestid.patchSelect, undefined, SLOW);
    // The newest, shown, is short of games but offered first; 0.5.0 (999) and the sixth, 0.0.9, are not.
    expect(within(select).getAllByRole("option").map((option) => option.textContent)).toEqual([
      "0.7.0 · 300 games",
      "0.6.0 · 1,500 games",
      "0.4.0 · 2,000 games",
      "0.3.0 · 1,000 games",
      "0.2.0 · 5,000 games",
      "0.1.0 · 4,000 games",
    ]);
    expect(screen.getByTestId(statsTestid.summaryTotalGames)).toHaveTextContent("Games on patch 0.7.0");
    expect(screen.getByTestId(statsTestid.patchNext)).toBeDisabled();

    // Back one patch: 0.6.0, asked for at its version.
    const previous = screen.getByTestId(statsTestid.patchPrevious);
    expect(previous).toHaveTextContent("← Previous patch (0.6.0)");
    await user.click(previous);
    await waitFor(() => expect(window.location.search).toBe("?patch=0.6.0"), SLOW);
    expect(requested("/api/stats/cards").at(-1)?.searchParams.get("patch")).toBe("0.6.0");
    // Back again, to 0.5.0: not offered for its 999 games, but shown, so the picker holds it.
    await user.click(screen.getByTestId(statsTestid.patchPrevious));
    await waitFor(() => expect(window.location.search).toBe("?patch=0.5.0"), SLOW);
    await waitFor(() => expect(screen.getByTestId(statsTestid.patchSelect)).toHaveValue("0.5.0"), SLOW);

    // Forward: the next patch, then the newest, which is the plain address.
    const next = screen.getByTestId(statsTestid.patchNext);
    expect(next).toHaveTextContent("Next patch (0.6.0) →");
    await user.click(next);
    await waitFor(() => expect(window.location.search).toBe("?patch=0.6.0"), SLOW);
    await user.click(screen.getByTestId(statsTestid.patchNext));
    await waitFor(() => expect(window.location.search).toBe(""), SLOW);

    // The picker asks for a patch at its newest version.
    await user.selectOptions(screen.getByTestId(statsTestid.patchSelect), "0.4.0b");
    await waitFor(() => expect(window.location.search).toBe("?patch=0.4.0b"), SLOW);
    expect(screen.getByTestId(statsTestid.summaryTotalGames)).toHaveTextContent("Games on patch 0.4.0");
  });

  it("R1973002 a server with no patch list shows no patch bar", async () => {
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    expect(screen.queryByTestId(statsTestid.patchBar)).toBeNull();
  });
});

describe("R1973000 the Daily runs tab is gone", () => {
  it("R1973000 the page offers Cards, Sets and Players, asks nothing of /api/stats/ci, and ?tab=ci opens the Cards tab", async () => {
    at("/stats?tab=ci");
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["Cards", "Sets", "Players"]);
    expect(screen.queryByText("Daily runs")).toBeNull();
    expect(requested("/api/stats/ci")).toEqual([]);
  });
});

describe("R2446000 the Cards table's columns", () => {
  it("R2446000 offers no Cost, Rarity, Keywords or Your stats column, toggle or sort, signed in with local stats", async () => {
    window.localStorage.setItem(
      SESSION_STORAGE_KEY,
      JSON.stringify({ accessToken: "active-token", refreshToken: "refresh", expiresAt: 9999999999 }),
    );
    recordGame({ ...EMPTY_LOG, seen: ["core-001"], played: { "core-001": 5 } }, "win");

    render(<StatsRoute />);

    const table = await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent?.replace(/[↑↓]/g, "").trim())).toEqual([
      "Card",
      "Name",
      "Games",
      "Win rate",
      "When drawn",
      "Drawn vs not drawn",
      "Radiant",
      "Normal",
      "Play rate",
    ]);
    for (const label of ["Cost", "Rarity", "Keywords", "Your stats"]) {
      expect(screen.queryByLabelText(label)).toBeNull();
    }
    expect(screen.queryByText(/5×/)).toBeNull();
    expect(
      within(screen.getByTestId(statsTestid.sortSelect))
        .getAllByRole("option")
        .map((option) => option.getAttribute("value")),
    ).toEqual(["name", "games", "winRate", "drawnWinRate", "drawnDelta", "radiantWinRate", "normalWinRate", "playRate"]);
  });

  it("R2446000 an old address's sort by cost, rarity or keyword opens on the default sort, win rate descending", async () => {
    for (const gone of ["cost", "rarity", "keyword"]) {
      at(`/stats?sort=${gone}&dir=asc`);
      render(<StatsRoute />);

      const table = await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
      expect(screen.getByTestId(statsTestid.sortSelect)).toHaveValue("winRate");
      expect(screen.getByTestId(statsTestid.sortDir)).toHaveTextContent("↓");
      expect(within(table).getByRole("columnheader", { name: /Win rate/ })).toHaveAttribute("aria-sort", "descending");
      expect([...table.querySelectorAll("tbody tr")].map((row) => row.getAttribute("data-card-id"))).toEqual([
        "core-001",
        "core-003",
        "core-002",
      ]);
      await waitFor(() => expect(window.location.search).toBe(""), SLOW);
      cleanup();
    }
  });

  it("R2446000 the rarity, cost and keyword filters stay: the address sets them and the server is asked for rarity and cost", async () => {
    const user = userEvent.setup();
    at("/stats?rarity=Rare&cost=2&keyword=Taunt");
    render(<StatsRoute />);

    await screen.findByTestId(statsTestid.screen, undefined, SLOW);
    expect(screen.getByLabelText("Filter by rarity")).toHaveValue("Rare");
    expect(screen.getByLabelText("Filter by cost")).toHaveValue("2");
    expect(screen.getByLabelText("Filter by keyword")).toHaveValue("Taunt");
    await waitFor(
      () =>
        expect(requested("/api/stats/cards").map((url) => [url.searchParams.get("rarity"), url.searchParams.get("cost")])).toContainEqual([
          "Rare",
          "2",
        ]),
      SLOW,
    );

    await user.selectOptions(screen.getByLabelText("Filter by cost"), "3");
    await waitFor(() => expect(requested("/api/stats/cards").some((url) => url.searchParams.get("cost") === "3")).toBe(true), SLOW);
    const params = new URLSearchParams(window.location.search);
    expect(params.get("rarity")).toBe("Rare");
    expect(params.get("cost")).toBe("3");
    expect(params.get("keyword")).toBe("Taunt");
  });
});

describe("R2089000 the Created cards", () => {
  /**
   * Rush Token, made during 55 seats' games: 30 used a copy (60%, 8 of them Radiant at its first use, too
   * few for a rate) and 25 used none (52%), 8 points under.
   */
  const RUSH = {
    id: "core-t-rush",
    name: "Rush Token",
    cost: 1,
    rarity: "Token",
    set: "Core",
    games: 55,
    createdPlayedGames: 30,
    createdPlayedWinRate: 0.6,
    createdNotPlayedGames: 25,
    createdNotPlayedWinRate: 0.52,
    createdPlayedRadiantGames: 8,
    createdPlayedRadiantWinRate: 0.75,
    createdPlayedNormalGames: 22,
    createdPlayedNormalWinRate: 12 / 22,
    createdPlayedDelta: 0.08,
    hasEnoughGames: true,
    window: { from: "0.2.0", liveGames: 412, source: "provisional", counted: ["0.2.0"] },
  };

  /** Answers the card statistics with `body`, and anything else with nothing. */
  function serveCards(body: unknown): void {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = new URL(String(input), window.location.origin);
        const answer = url.pathname === "/api/stats/cards" ? body : {};
        return Promise.resolve(new Response(JSON.stringify(answer), { status: 200 }));
      }),
    );
  }

  it("R2089000 the Cards tab lists Created cards apart from the deck table", async () => {
    serveCards({ ...MOCK_CARD_STATS_PROVISIONAL, created: [RUSH] });
    render(<StatsRoute />);

    const panel = await screen.findByTestId(statsTestid.createdPanel, undefined, SLOW);
    expect(screen.getByRole("tabpanel")).toContainElement(panel);
    expect(within(panel).getByRole("heading", { name: "Created cards" })).toBeInTheDocument();
    expect(panel).toHaveTextContent("each needs 20 games");
    const table = within(panel).getByTestId(statsTestid.createdTable);
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Card",
      "Games",
      "Used",
      "Not used",
      "Used Radiant",
      "Used normal",
      "Used vs not used",
    ]);
    const row = within(table).getByText("Rush Token").closest("tr")!;
    expect(row).toHaveAttribute("data-card-id", "core-t-rush");
    expect([...row.querySelectorAll("td")].map((td) => td.textContent)).toEqual(["55", "60%", "52%", "—", "55%", "+8%"]);
    // The 8 Radiant seats are below R654's floor: no 75%.
    expect(within(row).queryByText("75%")).toBeNull();

    // The deck table has no row for it.
    const deck = screen.getByTestId(statsTestid.cardsTable);
    expect(within(deck).queryByText("Rush Token")).toBeNull();
    expect(deck.querySelector('[data-card-id="core-t-rush"]')).toBeNull();

    // The search box asks of the Created cards what it asks of the deck's.
    fireEvent.change(screen.getByTestId(statsTestid.searchInput), { target: { value: "rush" } });
    expect(screen.getByText("No cards match the selected filters.")).toBeInTheDocument();
    expect(within(screen.getByTestId(statsTestid.createdPanel)).getByText("Rush Token")).toBeInTheDocument();
    fireEvent.change(screen.getByTestId(statsTestid.searchInput), { target: { value: "spark" } });
    expect(screen.getByText("Spark Pup")).toBeInTheDocument();
    expect(screen.queryByTestId(statsTestid.createdPanel)).toBeNull();

    // The Sets tab counts the deck cards alone and shows no Created section.
    fireEvent.click(screen.getByTestId(statsTestid.tabSets));
    const sets = await screen.findByTestId(statsTestid.setsTable, undefined, SLOW);
    expect(sets).not.toHaveTextContent("Rush Token");
    expect(screen.queryByTestId(statsTestid.createdPanel)).toBeNull();
  });

  it("R2089000 shows no Created section when the response has none", async () => {
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    expect(screen.queryByTestId(statsTestid.createdPanel)).toBeNull();
    cleanup();

    serveCards({ ...MOCK_CARD_STATS_PROVISIONAL, created: [] });
    render(<StatsRoute />);
    await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW);
    expect(screen.queryByTestId(statsTestid.createdPanel)).toBeNull();
    expect(screen.queryByText("Created cards")).toBeNull();
  });
});

describe("R2370000 the Tokens toggle", () => {
  /** A Created card's figures: `played` seats used one at `playedRate`, `notPlayed` made one and used none. */
  function created(id: string, name: string, played: number, playedRate: number | null, notPlayed: number, notPlayedRate: number | null) {
    return {
      id,
      name,
      cost: 1,
      rarity: "Token",
      set: "Core",
      games: played + notPlayed,
      createdPlayedGames: played,
      createdPlayedWinRate: playedRate,
      createdNotPlayedGames: notPlayed,
      createdNotPlayedWinRate: notPlayedRate,
      createdPlayedRadiantGames: 0,
      createdPlayedRadiantWinRate: null,
      createdPlayedNormalGames: played,
      createdPlayedNormalWinRate: playedRate,
      createdPlayedDelta: played >= 20 && notPlayed >= 20 && playedRate !== null && notPlayedRate !== null ? playedRate - notPlayedRate : null,
      hasEnoughGames: played + notPlayed >= 20,
      window: { from: "0.2.0", liveGames: 412, source: "provisional", counted: ["0.2.0"] },
    };
  }

  /** 55 seats: 18 of 30 that used one won and 13 of 25 that used none, 31 of 55 in all (56%). */
  const RUSH = created("core-t-rush", "Rush Token", 30, 0.6, 25, 0.52);
  /** 40 seats, each used one, 28 won (70%). */
  const SHEEP = created("core-t-sheep", "Sheep Token", 40, 0.7, 0, null);
  /** 12 seats, too few for a rate however many won. */
  const GHOUL = created("core-t-ghoul", "Ghoul Token", 12, 0.9, 0, null);
  /** A Created copy of a deck card: no token. */
  const COPY = created("core-001", "Spark Pup", 30, 0.5, 0, null);

  /** Answers the card statistics with `body`, and anything else with nothing. */
  function serveCards(body: unknown): void {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = new URL(String(input), window.location.origin);
        const answer = url.pathname === "/api/stats/cards" ? body : {};
        return Promise.resolve(new Response(JSON.stringify(answer), { status: 200 }));
      }),
    );
  }

  /** The table's rows as their card ids, and each row's cells after its name. */
  function rowsOf(table: HTMLElement): { ids: (string | null)[]; cells: string[][] } {
    const rows = within(table).getAllByRole("row").slice(1);
    return {
      ids: rows.map((row) => row.getAttribute("data-card-id")),
      cells: rows.map((row) => [...row.querySelectorAll("td")].map((td) => td.textContent ?? "")),
    };
  }

  it("R2370000 starts off, and on lists the tokens alone, best first, each with its win rate", async () => {
    serveCards({ ...MOCK_CARD_STATS_PROVISIONAL, created: [RUSH, SHEEP, COPY, GHOUL] });
    render(<StatsRoute />);

    await screen.findByTestId(statsTestid.createdPanel, undefined, SLOW);
    const toggle = screen.getByTestId(statsTestid.tokensToggle);
    expect(toggle).not.toBeChecked();
    expect(screen.getByLabelText("Show tokens")).toBe(toggle);
    expect(screen.queryByTestId(statsTestid.tokensPanel)).toBeNull();
    expect(window.location.search).not.toContain("tokens=");
    const createdBefore = rowsOf(screen.getByTestId(statsTestid.createdTable));

    fireEvent.click(toggle);
    const panel = screen.getByTestId(statsTestid.tokensPanel);
    expect(screen.getByRole("tabpanel")).toContainElement(panel);
    expect(within(panel).getByRole("heading", { name: "Tokens" })).toBeInTheDocument();
    expect(panel).toHaveTextContent("Each needs 20 games");
    const table = within(panel).getByTestId(statsTestid.tokensTable);
    expect(within(table).getAllByRole("columnheader").map((th) => th.textContent)).toEqual([
      "Token",
      "Games",
      "Win rate",
      "Used",
      "Not used",
      "Used vs not used",
    ]);
    // Sheep (70%) over Rush (31 of 55, 56%); Ghoul's 12 seats are under the floor, so it shows no rate
    // and comes last; Spark Pup's copy is no token.
    expect(rowsOf(table)).toEqual({
      ids: ["core-t-sheep", "core-t-rush", "core-t-ghoul"],
      cells: [
        ["40", "70%", "70%", "—", "—"],
        ["55", "56%", "60%", "52%", "+8%"],
        ["12", "—", "—", "—", "—"],
      ],
    });
    expect(window.location.search).toContain("tokens=1");

    // The Created cards are as they were, the tokens among them.
    expect(rowsOf(screen.getByTestId(statsTestid.createdTable))).toEqual(createdBefore);
    expect(createdBefore.ids).toEqual(["core-t-rush", "core-t-sheep", "core-001", "core-t-ghoul"]);

    // The search box asks of the tokens what it asks of the other cards.
    fireEvent.change(screen.getByTestId(statsTestid.searchInput), { target: { value: "rush" } });
    expect(rowsOf(within(screen.getByTestId(statsTestid.tokensPanel)).getByTestId(statsTestid.tokensTable)).ids).toEqual([
      "core-t-rush",
    ]);

    fireEvent.click(screen.getByTestId(statsTestid.tokensToggle));
    expect(screen.queryByTestId(statsTestid.tokensPanel)).toBeNull();
    expect(window.location.search).not.toContain("tokens=");
  });

  it("R2370000 the address turns it on, and with no token to show it says so", async () => {
    at("/stats?tokens=1");
    serveCards({ ...MOCK_CARD_STATS_PROVISIONAL, created: [COPY] });
    render(<StatsRoute />);

    const panel = await screen.findByTestId(statsTestid.tokensPanel, undefined, SLOW);
    expect(screen.getByTestId(statsTestid.tokensToggle)).toBeChecked();
    expect(panel).toHaveTextContent("No tokens match the selected filters.");
    expect(within(panel).queryByRole("table")).toBeNull();
    expect(window.location.search).toContain("tokens=1");
  });
});

describe("R2125000 the Cards tab opens on its last table", () => {
  it("R2125000 the Cards tab draws its kept table at once and keeps it when the read behind it fails", async () => {
    rememberRead(null, cardStatsPath(), MOCK_CARD_STATS_CLEARED);
    const held = holdFetch((url) => url.pathname === "/api/stats/cards");
    render(<StatsRoute />);

    expect(screen.queryByText("Loading card statistics…")).not.toBeInTheDocument();
    expect(screen.getByTestId(statsTestid.cardsTable)).toBeInTheDocument();
    await waitFor(() => expect(held).toHaveLength(1), SLOW);

    await act(async () => {
      held[0]!.answer({}, 503);
    });
    await act(() => new Promise((resolve) => setTimeout(resolve, 0)));

    expect(screen.queryByText("Failed to load card statistics.")).not.toBeInTheDocument();
    expect(screen.queryByText("Loading card statistics…")).not.toBeInTheDocument();
    expect(screen.getByTestId(statsTestid.cardsTable)).toBeInTheDocument();
  });

  it("R2125000 a filter with no kept table waits behind its skeleton, a kept one draws at once", async () => {
    rememberRead(null, cardStatsPath(), MOCK_CARD_STATS_CLEARED);
    const held = holdFetch((url) => url.pathname === "/api/stats/cards");
    render(<StatsRoute />);
    await waitFor(() => expect(held).toHaveLength(1), SLOW);

    fireEvent.change(screen.getByTestId(statsTestid.modeSelect), { target: { value: "bo1" } });
    await waitFor(() => expect(held).toHaveLength(2), SLOW);

    expect(screen.getByText("Loading card statistics…")).toBeInTheDocument();
  });
});
