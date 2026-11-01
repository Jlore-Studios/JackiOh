// `/match/<id>`: the board specs 05 and 06 drive, with the socket faked. A `view` frame with no `legal`
// leaves `waitForMyTurn` waiting forever, so the route says so out loud (SURFACE §11.3); legality is
// the engine's (BUILD M5-T2, CLAUDE.md rule 7).

import { afterEach, beforeEach, describe, expect, it, onTestFinished, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactElement } from "react";

import { registeredCatalog } from "@jackioh/engine";
import type { ActionBody, PlayerView } from "@jackioh/shared";

import { DISCONNECT_GRACE_SECONDS, MULLIGAN_CLOCK_MS, RANK_STREAK_FLAME_LENGTH as N, SERIES_MAX_GAMES, SERIES_WINS_NEEDED, TURN_CLOCK_MS } from "@jackioh/server-config";

import { MATCH_RANKS_READ_ATTEMPTS, MATCH_RANKS_RETRY_MS } from "../game/config.ts";

import { VOIDED_CLOSE_CODE, type SocketLike } from "../game/net.ts";
import { baseView, card, emptySide, opponentModifiers, pendingFor, yourModifiers } from "../test/fixtures.ts";
import { formatClock } from "../game/Clock.tsx";
import { TURN_CLOCK_FINAL_MS, TURN_CLOCK_LAST_MS } from "../game/clockConstants.ts";
import { leaveMatchTestid } from "../game/ConfirmLeaveMatch.tsx";
import { modifierListTestid } from "../game/ModifierList.tsx";
import { resultRevealMs } from "../game/Result.tsx";
import { streakWords } from "../rank/StreakFlame.tsx";
import MatchRoute, { connectionWords, matchTestid, promptHolderOf, withoutToken } from "./match.tsx";
import { navigate, paths, usePathname } from "../net/navigate.ts";
import { replayTestid } from "../replay/ReplayPlayer.tsx";
import { gifTestid } from "../replay/ShareGif.tsx";
import { foldedReplaySteps } from "../test/replay.ts";
import { navTestid } from "./nav.tsx";

vi.mock("../net/navigate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/navigate.ts")>();
  return { ...actual, navigate: vi.fn() };
});

// R768000: "Share the last turn" makes a GIF at once; the export itself is `gif/export.test.ts`'s.
vi.mock("../replay/gif/export.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../replay/gif/export.ts")>();
  return { ...actual, exportGif: vi.fn(() => new Promise(() => undefined)), canShareFile: vi.fn(() => false), shareGif: vi.fn() };
});

class FakeSocket implements SocketLike {
  readyState = 0;
  onopen: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  readonly sent: string[] = [];

  constructor(readonly url: string) {}

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = 3;
  }

  frames(): Record<string, unknown>[] {
    return this.sent.map((text) => JSON.parse(text) as Record<string, unknown>);
  }
}

let sockets: FakeSocket[] = [];

function socketFactory(url: string): SocketLike {
  const socket = new FakeSocket(url);
  sockets.push(socket);
  return socket;
}

function live(): FakeSocket {
  const last = sockets.at(-1);
  if (last === undefined) throw new Error("no socket was opened");
  return last;
}

/** Open the socket and push one view, the way the actor does on attach (§9.5). */
function attach(extra: Record<string, unknown> = {}): void {
  act(() => {
    live().readyState = 1;
    live().onopen?.({});
    live().onmessage?.({
      data: JSON.stringify({ type: "view", view: baseView({ viewer: "p1", active: "p1" }), ...extra }),
    });
  });
}

function pushView(view: PlayerView): void {
  act(() => {
    live().onmessage?.({ data: JSON.stringify({ type: "view", view, legal: [] }) });
  });
}

beforeEach(() => {
  sockets = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve({
        ok: true,
        status: 200,
        text: () => Promise.resolve(JSON.stringify({ version: "v1", defs: {} })),
      } as unknown as Response),
    ),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("the networked board", () => {
  it("renders the board from the view alone, and names the frame that is missing", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();

    // §10.8's view is enough to draw the board.
    expect(screen.getByTestId("hero-you")).toBeInTheDocument();
    expect(screen.getByTestId("hero-opponent")).toBeInTheDocument();
    expect(screen.getByTestId("zone-you-units-1")).toBeInTheDocument();
    expect(screen.getByTestId("zone-you-backrow-5")).toBeInTheDocument();

    // ...and not enough to click anything, which is the gap the notice names.
    expect(screen.getByTestId("missing-legal-frame")).toBeInTheDocument();
    expect(screen.getByTestId("end-turn")).toBeDisabled();
  });

  it("comes alive on a view that carries the list, and the click goes out as an `action`", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();

    const legal: ActionBody[] = [{ type: "endTurn" }];
    act(() => {
      live().onmessage?.({
        data: JSON.stringify({ type: "view", view: baseView({ viewer: "p1", active: "p1" }), legal }),
      });
    });

    expect(screen.queryByTestId("missing-legal-frame")).toBeNull();
    const endTurn = screen.getByTestId("end-turn");
    expect(endTurn).not.toBeDisabled();

    fireEvent.click(endTurn);
    const sent = live().frames().at(-1) as { type: string; action: Record<string, unknown> };
    expect(sent.type).toBe("action");
    expect(sent.action.type).toBe("endTurn");
    expect(typeof sent.action.nonce).toBe("string");
  });

  it("takes the same list riding on the `view` frame", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    expect(screen.queryByTestId("missing-legal-frame")).toBeNull();
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
  });

  /** #1378: a failed network never leaves the board naming def ids. */
  it("#1378 names the board's cards from the shipped catalog on the first view, asking for no catalog, while every request fails", () => {
    const requests = vi.fn((input: unknown) => Promise.reject(new TypeError(`Failed to fetch ${String(input)}`)));
    vi.stubGlobal("fetch", requests);

    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({
      view: baseView({
        viewer: "p1",
        active: "p1",
        you: emptySide("p1", { hand: [card({ instanceId: "h-1378", defId: "core-001" })] }),
      }),
      legal: [{ type: "endTurn" }],
    });

    expect(screen.getByTestId("hand-card-h-1378").querySelector(".card-name")?.textContent).toBe(
      registeredCatalog()["core-001"]?.name,
    );
    expect(requests.mock.calls.map(([url]) => String(url)).filter((url) => /\/api\/catalog$/u.test(url))).toEqual([]);
  });

  it("shows a holding panel until the first view lands", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    expect(screen.getByTestId("match-connecting")).toBeInTheDocument();
    expect(screen.queryByTestId("hero-you")).toBeNull();
  });

  it("relays the actor's refusal verbatim (§9.3)", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    act(() => {
      live().onmessage?.({
        data: JSON.stringify({
          type: "error",
          code: "illegal_action",
          message: "it is not your turn",
          nonce: "n1",
        }),
      });
    });
    expect(screen.getByTestId("action-error")).toHaveTextContent("it is not your turn");
  });

  it("publishes the seat on window.__jackioh, as e2e/support/types.ts expects", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    const handle = (window as unknown as { __jackioh?: { seat?: string; seed?: string } }).__jackioh;
    expect(handle?.seat).toBe("p1");
    // And no seed: the server never sends one, because (seed, log) is the library order (§9.3).
    expect(handle?.seed).toBe("");
  });
});

