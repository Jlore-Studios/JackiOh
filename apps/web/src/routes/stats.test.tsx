// Tests for the public statistics page (SPEC §9.11, R654).

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SHIPPED_SETS } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { paths } from "../net/navigate.ts";
import { SESSION_STORAGE_KEY } from "../net/session.ts";
import { PLAYER_SEARCH_DEBOUNCE_MS } from "../stats/config.ts";
import { recordGame, resetPlayerStats } from "../stats/store.ts";
import { statsTestid } from "../stats/testids.ts";
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

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

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

  it("R377 reveals Played Δ column and renders signed points and cohort sample sizes", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    // Played Δ column header is shown by default
    expect(screen.getByRole("columnheader", { name: /Played Δ/i })).toBeInTheDocument();

    // Spark Pup (eligible, delta = +0.20): renders +20% and (30 / 10)
    expect(screen.getByText("+20%")).toBeInTheDocument();
    expect(screen.getByText("(30 / 10)")).toBeInTheDocument();

    // Aqua Warden (eligible, delta = -0.10): renders -10% and (20 / 5)
    expect(screen.getByText("-10%")).toBeInTheDocument();
    expect(screen.getByText("(20 / 5)")).toBeInTheDocument();

    // Lava Hound (below sample floor: 10 games < 20): its When drawn renders "—"
    const houndRow = screen.getByText("Lava Hound").closest("tr")!;
    expect(within(houndRow).getAllByText("—")).toHaveLength(3);

    // Toggle Played Δ off: the column header leaves
    await user.click(screen.getByLabelText(/Played Δ/i));
    expect(screen.queryByRole("columnheader", { name: /Played Δ/i })).not.toBeInTheDocument();

    // Toggle it back on: the column header returns
    await user.click(screen.getByLabelText(/Played Δ/i));
    expect(screen.getByRole("columnheader", { name: /Played Δ/i })).toBeInTheDocument();
  });

  it("R654 renders not enough data for a Played Δ the server withholds or a card below the floor", async () => {
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    // Lava Hound is below the floor (10 games < 20): no delta and no cohort sizes.
    const houndRow = screen.getByText("Lava Hound").closest("tr")!;
    expect(within(houndRow).getByText("not enough data")).toBeInTheDocument();
    expect(within(houndRow).queryByText("+27%")).not.toBeInTheDocument();
    expect(within(houndRow).queryByText("(5 / 3)")).not.toBeInTheDocument();

    // The explainer says what the delta's floor is.
    expect(
      screen.getByText(/Played Δ needs 20 games on each side, played and drawn but not played/),
    ).toBeInTheDocument();
  });

  it("R377 sorts cards by Played Δ descending and ascending", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    const deltaHeader = screen.getByRole("columnheader", { name: /Played Δ/i });
    expect(deltaHeader).toHaveAttribute("aria-sort", "none");

    // Click to sort descending: Spark Pup (+20%) first, then Aqua Warden (-10%), then Lava Hound (< minSample)
    await user.click(deltaHeader);
    expect(deltaHeader).toHaveAttribute("aria-sort", "descending");

    let rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Spark Pup");
    expect(rows[1]).toHaveTextContent("Aqua Warden");
    expect(rows[2]).toHaveTextContent("Lava Hound");

    // Click again to toggle to ascending: Aqua Warden (-10%) first, then Spark Pup (+20%), then Lava Hound
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

  it("R654 keyword filter+sort", async () => {
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

    // Keywords column is hidden by default; show it first
    await user.click(screen.getByLabelText("Keywords"));

    // Keyword column header is present
    const keywordHeader = screen.getByRole("columnheader", { name: /Keywords/i });
    expect(keywordHeader).toBeInTheDocument();

    // Verify keyword labels in table
    const table = screen.getByTestId(statsTestid.cardsTable);
    expect(within(table).getByText("Immutable, Taunt")).toBeInTheDocument();
    expect(within(table).getByText("Cleave, Rush")).toBeInTheDocument();
    expect(within(table).getByText("Taunt")).toBeInTheDocument();

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

    // Click Keywords header to sort ascending: Cleave, Rush -> Immutable, Taunt -> Taunt -> (none)
    await user.click(keywordHeader);
    expect(keywordHeader).toHaveAttribute("aria-sort", "ascending");

    rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Prem Panther");
    expect(rows[1]).toHaveTextContent("Midrange Menace");
    expect(rows[2]).toHaveTextContent("Lava Golem");
    expect(rows[3]).toHaveTextContent("Spark Pup");

    // Click Keywords header to toggle descending: Taunt -> Immutable, Taunt -> Cleave, Rush -> (none)
    await user.click(keywordHeader);
    expect(keywordHeader).toHaveAttribute("aria-sort", "descending");

    rows = screen.getByTestId(statsTestid.cardsTable).querySelectorAll("tbody tr");
    expect(rows[0]).toHaveTextContent("Lava Golem");
    expect(rows[1]).toHaveTextContent("Midrange Menace");
    expect(rows[2]).toHaveTextContent("Prem Panther");
    expect(rows[3]).toHaveTextContent("Spark Pup");
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

    // Clicking table header updates select control (Cost is hidden by default; show it first)
    await user.click(screen.getByLabelText("Cost"));
    const costHeader = screen.getByRole("columnheader", { name: /Cost/i });
    await user.click(costHeader);
    expect(sortSelect).toHaveValue("cost");
    expect(sortDirBtn).toHaveTextContent("↑");
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

  it("shows Played Δ and hides cost, rarity and keywords by default", async () => {
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();

    expect(screen.getByLabelText("Cost")).not.toBeChecked();
    expect(screen.getByLabelText("Rarity")).not.toBeChecked();
    expect(screen.getByLabelText("Keywords")).not.toBeChecked();
    expect(screen.queryByRole("columnheader", { name: "Cost" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Rarity" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Keywords" })).not.toBeInTheDocument();

    expect(screen.getByLabelText("Played Δ")).toBeChecked();
    expect(screen.getByLabelText("Games")).toBeChecked();
    expect(screen.getByLabelText("When drawn")).toBeChecked();
    expect(screen.getByLabelText("Radiant")).toBeChecked();
    expect(screen.getByLabelText("Normal")).toBeChecked();
    expect(screen.getByLabelText("Play rate")).toBeChecked();
    expect(screen.getByRole("columnheader", { name: /Played Δ/i })).toBeInTheDocument();
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

  it("R377 previous-patch fallback keeps the selected mode", async () => {
    const user = userEvent.setup();
    render(<StatsRoute />);

    expect(await screen.findByTestId(statsTestid.cardsTable, undefined, SLOW)).toBeInTheDocument();
    await user.selectOptions(screen.getByTestId(statsTestid.modeSelect), "bo1");
    await waitFor(() => expect(cardsCalls().some((url) => url.searchParams.get("mode") === "bo1")).toBe(true));

    await user.click(await screen.findByTestId(statsTestid.fallbackToggle, undefined, SLOW));
    await waitFor(() =>
      expect(
        cardsCalls().some(
          (url) => url.searchParams.get("patch") === "0.1.0" && url.searchParams.get("mode") === "bo1",
        ),
      ).toBe(true),
    );
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
  it("a filter changed while the table loads ends on the newer figures, never on the cancelled request's failure", async () => {
    const held = holdFetch((url) => url.pathname === "/api/stats/cards");
    render(<StatsRoute />);

    await waitFor(() => expect(held).toHaveLength(1), SLOW);
    fireEvent.change(screen.getByTestId(statsTestid.modeSelect), { target: { value: "bo1" } });
    await waitFor(() => expect(held).toHaveLength(2), SLOW);
    expect(held[0]!.signal?.aborted).toBe(true);
    expect(held[1]!.url.searchParams.get("mode")).toBe("bo1");

    await act(() => new Promise((resolve) => setTimeout(resolve, 0)));
    expect(screen.getByText("Loading card statistics…")).toBeInTheDocument();
    expect(screen.queryByText("Failed to load card statistics.")).not.toBeInTheDocument();

    await act(async () => {
      held[1]!.answer(MOCK_KEYWORD_CARDS);
    });
    expect(await screen.findByText("Midrange Menace", undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByText("Failed to load card statistics.")).not.toBeInTheDocument();
  });

  it("the table's Retry asks again, and a filter changed while the retry loads cancels it", async () => {
    const held = holdFetch((url) => url.pathname === "/api/stats/cards");
    render(<StatsRoute />);

    await waitFor(() => expect(held).toHaveLength(1), SLOW);
    await act(async () => {
      held[0]!.answer({}, 500);
    });
    expect(await screen.findByText("Failed to load card statistics.", undefined, SLOW)).toBeInTheDocument();

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
    expect(screen.queryByText("Failed to load card statistics.")).not.toBeInTheDocument();
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
    expect(within(modal).getByText("Loading drill-down data…")).toBeInTheDocument();
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

  it("View previous patch asks the server once", async () => {
    render(<StatsRoute />);

    fireEvent.click(await screen.findByTestId(statsTestid.fallbackToggle, undefined, SLOW));
    await waitFor(
      () => expect(screen.getByTestId(statsTestid.summaryTotalGames)).toHaveTextContent("Games on patch 0.1.0"),
      SLOW,
    );
    expect(requested("/api/stats/cards").filter((url) => url.searchParams.get("patch") === "0.1.0")).toHaveLength(1);
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