describe("the series banner on a series game (R259)", () => {
  /** A Conquest game's series, as `GET /api/matches/:id/series` answers it. */
  function seriesAnswer(over: boolean): Record<string, unknown> {
    const decks = [0, 1, 2].map((slot) => ({
      slot,
      name: `Deck ${String(slot + 1)}`,
      cards: [],
      won: over && slot === 0,
      games: slot === 0 ? 1 : 0,
    }));
    return {
      series: {
        id: "series-1",
        status: over ? "picking" : "playing",
        gameNo: over ? 2 : 1,
        winsNeeded: SERIES_WINS_NEEDED,
        maxGames: SERIES_MAX_GAMES,
        pickDeadline: null,
        now: 0,
        currentMatchId: over ? null : "m-1",
        ranked: true,
        you: { seat: "p1", wins: over ? 1 : 0, trioName: "Main trio", decks, pick: null, autoPick: false },
        opponent: { wins: 0, decks: decks.map(({ slot }) => ({ slot, won: false })), picked: false },
        games: [],
        result: null,
      },
    };
  }

  it("R259 shows the score, and once the game is over the way on to the next game", async () => {
    let gameOver = false;
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) =>
        Promise.resolve({
          ok: true,
          status: 200,
          text: () =>
            Promise.resolve(
              JSON.stringify(url.includes("/api/matches/m-1/series") ? seriesAnswer(gameOver) : { version: "v1", defs: {} }),
            ),
        } as unknown as Response),
      ),
    );
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();

    const banner = await screen.findByTestId("series-banner");
    expect(banner).toHaveTextContent("You 0 – 0 Opponent");
    expect(screen.queryByTestId("series-banner-continue")).toBeNull();

    gameOver = true;
    act(() => {
      live().onmessage?.({
        data: JSON.stringify({
          type: "view",
          view: baseView({ viewer: "p1", active: "p1", result: { winner: "p1", reason: "hero-death" } }),
        }),
      });
    });

    const next = await screen.findByTestId("series-banner-continue");
    expect(next).toHaveTextContent("Continue to game 2");
    expect(next).toHaveAttribute("href", "/series/series-1");
    expect(screen.getByTestId("series-banner")).toHaveTextContent("You 1 – 0 Opponent");
  });

  it("shows no banner on a game that is not in a series", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) =>
        Promise.resolve({
          ok: true,
          status: 200,
          text: () => Promise.resolve(JSON.stringify(url.includes("/series") ? { series: null } : { version: "v1", defs: {} })),
        } as unknown as Response),
      ),
    );
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    await act(async () => {
      await Promise.resolve();
    });

    expect(screen.queryByTestId("series-banner")).toBeNull();
  });
});

describe("withoutToken", () => {
  /** Printing the URL raw would put a live bearer token on screen and in the DOM. */
  it("strips the access token from the socket URL it displays", () => {
    const withToken =
      "wss://jackioh-server.onrender.com/ws/match?token=eyJhbGciOiJFUzI1NiJ9.SECRET.sig&matchId=abc";
    const shown = withoutToken(withToken);
    expect(shown).not.toContain("SECRET");
    expect(shown).not.toContain("token=");
    expect(shown).toContain("wss://jackioh-server.onrender.com/ws/match");
  });

  it("returns an unparseable value unchanged rather than throwing", () => {
    expect(withoutToken("not a url")).toBe("not a url");
  });
});

describe("R268 the mulligan clock on the match bar", () => {
  it("shows the one mulligan countdown on both sides, to the seat still choosing and to the one that is ready", () => {
    for (const mulligan of [
      { youReady: false, opponentReady: false },
      { youReady: true, opponentReady: false, kept: [] },
    ]) {
      render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
      const view = baseView({ viewer: "p1", active: "p1", turn: 0, phase: "mulligan", clockMs: 30_000, mulligan });
      act(() => {
        live().onopen?.({});
        live().onmessage?.({ data: JSON.stringify({ type: "view", view }) });
      });
      for (const id of ["clock-you", "clock-opponent"]) {
        const line = screen.getByTestId(id);
        expect(line, id).toHaveAttribute("data-kind", "mulligan");
        expect(line, id).toHaveAttribute("data-total-ms", String(MULLIGAN_CLOCK_MS));
        expect(line, id).toHaveTextContent("30s");
      }
      cleanup();
      sockets = [];
    }
  });

  it("counts the mulligan window down between clock frames, off the frame's own deadline", () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "setTimeout", "clearTimeout", "performance", "Date"] });
    try {
      render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
      const now = 1_000_000;
      const view = baseView({ viewer: "p2", active: "p1", turn: 0, phase: "mulligan", clockMs: MULLIGAN_CLOCK_MS, mulligan: { youReady: true, opponentReady: false, kept: [] } });
      act(() => {
        live().onopen?.({});
        live().onmessage?.({ data: JSON.stringify({ type: "view", view }) });
        live().onmessage?.({
          data: JSON.stringify({
            type: "clock",
            now,
            clocks: { turnDeadline: null, promptDeadline: now + MULLIGAN_CLOCK_MS, graceDeadline: { p1: null, p2: null }, ceilingAt: now + 3_600_000 },
          }),
        });
      });
      expect(screen.getByTestId("clock-you")).toHaveTextContent(`${String(MULLIGAN_CLOCK_MS / 1000)}s`);
      act(() => {
        vi.advanceTimersByTime(10_000);
      });
      for (const id of ["clock-you", "clock-opponent"]) {
        expect(screen.getByTestId(id), id).toHaveTextContent(`${String(MULLIGAN_CLOCK_MS / 1000 - 10)}s`);
      }
    } finally {
      vi.useRealTimers();
    }
  });

  it("once the window closes, the clock is the active player's turn clock again", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    act(() => {
      live().onopen?.({});
      live().onmessage?.({ data: JSON.stringify({ type: "view", view: baseView({ viewer: "p1", active: "p2", clockMs: 60_000 }) }) });
    });
    expect(screen.getByTestId("clock-you")).toHaveTextContent("—");
    expect(screen.getByTestId("clock-opponent")).toHaveTextContent("60s");
    expect(screen.getByTestId("clock-opponent")).not.toHaveAttribute("data-kind", "mulligan");
  });
});

describe("R439 the turn clock's last 30 seconds on the match bar", () => {
  function clockFrame(now: number, ms: number | null): string {
    return JSON.stringify({
      type: "clock",
      now,
      clocks: {
        turnDeadline: ms === null ? null : now + ms,
        promptDeadline: null,
        graceDeadline: { p1: null, p2: null },
        ceilingAt: now + 3_600_000,
      },
    });
  }

  function clockRoot(): Element | null {
    return document.querySelector(".turn-mechanism .clock");
  }

  it("R439 counts the turn clock down between the server's frames and turns urgent at 30 seconds", () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "setTimeout", "clearTimeout", "performance", "Date"] });
    try {
      render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
      const now = 1_000_000;
      act(() => {
        live().onopen?.({});
        live().onmessage?.({ data: JSON.stringify({ type: "view", view: baseView({ viewer: "p1", active: "p1", turn: 3 }) }) });
        live().onmessage?.({ data: clockFrame(now, TURN_CLOCK_FINAL_MS + 5_000) });
      });
      expect(screen.getByTestId("clock-you")).toHaveAttribute("data-kind", "turn");
      expect(clockRoot()).toHaveAttribute("data-clock-urgency", "none");

      act(() => {
        vi.advanceTimersByTime(5_000);
      });
      expect(clockRoot()).toHaveAttribute("data-clock-urgency", "final");
      expect(clockRoot()).toHaveAttribute("data-clock-side", "you");
      expect(screen.getByTestId("clock-you")).toHaveTextContent(formatClock(TURN_CLOCK_FINAL_MS));
      expect(screen.getByTestId("turn-clock-fuse")).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  it("R439 a view on the next turn never reads the last turn's deadline, even before its own frame lands", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    const now = 1_000_000;
    act(() => {
      live().onopen?.({});
      live().onmessage?.({ data: JSON.stringify({ type: "view", view: baseView({ viewer: "p1", active: "p2", turn: 3 }) }) });
      live().onmessage?.({ data: clockFrame(now, TURN_CLOCK_LAST_MS / 2) });
    });
    expect(clockRoot()).toHaveAttribute("data-clock-side", "opponent");
    expect(clockRoot()).toHaveAttribute("data-clock-urgency", "last10");

    // The viewer's turn begins: its view first, the server's fresh frame just after.
    act(() => {
      live().onmessage?.({
        data: JSON.stringify({ type: "view", view: baseView({ viewer: "p1", active: "p1", turn: 4, clockMs: TURN_CLOCK_MS }) }),
      });
    });
    expect(clockRoot(), "the old turn's 5 seconds are not the new turn's").toHaveAttribute("data-clock-urgency", "none");
    expect(screen.getByTestId("clock-you")).toHaveTextContent(formatClock(TURN_CLOCK_MS));

    act(() => {
      live().onmessage?.({ data: clockFrame(now + 1_000, TURN_CLOCK_MS) });
    });
    expect(clockRoot()).toHaveAttribute("data-clock-urgency", "none");
    expect(screen.getByTestId("clock-you")).toHaveAttribute("data-kind", "turn");
  });

  it("R439 a finished game runs no clock and warns nobody", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    act(() => {
      live().onopen?.({});
      live().onmessage?.({
        data: JSON.stringify({
          type: "view",
          view: baseView({ viewer: "p1", active: "p1", phase: "over", result: { winner: "p2", reason: "hero-death" } }),
        }),
      });
      live().onmessage?.({ data: clockFrame(1_000_000, TURN_CLOCK_LAST_MS) });
    });
    expect(clockRoot()).toHaveAttribute("data-clock-urgency", "none");
    expect(screen.queryByTestId("turn-clock-fuse")).toBeNull();
  });

  it("R79 names the prompt's holder from the view", () => {
    expect(promptHolderOf(null, "p1")).toBeNull();
    expect(promptHolderOf({ forYou: false, pendingFor: "p2" }, "p1")).toBe("p2");
    expect(
      promptHolderOf({ forYou: true, choiceId: "c", kind: "discover", options: [], min: 1, max: 1, prompt: "" }, "p2"),
    ).toBe("p2");
  });
});

/** `WS_CLOSE.forbidden` in crates/server/src/actor/ws_server.rs (R148): a refusal, never retried. */
const FORBIDDEN_CLOSE = 4403;

// des-4, des-10, str-1: what a player reads around the online board. The ids, the socket URL and
// the protocol detail stay out of a production build; the console keeps the detail.
describe("the match screen in a player's words", () => {
  it("says each connection state in words", () => {
    expect(connectionWords("open")).toBe("Connected");
    expect(connectionWords("connecting")).toBe("Connecting…");
    expect(connectionWords("reconnecting")).toBe("Reconnecting…");
    expect(connectionWords("refused")).toBe("This match isn't available");
    expect(connectionWords("closed")).toBe("Disconnected");
  });

  it("waits in the tavern panel, which says the server may be waking up", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    const panel = screen.getByTestId("match-connecting");
    expect(panel).toHaveClass("tavern");
    expect(panel).toHaveTextContent("Joining the match…");
    const status = screen.getByTestId("match-status");
    expect(status).toHaveTextContent("Connecting…");
    expect(status).toHaveAttribute("data-connection", "connecting");
  });

  it("says a refused match isn't available, and keeps the server's reason for the console", () => {
    const logged = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try {
      render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
      act(() => {
        live().onclose?.({ code: FORBIDDEN_CLOSE, reason: "not a participant in this match", wasClean: true });
      });
      expect(screen.getByTestId("match-refused")).toHaveTextContent("This match isn't available.");
      expect(screen.getByTestId("match-connecting").textContent).not.toMatch(/not a participant|socket/);
      expect(logged.mock.calls.flat().join(" ")).toMatch(/not a participant in this match/);
    } finally {
      logged.mockRestore();
    }
  });

  it("#1599 a match voided before the first view says it is gone and offers the lobby, not Joining", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    expect(screen.getByTestId("match-connecting")).toHaveTextContent("Joining the match…");

    act(() => {
      live().onclose?.({ code: VOIDED_CLOSE_CODE, reason: "voided", wasClean: true });
    });
    expect(screen.getByTestId("match-gone")).toHaveTextContent("This match is no longer available.");
    expect(screen.getByTestId("match-connecting")).not.toHaveTextContent("Joining the match…");
    expect(screen.queryByTestId("gate-loading")).toBeNull();
    expect(screen.queryByTestId("match-refused")).toBeNull();
    expect(screen.queryByTestId("match-reconnect")).toBeNull();
    expect(screen.getByRole("link", { name: "Back to the lobby" })).toHaveAttribute("href", "/play");
    // A voided match has no socket to reopen (R679).
    expect(sockets).toHaveLength(1);
  });

  it("names the connection on the board's bar in words", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    const status = screen.getByTestId("match-status");
    expect(status).toHaveTextContent("Connected");
    expect(status).toHaveAttribute("data-connection", "open");
  });

  it("says a read-only board in a player's words, with no build plan or repo path in it", () => {
    const logged = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try {
      render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
      attach();
      const notice = screen.getByTestId("missing-legal-frame");
      expect(notice.textContent).not.toMatch(/BUILD M|SPEC §|apps\/|legalActions/);
      expect(notice).toHaveTextContent("Reload the page to rejoin the match.");
      expect(logged.mock.calls.flat().join(" ")).toMatch(/legalActions/);
    } finally {
      logged.mockRestore();
    }
  });

  it("puts Your turn in the tab while it is the player's turn", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    expect(document.title).toBe("Your turn · JackiOh");
    act(() => {
      live().onmessage?.({
        data: JSON.stringify({ type: "view", view: baseView({ viewer: "p1", active: "p2" }), legal: [] }),
      });
    });
    expect(document.title).toBe("Match · JackiOh");
  });

  it("#1792 puts Your turn in the tab while the seat still owes its mulligan, and takes it off once it is ready", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    const p2: Partial<PlayerView> = {
      viewer: "p2",
      active: "p1",
      turn: 0,
      phase: "mulligan",
      you: emptySide("p2"),
      opponent: emptySide("p1", { hand: { count: 4 } }),
    };
    attach({
      view: baseView({
        ...p2,
        mulligan: { youReady: false, opponentReady: false },
        pending: pendingFor("mulligan", [], { min: 0, max: 0 }),
      }),
      legal: [],
    });
    expect(document.title).toBe("Your turn · JackiOh");
    pushView(
      baseView({
        ...p2,
        mulligan: { youReady: true, opponentReady: false, kept: [] },
        pending: { forYou: false, pendingFor: "p1" },
      }),
    );
    expect(document.title).toBe("Match · JackiOh");
  });

  it("#1792 keeps Your turn off Player 1's tab once its mulligan is in, though the view names it active", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({
      view: baseView({
        viewer: "p1",
        active: "p1",
        turn: 0,
        phase: "mulligan",
        mulligan: { youReady: true, opponentReady: false, kept: [] },
        pending: { forYou: false, pendingFor: "p2" },
      }),
      legal: [],
    });
    expect(document.title).toBe("Match · JackiOh");
  });

  it("#1792 flags the tab for a prompt the viewer holds on the opponent's turn, and not for the opponent's on its own", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({
      view: baseView({
        viewer: "p1",
        active: "p2",
        pending: pendingFor("discover", [{ key: "mode:core-043", label: "Flood", defId: "core-043" }]),
      }),
      legal: [],
    });
    expect(document.title).toBe("Your turn · JackiOh");
    pushView(baseView({ viewer: "p1", active: "p1", pending: { forYou: false, pendingFor: "p2" } }));
    expect(document.title).toBe("Match · JackiOh");
  });
});

describe("the match screen's ranks (R604, R612)", () => {
  /** Profile ids as the server sends them: keys, never shown (R1436). */
  const P1_ID = "c0ffee00-1234-4abc-9def-0123456789ab";
  const P2_ID = "deadbeef-5678-4cde-8f01-23456789abcd";
  const ranksBody = {
    ranked: true,
    seats: {
      p1: {
        profileId: P1_ID,
        username: "Max#3",
        rank: { tier: "normal", division: 3, pips: 1, pipsPerDivision: 3, floor: "rotten" },
        you: true,
      },
      p2: {
        profileId: P2_ID,
        username: "שרה",
        rank: { tier: "raisin", placementsPlayed: 2, placementGames: 5 },
        you: false,
      },
    },
  };

  function stubFetch(ranks: unknown): ReturnType<typeof vi.fn> {
    const fetchMock = vi.fn((url: string) =>
      Promise.resolve({
        ok: true,
        status: 200,
        text: () =>
          Promise.resolve(
            JSON.stringify(String(url).includes("/ranks") ? ranks : { version: "v1", defs: {} }),
          ),
      } as unknown as Response),
    );
    vi.stubGlobal("fetch", fetchMock);
    return fetchMock;
  }

  it("names both seats' ranks and whether the game moves them", async () => {
    stubFetch(ranksBody);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    const banner = await screen.findByTestId("match-ranks");
    expect(banner).toHaveTextContent("Ranked match");
    expect(banner).toHaveTextContent("Max#3 (you) Normal Grape III");
    expect(banner).toHaveTextContent("שרה Raisin");
  });

  it("shows the full line on desktop and a compact line for phones, both with both seats", async () => {
    stubFetch(ranksBody);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    const banner = await screen.findByTestId("match-ranks");
    const full = banner.querySelector(".match-ranks__full");
    expect(full).toHaveTextContent("Ranked match");
    expect(full).toHaveTextContent("Normal Grape III");
    const short = banner.querySelector(".match-ranks__short");
    expect(short).toHaveTextContent("Normal III");
    expect(short).toHaveTextContent("Raisin");
    // #1650: a small crest beside each seat's words on both lines, which carry no pips or placements words.
    for (const name of ["Normal Grape III · 1/3 pips", "Raisin · 2/5 placements"]) {
      const crests = within(banner).getAllByRole("img", { name, hidden: true });
      expect(crests).toHaveLength(2);
      for (const crest of crests) expect(crest).toHaveAttribute("data-size", "small");
    }
    expect(banner.textContent).not.toMatch(/pips|placements/);
    const names = [...(short?.querySelectorAll("bdi.username") ?? [])];
    expect(names.map((name) => name.getAttribute("title"))).toEqual(["Max#3", "שרה"]);
  });

  it("R1436 names both seats by username, isolated, and shows no profile id nor any slice of one", async () => {
    stubFetch(ranksBody);
    const { container } = render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    const banner = await screen.findByTestId("match-ranks");
    for (const line of [".match-ranks__full", ".match-ranks__short"]) {
      const names = [...(banner.querySelector(line)?.querySelectorAll("bdi.username") ?? [])];
      expect(names.map((name) => name.getAttribute("title"))).toEqual(["Max#3", "שרה"]);
    }
    const names = [...(banner.querySelector(".match-ranks__full")?.querySelectorAll("bdi.username") ?? [])];
    expect(names[1]?.querySelector(":scope > bdi")).toHaveAttribute("dir", "auto");
    const text = container.textContent;
    for (const id of [P1_ID, P2_ID]) {
      expect(text).not.toContain(id);
      expect(text).not.toContain(id.slice(0, 8));
    }
  });

  it("R1436 a seat without a profile id and username is not a ranks body: no banner", async () => {
    const fetchMock = stubFetch({
      ranked: true,
      // The body a server before usernames sent: both seats named by a tag alone.
      seats: {
        p1: { tag: "ABC123", rank: ranksBody.seats.p1.rank, you: true },
        p2: { tag: "DEF456", rank: ranksBody.seats.p2.rank, you: false },
      },
    });
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    await screen.findByTestId("hero-you");
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/ranks"), expect.anything());
    });
    await act(async () => {});
    expect(screen.queryByTestId("match-ranks")).toBeNull();
  });

  it("says an unranked match moves nothing, and stays silent when the read is not a ranks body", async () => {
    stubFetch({ ...ranksBody, ranked: false });
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    expect(await screen.findByTestId("match-ranks")).toHaveTextContent("Unranked match");

    cleanup();
    const fetchMock = stubFetch({ version: "v1", defs: {} });
    render(<MatchRoute matchId="m-2" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    await screen.findByTestId("hero-you");
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining("/ranks"), expect.anything());
    });
    await act(async () => {});
    expect(screen.queryByTestId("match-ranks")).toBeNull();
  });

  function withStreaks(p1: number, p2: number, ranked = true): unknown {
    return { ranked, seats: { p1: { ...ranksBody.seats.p1, streak: p1 }, p2: { ...ranksBody.seats.p2, streak: p2 } } };
  }

  it("R2101000 a ranked match's line shows each seat's flame and count at or past RANK_STREAK_FLAME_LENGTH, on both lines", async () => {
    stubFetch(withStreaks(N, N + 2));
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    const banner = await screen.findByTestId("match-ranks");
    for (const line of [".match-ranks__full", ".match-ranks__short"]) {
      const flames = [...(banner.querySelector(line)?.querySelectorAll(".streak-flame") ?? [])];
      expect(flames.map((flame) => flame.getAttribute("aria-label"))).toEqual([streakWords(N), streakWords(N + 2)]);
      expect(flames.map((flame) => flame.textContent)).toEqual([String(N), String(N + 2)]);
    }
    expect(banner).toHaveTextContent(`Max#3 (you) ${String(N)} Normal Grape III`);
  });

  it("R2101000 no seat shows a flame below the threshold or after a loss", async () => {
    stubFetch(withStreaks(N - 1, 0));
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    const banner = await screen.findByTestId("match-ranks");
    expect(banner).toHaveTextContent("Ranked match");
    expect(banner.querySelector(".streak-flame")).toBeNull();
  });

  it("R2101000 an unranked (casual) match's line shows no flame, whatever the streaks", async () => {
    stubFetch(withStreaks(N + 1, N + 1, false));
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });

    const banner = await screen.findByTestId("match-ranks");
    expect(banner).toHaveTextContent("Unranked match");
    expect(banner.querySelector(".streak-flame")).toBeNull();
  });
});

describe("R2388000 a ranked match's result shows the rank it moved each seat to", () => {
  const P1_ID = "c0ffee00-1234-4abc-9def-0123456789ab";
  const P2_ID = "deadbeef-5678-4cde-8f01-23456789abcd";
  const p1 = { profileId: P1_ID, username: "Max#3", streak: 0, you: true };
  const p2 = { profileId: P2_ID, username: "Rowan", streak: 0, you: false };
  const finished = baseView({ viewer: "p1", active: "p1", result: { winner: "p1", reason: "hero-death" } });

  async function advance(ms: number): Promise<void> {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ms);
    });
  }

  /** Every `/ranks` URL answers the next body in turn (the last one repeats) and counts the read. */
  function stubReads(bodies: unknown[]): () => number {
    let reads = 0;
    const fetchMock = vi.fn((url: string) => {
      if (String(url).includes("/ranks")) {
        const body = bodies[Math.min(reads, bodies.length - 1)];
        reads += 1;
        return Promise.resolve({
          ok: true,
          status: 200,
          text: () => Promise.resolve(JSON.stringify(body)),
        } as unknown as Response);
      }
      return Promise.resolve({
        ok: true,
        status: 200,
        text: () => Promise.resolve(JSON.stringify({ version: "v1", defs: {} })),
      } as unknown as Response);
    });
    vi.stubGlobal("fetch", fetchMock);
    return () => reads;
  }

  const beforeP1 = { tier: "normal", division: 1, pips: 2, pipsPerDivision: 3, floor: "normal" };
  const beforeP2 = { tier: "raisin", placementsPlayed: 2, placementGames: 5 };
  const before = {
    ranked: true,
    seats: { p1: { ...p1, rank: beforeP1 }, p2: { ...p2, rank: beforeP2 } },
  };
  const afterP1 = { tier: "large", division: 3, pips: 0, pipsPerDivision: 3, floor: "normal" };
  const afterP2 = { tier: "raisin", placementsPlayed: 3, placementGames: 5 };
  const moved = {
    ranked: true,
    seats: {
      p1: { ...p1, rank: afterP1, rankBefore: beforeP1, rankAfter: afterP1 },
      p2: { ...p2, rank: afterP2, rankBefore: beforeP2, rankAfter: afterP2 },
    },
  };

  it("R2388000 the result panel shows each seat's move once the record lands, and the bar the rank after", async () => {
    vi.useFakeTimers();
    onTestFinished(() => {
      vi.useRealTimers();
    });
    const reads = stubReads([before, before, moved]);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    await advance(0);
    expect(screen.getByTestId("match-ranks")).toHaveTextContent("Max#3 (you) Normal Grape I");

    pushView(finished);
    await advance(0);
    expect(screen.queryByTestId("match-rank-move")).toBeNull();

    await advance(MATCH_RANKS_RETRY_MS);
    const moves = screen.getAllByTestId("match-rank-move");
    expect(moves.map((move) => move.getAttribute("data-seat"))).toEqual(["p1", "p2"]);
    expect(moves[0]).toHaveTextContent("Max#3 (you): Normal Grape I · 2/3 pips → Large Grape III · 0/3 pips");
    expect(moves[1]).toHaveTextContent("Rowan: Raisin · 2/5 placements → Raisin · 3/5 placements");
    const full = screen.getByTestId("match-ranks").querySelector(".match-ranks__full");
    expect(full).toHaveTextContent("Max#3 (you) Large Grape III");
    expect(full?.textContent).not.toContain("Normal Grape");
    expect(reads()).toBe(3);
    await advance(MATCH_RANKS_RETRY_MS * MATCH_RANKS_READ_ATTEMPTS);
    expect(reads()).toBe(3);
  });

  it("R2388000 a Jlorious seat shows its position and a bot's seat its rank, as the bar does", async () => {
    vi.useFakeTimers();
    onTestFinished(() => {
      vi.useRealTimers();
    });
    const jloriousBefore = { tier: "jlorious", position: 12 };
    const jloriousAfter = { tier: "jlorious", position: 9 };
    const mythicBefore = { tier: "mythic", division: 1, pips: 4, pipsPerDivision: 5, floor: "mythic" };
    const mythicAfter = { tier: "mythic", division: 1, pips: 3, pipsPerDivision: 5, floor: "mythic" };
    stubReads([
      {
        ranked: true,
        seats: {
          p1: { ...p1, rank: jloriousAfter, rankBefore: jloriousBefore, rankAfter: jloriousAfter },
          p2: { ...p2, rank: mythicAfter, rankBefore: mythicBefore, rankAfter: mythicAfter },
        },
      },
    ]);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ view: finished });
    await advance(0);

    const moves = screen.getAllByTestId("match-rank-move");
    expect(moves.map((move) => move.getAttribute("data-seat"))).toEqual(["p1", "p2"]);
    expect(moves[0]).toHaveTextContent("Max#3 (you): Jlorious #12 → Jlorious #9");
    expect(moves[1]).toHaveTextContent("Rowan: Mythic Grape I · 4/5 pips → Mythic Grape I · 3/5 pips");
    const full = screen.getByTestId("match-ranks").querySelector(".match-ranks__full");
    expect(full).toHaveTextContent("Max#3 (you) #9 Jlorious");
    expect(full).toHaveTextContent("Rowan Mythic Grape I");
  });

  it("R2388000 an unranked match's result shows no move and reads the ranks once more", async () => {
    vi.useFakeTimers();
    onTestFinished(() => {
      vi.useRealTimers();
    });
    const reads = stubReads([{ ...before, ranked: false }]);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ view: finished });
    await advance(MATCH_RANKS_RETRY_MS * MATCH_RANKS_READ_ATTEMPTS);

    expect(screen.queryByTestId("match-rank-move")).toBeNull();
    expect(reads()).toBe(2);
  });

  it("R2388000 a ranked read whose record never comes stops after MATCH_RANKS_READ_ATTEMPTS reads", async () => {
    vi.useFakeTimers();
    onTestFinished(() => {
      vi.useRealTimers();
    });
    const reads = stubReads([before]);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ view: finished });
    await advance(MATCH_RANKS_RETRY_MS * (MATCH_RANKS_READ_ATTEMPTS + 2));

    expect(reads()).toBe(1 + MATCH_RANKS_READ_ATTEMPTS);
    expect(screen.queryByTestId("match-rank-move")).toBeNull();
    expect(screen.getByTestId("match-ranks")).toHaveTextContent("Max#3 (you) Normal Grape I");
  });
});

describe("the daily Jlore cap on a finished ladder match (R1483000)", () => {
  const finished = baseView({ viewer: "p1", active: "p1", result: { winner: "p1", reason: "hero-death" } });

  function push(view: ReturnType<typeof baseView>, extra: Record<string, unknown> = {}): void {
    act(() => {
      live().onmessage?.({ data: JSON.stringify({ type: "view", view, ...extra }) });
    });
  }

  it("R1483000 shows Daily Jlore reached only on a capped finished match", async () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    // The flag on a game still being played says nothing: the line is the finished game's.
    push(baseView({ viewer: "p1", active: "p1" }), { jloreCapped: true });
    expect(screen.queryByTestId("result-daily-jlore")).toBeNull();

    push(finished, { jloreCapped: true });
    expect(await screen.findByTestId("result-daily-jlore")).toHaveTextContent("Daily Jlore reached");
    cleanup();

    for (const extra of [{ jloreCapped: false }, {}]) {
      render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
      attach({ view: finished, ...extra });
      expect(await screen.findByTestId("result-back")).toBeInTheDocument();
      expect(screen.queryByTestId("result-daily-jlore")).toBeNull();
      cleanup();
    }
  });
});

describe("R169 the modifier list at the online table", () => {
  /** The panel's headings, in order: who each group of labels belongs to. */
  function headings(panel: HTMLElement): (string | null)[] {
    return Array.from(panel.querySelectorAll(".modifier-list__who"), (who) => who.textContent);
  }

  it("R169 shows the chip with both sides' count and opens every label under You and Opponent, never AI", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({
      view: baseView({
        viewer: "p1",
        active: "p1",
        you: emptySide("p1", { modifiers: yourModifiers }),
        opponent: emptySide("p2", { hand: { count: 4 }, modifiers: opponentModifiers }),
      }),
    });

    const chip = screen.getByTestId(modifierListTestid.chip);
    expect(chip).toHaveAttribute("data-count", String(yourModifiers.length + opponentModifiers.length));

    fireEvent.click(chip);
    const panel = screen.getByTestId(modifierListTestid.panel);
    for (const modifier of yourModifiers) {
      expect(panel.querySelector('[data-side="you"]')).toHaveTextContent(modifier.label);
    }
    for (const modifier of opponentModifiers) {
      expect(panel.querySelector('[data-side="opponent"]')).toHaveTextContent(modifier.label);
    }
    expect(headings(panel)).toEqual(["You", "Opponent"]);
  });

  it("R169 a long modifier on the opponent's side alone shows the chip too", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({
      view: baseView({
        viewer: "p1",
        active: "p1",
        opponent: emptySide("p2", { hand: { count: 4 }, modifiers: opponentModifiers }),
      }),
    });

    const chip = screen.getByTestId(modifierListTestid.chip);
    expect(chip).toHaveAttribute("data-count", "1");
    expect(chip).toHaveAccessibleName("1 active modifier");

    fireEvent.click(chip);
    expect(headings(screen.getByTestId(modifierListTestid.panel))).toEqual(["Opponent"]);
  });

  it("R169 shows no chip while no modifier is live", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    expect(screen.queryByTestId(modifierListTestid.chip)).toBeNull();
  });
});

describe("the XP reveal on a finished match (#1664)", () => {
  const finished = baseView({ viewer: "p1", active: "p1", result: { winner: "p1", reason: "hero-death" } });
  const economy = {
    jlore: 30,
    xp: 540,
    jloreCapped: false,
    tracks: [
      {
        setId: "Core",
        before: { level: 1, xp: 60, levelXp: 100, xpToNext: 40 },
        after: { level: 2, xp: 130, levelXp: 200, xpToNext: 170 },
        levels: [{ level: 2, rewards: { jlore: 25, setPacks: 2, cardTokens: 3 } }],
      },
    ],
    quests: [{ questId: "win-3", title: "Win 3 games", jlore: 80, xp: 4000 }],
  };

  function push(view: ReturnType<typeof baseView>, extra: Record<string, unknown> = {}): void {
    act(() => {
      live().onmessage?.({ data: JSON.stringify({ type: "view", view, ...extra }) });
    });
  }

  /** `announceWallet`'s event, counted: the pill reads the wallet again for each. */
  function watchWallet(): () => number {
    let count = 0;
    const onChanged = (): void => {
      count += 1;
    };
    window.addEventListener("jackioh:wallet-changed", onChanged);
    onTestFinished(() => {
      window.removeEventListener("jackioh:wallet-changed", onChanged);
    });
    return () => count;
  }

  it("#1664 plays the tracks and lists the quests under the result once the server's economy arrives and the panel is in", () => {
    vi.useFakeTimers();
    onTestFinished(() => {
      vi.useRealTimers();
    });
    const announced = watchWallet();
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    // The result lands first; the economy comes with the view that follows its write.
    push(finished);
    expect(screen.getByTestId("result-back")).toBeInTheDocument();
    expect(screen.queryByTestId("xp-reveal")).toBeNull();
    expect(announced()).toBe(0);

    push(finished, { economy });
    expect(announced()).toBe(1);
    // The panel waits out the effects layer's Victory, and the bars wait for it: they play where they are seen.
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-revealed", "false");
    expect(screen.queryByTestId("xp-reveal")).toBeNull();
    act(() => {
      vi.advanceTimersByTime(resultRevealMs());
    });
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-revealed", "true");
    expect(screen.getByTestId("xp-reveal-bar-Core")).toHaveTextContent("Core track");
    expect(screen.getByTestId("xp-reveal-level-Core")).toHaveAttribute("data-level", "1");
    expect(screen.getByTestId("xp-reveal-quest-win-3")).toHaveTextContent("Win 3 games");
    expect(screen.getByTestId("xp-reveal-quest-win-3")).toHaveTextContent("80 Jlore · 4,000 XP");
    expect(announced()).toBe(1);

    // Every later view of the seat carries it: the bars play on and the pill is told once.
    push(finished, { economy });
    expect(screen.getAllByTestId("xp-reveal")).toHaveLength(1);
    expect(announced()).toBe(1);
  });

  it("#1664 shows nothing, and tells the pill nothing, for a match that paid no economy", async () => {
    const announced = watchWallet();
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ view: finished });
    expect(await screen.findByTestId("result-back")).toBeInTheDocument();
    expect(screen.queryByTestId("xp-reveal")).toBeNull();
    expect(announced()).toBe(0);
  });

  it("#1664 an economy on a game still being played shows no bars", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ economy });
    expect(screen.queryByTestId("xp-reveal")).toBeNull();
  });
});

describe("the death screen's rematch offers (R672)", () => {
  /** A finished game of a series in this status, shaped like the R259 block's answer. */
  function seriesIn(status: string): Record<string, unknown> {
    const decks = [0, 1, 2].map((slot) => ({
      slot,
      name: `Deck ${String(slot + 1)}`,
      cards: [],
      won: false,
      games: 0,
    }));
    return {
      id: "series-1",
      status,
      gameNo: 1,
      winsNeeded: SERIES_WINS_NEEDED,
      maxGames: SERIES_MAX_GAMES,
      pickDeadline: null,
      now: 0,
      currentMatchId: "m-1",
      ranked: true,
      you: { seat: "p1", wins: 0, trioName: "Main trio", decks, pick: null, autoPick: false },
      opponent: { wins: 0, decks: decks.map(({ slot }) => ({ slot, won: false })), picked: false },
      games: [],
      result: null,
    };
  }

  /** `GET /api/matches/m-1/series` answers this series (or none); `/rematch` says the opponent is here. */
  function stubMatchFetch(series: Record<string, unknown> | null): void {
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) =>
        Promise.resolve({
          ok: true,
          status: 200,
          text: () =>
            Promise.resolve(
              JSON.stringify(
                String(url).includes("/api/matches/m-1/rematch")
                  ? { youOffered: null, opponentOffer: null, opponentHere: true, matchId: null }
                  : String(url).includes("/series")
                    ? { series }
                    : { version: "v1", defs: {} },
              ),
            ),
        } as unknown as Response),
      ),
    );
  }

  function finish(): void {
    attach({
      view: baseView({ viewer: "p1", active: "p1", result: { winner: "p1", reason: "hero-death" } }),
    });
  }

  it("offers a rematch on a finished game that is in no series", async () => {
    stubMatchFetch(null);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    expect(await screen.findByTestId("rematch-offer")).toBeInTheDocument();
    expect(screen.getByTestId("rematch-double")).toBeInTheDocument();
  });

  it("shows no rematch on a live series game, whose continue flow owns what comes next", async () => {
    stubMatchFetch(seriesIn("playing"));
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    // The banner read the series (so the hook ran), yet no rematch rendered for it.
    await screen.findByTestId("series-banner");
    await act(async () => {});
    expect(screen.queryByTestId("rematch-offer")).toBeNull();
    expect(screen.queryByTestId("rematch-double")).toBeNull();
  });

  it("shows no rematch once the series is over either: finished series games are refused too", async () => {
    stubMatchFetch(seriesIn("over"));
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    await screen.findByTestId("series-banner");
    await act(async () => {});
    expect(screen.queryByTestId("rematch-offer")).toBeNull();
    expect(screen.queryByTestId("rematch-double")).toBeNull();
  });

  it("takes the seat to the rematch even after View the board folded the panel", async () => {
    vi.mocked(navigate).mockClear();
    // The rematch answers are held back until the panel is folded, so nothing can have navigated yet.
    const releaseRematch: Array<(body: string) => void> = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) =>
        Promise.resolve({
          ok: true,
          status: 200,
          text: () => {
            if (String(url).includes("/api/matches/m-1/rematch")) {
              return new Promise<string>((resolve) => {
                releaseRematch.push(resolve);
              });
            }
            return Promise.resolve(
              JSON.stringify(String(url).includes("/series") ? { series: null } : { version: "v1", defs: {} }),
            );
          },
        } as unknown as Response),
      ),
    );
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    // Fold the panel to its chip: the buttons unmount with it.
    fireEvent.click(await screen.findByTestId("result-view-board"));
    expect(screen.getByTestId("result-reopen")).toBeInTheDocument();
    expect(screen.queryByTestId("rematch-offer")).toBeNull();
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();

    // The matching offer lands afterwards: the watcher outlives the fold and navigates.
    // (The buttons' released poll is dropped by their unmount guard.)
    await act(async () => {
      for (const release of releaseRematch) {
        release(JSON.stringify({ youOffered: 1, opponentOffer: 1, opponentHere: true, matchId: "match-2" }));
      }
    });
    await waitFor(() => {
      expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-2");
    });
  });

  /** The test above's held rematch answers, plus `stubReplayFetch`'s replay pages and this `/series`. */
  function stubHeldRematchFetch(series: Record<string, unknown> | null): Array<(body: string) => void> {
    const steps = foldedReplaySteps();
    const releaseRematch: Array<(body: string) => void> = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        const address = new URL(String(url), "http://localhost");
        if (address.pathname === "/api/matches/m-1/rematch") {
          return Promise.resolve({
            ok: true,
            status: 200,
            text: () =>
              new Promise<string>((resolve) => {
                releaseRematch.push(resolve);
              }),
          } as unknown as Response);
        }
        const body = address.pathname === "/api/replays/m-1"
          ? (() => {
              const from = Number(address.searchParams.get("from") ?? "0");
              const count = Number(address.searchParams.get("count") ?? "1");
              return { matchId: "m-1", from, steps: steps.slice(from, from + count), total: steps.length };
            })()
          : address.pathname.endsWith("/series")
            ? { series }
            : { version: "v1", defs: {} };
        return Promise.resolve({ ok: true, status: 200, text: () => Promise.resolve(JSON.stringify(body)) } as unknown as Response);
      }),
    );
    return releaseRematch;
  }

  const AGREED = JSON.stringify({ youOffered: 1, opponentOffer: 1, opponentHere: true, matchId: "match-2" });

  it("takes the seat to the rematch even from the finished match's replay", async () => {
    vi.mocked(navigate).mockClear();
    // Held until the replay is open: the board's polls are then dropped by their unmount guard, so only
    // a watcher mounted beside the replay can navigate.
    const releaseRematch = stubHeldRematchFetch(null);
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    fireEvent.click(await screen.findByTestId(matchTestid.watchReplay));
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", "0"));
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();

    await act(async () => {
      for (const release of releaseRematch) release(AGREED);
    });
    await waitFor(() => {
      expect(vi.mocked(navigate)).toHaveBeenCalledWith("/match/match-2");
    });
  });

  it("takes a series game's seat to no rematch from the board or from its replay, and Back shows the result again", async () => {
    vi.mocked(navigate).mockClear();
    const releaseRematch = stubHeldRematchFetch(seriesIn("playing"));
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    // The board: once the series is in, nothing that navigates is still polling, so the agreed answer goes nowhere.
    await screen.findByTestId("series-banner");
    await act(async () => {
      for (const release of releaseRematch) release(AGREED);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();

    // The replay: no watcher mounts beside it either.
    fireEvent.click(await screen.findByTestId(matchTestid.watchReplay));
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", "0"));
    await act(async () => {
      for (const release of releaseRematch) release(AGREED);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(await screen.findByTestId(matchTestid.watchReplay)).toBeInTheDocument();
    expect(screen.queryByTestId(replayTestid.player)).toBeNull();
    expect(screen.queryByTestId("rematch-offer")).toBeNull();
    expect(vi.mocked(navigate)).not.toHaveBeenCalled();
  });
});

describe("a socket refused mid-game", () => {
  it("tells the player in words, not with the server's reason", () => {
    const logged = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try {
      render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
      attach({ legal: [{ type: "endTurn" }] });
      act(() => {
        live().onclose?.({ code: FORBIDDEN_CLOSE, reason: "", wasClean: true });
      });
      expect(document.body.textContent).not.toMatch(/refused this match socket/);
      expect(screen.getByTestId("match-status")).toHaveAttribute("data-connection", "refused");
      expect(document.body.textContent).toMatch(/This match isn't available/);
      // A refusal is settled: no retry is offered.
      expect(screen.queryByTestId("match-reconnect")).toBeNull();
    } finally {
      logged.mockRestore();
    }
  });
});

describe("a match voided mid-game (#1899)", () => {
  it("#1899 a match voided on a live board says it is gone and offers the lobby, not a reconnect", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    expect(screen.queryByTestId("match-gone")).toBeNull();

    act(() => {
      live().onclose?.({ code: VOIDED_CLOSE_CODE, reason: "voided", wasClean: true });
    });
    expect(screen.getByTestId("match-status")).toHaveAttribute("data-connection", "closed");
    expect(screen.getByTestId("match-gone")).toHaveTextContent("This match is no longer available.");
    expect(screen.getByTestId("hero-you")).toBeInTheDocument();
    expect(screen.queryByTestId("match-reconnect")).toBeNull();
    expect(sockets).toHaveLength(1);

    const link = within(screen.getByTestId("match-gone")).getByRole("link", { name: "Back to the lobby" });
    expect(link).toHaveAttribute("href", "/play");
    vi.mocked(navigate).mockClear();
    fireEvent.click(link);
    expect(vi.mocked(navigate)).toHaveBeenCalledWith("/play");
  });

  it("#1899 a voided result's view, then the void's close, shows the gone notice", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    // What the server sends a voided match: its last view, then the close (R679).
    pushView(baseView({ viewer: "p1", active: "p1", phase: "over", result: { winner: "draw", reason: "voided" } }));
    act(() => {
      live().onclose?.({ code: VOIDED_CLOSE_CODE, reason: "voided", wasClean: true });
    });
    expect(screen.getByTestId("match-gone")).toHaveTextContent("This match is no longer available.");
    expect(screen.getByTestId("match-status")).toHaveAttribute("data-connection", "closed");
  });

  it("#1899 an ordinary drop mid-game still says Reconnecting… with Reconnect now, and no gone notice", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    act(() => {
      live().onclose?.({ code: 1006, reason: "", wasClean: false });
    });
    const status = screen.getByTestId("match-status");
    expect(status).toHaveAttribute("data-connection", "reconnecting");
    expect(status).toHaveTextContent("Reconnecting…");
    expect(screen.getByTestId("match-reconnect")).toHaveTextContent("Reconnect now");
    expect(screen.queryByTestId("match-gone")).toBeNull();
  });
});

describe("Reconnect now", () => {
  /** Drop the live socket the way a dead network does: an unclean close that backs off. */
  function drop(): void {
    act(() => {
      live().onclose?.({ code: 1006, reason: "", wasClean: false });
    });
  }

  it("shows only while the socket is reconnecting, and a click opens a socket at once", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    expect(screen.queryByTestId("match-reconnect")).toBeNull();

    attach({ legal: [{ type: "endTurn" }] });
    expect(screen.queryByTestId("match-reconnect")).toBeNull();

    drop();
    expect(screen.getByTestId("match-status")).toHaveAttribute("data-connection", "reconnecting");
    expect(screen.getByTestId("match-reconnect")).toHaveTextContent("Reconnect now");

    fireEvent.click(screen.getByTestId("match-reconnect"));
    expect(sockets).toHaveLength(2);

    attach({ legal: [{ type: "endTurn" }] });
    expect(screen.getByTestId("match-status")).toHaveAttribute("data-connection", "open");
    expect(screen.queryByTestId("match-reconnect")).toBeNull();
  });

  it("is offered on the holding panel too, when the first socket drops before any view", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    drop();
    expect(screen.getByTestId("match-connecting")).toContainElement(screen.getByTestId("match-reconnect"));

    fireEvent.click(screen.getByTestId("match-reconnect"));
    expect(sockets).toHaveLength(2);
  });

  it("a reconnect's first view takes down the not-connected alert", () => {
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ legal: [{ type: "endTurn" }] });
    drop();

    fireEvent.click(screen.getByTestId("end-turn"));
    expect(screen.getByTestId("action-error")).toHaveTextContent("not connected to the match");

    fireEvent.click(screen.getByTestId("match-reconnect"));
    act(() => {
      live().readyState = 1;
      live().onopen?.({});
    });
    // An open alone proves nothing: the alert stays until the server answers.
    expect(screen.getByTestId("action-error")).toHaveTextContent("not connected to the match");

    act(() => {
      live().onmessage?.({
        data: JSON.stringify({
          type: "view",
          view: baseView({ viewer: "p1", active: "p1" }),
          legal: [{ type: "endTurn" }],
        }),
      });
    });
    expect(screen.queryByTestId("action-error")).toBeNull();
  });
});

describe("R768000 a finished match's replay from its result panel", () => {
  const steps = foldedReplaySteps();

  /** `GET /api/replays/m-1?from=&count=` answers the folded game's steps; anything else the catalog. */
  function stubReplayFetch(): string[] {
    const asked: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((url: string) => {
        const address = new URL(String(url), "http://localhost");
        const body = address.pathname === "/api/replays/m-1"
          ? (() => {
              asked.push(address.search);
              const from = Number(address.searchParams.get("from") ?? "0");
              const count = Number(address.searchParams.get("count") ?? "1");
              return { matchId: "m-1", from, steps: steps.slice(from, from + count), total: steps.length };
            })()
          : address.pathname.endsWith("/series")
            ? { series: null }
            : address.pathname.endsWith("/rematch")
              ? { youOffered: null, opponentOffer: null, opponentHere: false, matchId: null }
              : { version: "v1", defs: {} };
        return Promise.resolve({ ok: true, status: 200, text: () => Promise.resolve(JSON.stringify(body)) } as unknown as Response);
      }),
    );
    return asked;
  }

  function finish(): void {
    attach({ view: baseView({ viewer: "p1", active: "p1", result: { winner: "p1", reason: "hero-death" } }) });
  }

  it("R768000 Watch the replay loads the match's replay at step 0, and Back returns to the board", async () => {
    const asked = stubReplayFetch();
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    fireEvent.click(await screen.findByTestId(matchTestid.watchReplay));
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", "0"));
    expect(asked[0]).toContain("from=0");

    fireEvent.click(screen.getByTestId(replayTestid.back));
    expect(await screen.findByTestId(matchTestid.watchReplay)).toBeInTheDocument();
    expect(screen.queryByTestId(replayTestid.player)).toBeNull();
  });

  it("R768000 Share the last turn opens it at its last step with Share as GIF open", async () => {
    stubReplayFetch();
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    finish();

    fireEvent.click(await screen.findByTestId(matchTestid.shareLastTurn));
    await waitFor(() => expect(screen.getByTestId(replayTestid.player)).toHaveAttribute("data-step", String(steps.length - 1)));
    await waitFor(() => expect(screen.getByTestId(gifTestid.toggle)).toHaveAttribute("aria-expanded", "true"));
  });

  it("R768000 a game still being played offers neither", () => {
    stubReplayFetch();
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach();
    expect(screen.queryByTestId(matchTestid.watchReplay)).toBeNull();
    expect(screen.queryByTestId(matchTestid.shareLastTurn)).toBeNull();
  });

  it("a voided match's result offers neither, since the server keeps no record of it", async () => {
    stubReplayFetch();
    render(<MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />);
    attach({ view: baseView({ viewer: "p1", active: "p1", phase: "over", result: { winner: "draw", reason: "voided" } }) });
    expect(await screen.findByTestId("result-back")).toBeInTheDocument();
    expect(screen.queryByTestId(matchTestid.watchReplay)).toBeNull();
    expect(screen.queryByTestId(matchTestid.shareLastTurn)).toBeNull();
  });
});

// R2479000: the leave guard (R1754000) holds every way off a live online board. The real `navigate`
// goes back in for these tests so that the board's Back meets the guard; the rest keep the bare mock.
describe("R2479000 Back in a live online match asks first, and names the disconnect grace", () => {
  const WORDS = `It goes on without you. Come back within ${String(DISCONNECT_GRACE_SECONDS)} seconds or it counts as a loss.`;
  const AT = paths.match("m-1");
  const over = baseView({ viewer: "p1", active: "p1", phase: "over", result: { winner: "p2", reason: "hero-death" } });

  /** The match, and what a move off it lands on: `usePathname`'s `popstate` listener needs a reader. */
  function Shell(): ReactElement {
    const path = usePathname();
    return path === AT ? (
      <MatchRoute matchId="m-1" token="tok" socketFactory={socketFactory} />
    ) : (
      <p data-testid="elsewhere">{path}</p>
    );
  }

  /** The browser's Back, waited for until its `popstate` has fired. */
  async function browserBack(): Promise<void> {
    await act(async () => {
      const popped = new Promise((resolve) => {
        window.addEventListener("popstate", resolve, { once: true });
      });
      window.history.back();
      await popped;
    });
  }

  beforeEach(async () => {
    const actual = await vi.importActual<typeof import("../net/navigate.ts")>("../net/navigate.ts");
    vi.mocked(navigate).mockImplementation(actual.navigate);
    window.history.replaceState(null, "", paths.play);
    window.history.pushState(null, "", AT);
  });

  afterEach(() => {
    vi.mocked(navigate).mockReset();
  });

  it("R2479000 the board's Back on a live match asks first and stays; Stay sends nothing, Leave goes to the menu", () => {
    render(<Shell />);
    attach();
    const sent = live().sent.length;

    fireEvent.click(screen.getByTestId(navTestid.back));
    const dialog = screen.getByTestId(leaveMatchTestid.dialog);
    expect(window.location.pathname).toBe(AT);
    expect(dialog).toHaveTextContent("Leave the match?");
    expect(dialog).toHaveTextContent(WORDS);
    expect(document.activeElement).toBe(screen.getByTestId(leaveMatchTestid.stay));

    fireEvent.click(screen.getByTestId(leaveMatchTestid.stay));
    expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();
    expect(window.location.pathname).toBe(AT);
    expect(live().sent).toHaveLength(sent);

    fireEvent.click(screen.getByTestId(navTestid.back));
    fireEvent.click(screen.getByTestId(leaveMatchTestid.leave));
    expect(window.location.pathname).toBe(paths.landing);
    expect(screen.getByTestId("elsewhere")).toHaveTextContent(paths.landing);
  });

  it("R2479000 Escape and the browser's Back on the question answer Stay", async () => {
    render(<Shell />);
    attach();

    fireEvent.click(screen.getByTestId(navTestid.back));
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();
    expect(window.location.pathname).toBe(AT);

    fireEvent.click(screen.getByTestId(navTestid.back));
    await browserBack();
    expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();
    expect(window.location.pathname).toBe(AT);
  });

  it("R2479000 the browser's Back off a live match is held the same way, and Leave goes back", async () => {
    render(<Shell />);
    attach();

    await browserBack();
    expect(window.location.pathname).toBe(AT);
    expect(screen.getByTestId(leaveMatchTestid.dialog)).toBeInTheDocument();

    fireEvent.click(screen.getByTestId(leaveMatchTestid.stay));
    // This Back takes two steps over the dialog's spent back-layer entry, so it waits for the question.
    await browserBack();
    await screen.findByTestId(leaveMatchTestid.dialog);
    expect(window.location.pathname).toBe(AT);

    fireEvent.click(screen.getByTestId(leaveMatchTestid.leave));
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.play);
    });
    expect(await screen.findByTestId("elsewhere")).toHaveTextContent(paths.play);
  });

  it("R2479000 once the view has a result the question closes and Back leaves at once", () => {
    render(<Shell />);
    attach();

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(screen.getByTestId(leaveMatchTestid.dialog)).toBeInTheDocument();

    pushView(over);
    expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();

    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("R2479000 a match voided or refused mid-game is left at once", () => {
    render(<Shell />);
    attach();

    act(() => {
      live().onclose?.({ code: VOIDED_CLOSE_CODE, reason: "voided", wasClean: true });
    });
    fireEvent.click(screen.getByTestId(navTestid.back));
    expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();
    expect(window.location.pathname).toBe(paths.landing);

    const logged = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try {
      act(() => {
        navigate(AT);
      });
      attach();
      act(() => {
        live().onclose?.({ code: FORBIDDEN_CLOSE, reason: "", wasClean: true });
      });
      fireEvent.click(screen.getByTestId(navTestid.back));
      expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();
      expect(window.location.pathname).toBe(paths.landing);
    } finally {
      logged.mockRestore();
    }
  });

  it("R2479000 a forced move goes at once, an unforced one is held, and the guard leaves with the route", () => {
    render(<Shell />);
    attach();

    act(() => {
      navigate(paths.together);
    });
    expect(screen.getByTestId(leaveMatchTestid.dialog)).toBeInTheDocument();
    expect(window.location.pathname).toBe(AT);
    fireEvent.click(screen.getByTestId(leaveMatchTestid.stay));

    act(() => {
      navigate(paths.play, { force: true });
    });
    expect(window.location.pathname).toBe(paths.play);
    expect(screen.queryByTestId(leaveMatchTestid.dialog)).toBeNull();
    expect(screen.getByTestId("elsewhere")).toBeInTheDocument();

    act(() => {
      navigate(paths.landing);
    });
    expect(window.location.pathname).toBe(paths.landing);
  });
});
