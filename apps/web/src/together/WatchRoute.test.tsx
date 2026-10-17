// #936 S7 B50: `/watch/:matchId?friend=<id>`, a friend's live game. `useWatch` (read through
// `WatchRoute`) opens `/ws/watch` with the session's token, the match and the friend, says hello once
// the socket opens, shows the waiting panel (naming the delay) until the first delayed view and then
// the friend's board read-only, shows a `forbidden` frame as the refused screen with the way back and
// never reopens it, opens again `WATCH_RECONNECT_MS` after an unexpected close up to
// `WATCH_RECONNECT_ATTEMPTS` times, and does not reopen a finished game.
//
// The socket is a fake `WebSocket` the test drives frame by frame; the friends list and the catalog are
// a stubbed `fetch` (`test/stubServer.ts`). Under fake timers `waitFor` hangs, so those tests advance
// time by hand.

import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SPECTATE_DELAY_MS } from "@jackioh/server-config";

import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { SILENT_SOCKET_MS, WAKE_PROBE_MS } from "../game/net.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { Friend } from "../net/social.ts";
import { baseView } from "../test/fixtures.ts";
import { must, stubServer } from "../test/stubServer.ts";
import { WATCH_RECONNECT_ATTEMPTS, WATCH_RECONNECT_MS } from "./config.ts";
import { WatchRoute } from "./WatchRoute.tsx";

/** A stubbed round trip plus a lazy route chunk can outrun the 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const ANN: Friend = {
  profileId: "f-ann",
  username: "Ann#2",
  status: "inMatch",
  matchId: "m-watch",
  watchable: true,
  since: "2026-10-01T00:00:00.000Z",
};

function serve(): void {
  stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/friends") {
      return { status: 200, body: { friends: [ANN], incoming: [], outgoing: [], blocked: [] } };
    }
    if (call.method === "GET" && call.path === "/api/catalog") return { status: 200, body: { version: "v1", defs: {} } };
    return undefined;
  });
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-watch" }));
}

// ---------------------------------------------------------------------------------------------
// a WebSocket the test drives
// ---------------------------------------------------------------------------------------------

let sockets: FakeWatchSocket[] = [];

type Listener = (event: { data?: unknown }) => void;

class FakeWatchSocket {
  readonly sent: string[] = [];
  closed = false;
  readyState = 0;
  private readonly listeners = new Map<string, Listener[]>();

  constructor(readonly url: string) {
    sockets.push(this);
  }

  addEventListener(type: string, listener: Listener): void {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener]);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    this.readyState = 3;
    this.closed = true;
  }

  emit(type: "open" | "message" | "close", event: { data?: unknown } = {}): void {
    if (type === "open") this.readyState = 1;
    if (type === "close") this.readyState = 3;
    act(() => {
      for (const listener of this.listeners.get(type) ?? []) listener(event);
    });
  }

  frame(frame: Record<string, unknown>): void {
    this.emit("message", { data: JSON.stringify(frame) });
  }
}

/** The newest socket the screen opened. */
function latest(): FakeWatchSocket {
  return must(sockets.at(-1), "a socket");
}

/** Advance fake time by `ms` and let React's updates settle. */
async function advance(ms: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

function wakeTab(): void {
  act(() => {
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "visible" });
    document.dispatchEvent(new Event("visibilitychange"));
  });
}

function connection(testId: string): string | null {
  return screen.getByTestId(testId).getAttribute("data-connection");
}

function watch(): void {
  render(<WatchRoute matchId="m-watch" friendId="f-ann" />);
}

beforeEach(() => {
  sockets = [];
  vi.stubGlobal("WebSocket", FakeWatchSocket);
  window.localStorage.clear();
  signedIn();
  serve();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("B50 the watch screen", () => {
  it("B50 opens /ws/watch with the token, the match and the friend, and says hello once it opens", () => {
    watch();
    expect(sockets).toHaveLength(1);
    const socket = latest();
    const url = new URL(socket.url);
    expect(url.pathname).toBe("/ws/watch");
    expect(url.searchParams.get("token")).toBe("tok-watch");
    expect(url.searchParams.get("matchId")).toBe("m-watch");
    expect(url.searchParams.get("friendId")).toBe("f-ann");
    expect(connection("watch-screen")).toBe("connecting");
    expect(socket.sent).toEqual([]);

    socket.emit("open");
    expect(connection("watch-screen")).toBe("open");
    expect(socket.sent.map((text) => JSON.parse(text) as unknown)).toEqual([{ type: "hello", matchId: "m-watch", friendId: "f-ann" }]);
  });

  it("B50 waits, naming the delay, until a view draws the friend's board read-only", async () => {
    watch();
    const socket = latest();
    socket.emit("open");
    expect(screen.getByTestId("watch-waiting")).toHaveTextContent(String(Math.round(SPECTATE_DELAY_MS / MS_PER_SECOND)));
    expect(screen.queryByTestId("game")).toBeNull();

    socket.frame({ type: "view", view: baseView({ viewer: "p1" }), legal: [] });
    expect(screen.queryByTestId("watch-waiting")).toBeNull();
    expect(screen.getByTestId("watch-status").getAttribute("data-connection")).toBe("open");
    expect(screen.getByTestId("game").getAttribute("data-replay")).toBe("true");
    await waitFor(() => {
      expect(screen.getByTestId("watch-status")).toHaveTextContent("Ann#2");
    }, SLOW);
    // The screen sends nothing after its hello (CLAUDE.md rule 7).
    expect(socket.sent).toHaveLength(1);
  });

  it("B50 a forbidden frame is the refused screen with the way back, never reopened", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    watch();
    const socket = latest();
    socket.emit("open");
    socket.frame({ type: "error", code: "forbidden", message: "you may not watch this match" });
    socket.emit("close");

    expect(screen.getByTestId("watch-refused")).toBeInTheDocument();
    expect(connection("watch-screen")).toBe("refused");
    expect(screen.getByTestId("watch-back").getAttribute("href")).toBe("/together");
    await advance(WATCH_RECONNECT_MS * (WATCH_RECONNECT_ATTEMPTS + 1));
    expect(sockets).toHaveLength(1);
  });

  it("B50 with no session it is refused and opens no socket", () => {
    window.localStorage.clear();
    watch();
    expect(screen.getByTestId("watch-refused")).toBeInTheDocument();
    expect(connection("watch-screen")).toBe("refused");
    expect(sockets).toHaveLength(0);
  });

  it("B50 an unexpected close keeps the last view, says Reconnecting, and opens again WATCH_RECONNECT_MS later", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    watch();
    const first = latest();
    first.emit("open");
    first.frame({ type: "view", view: baseView({ viewer: "p1" }), legal: [] });
    first.emit("close");

    expect(screen.getByTestId("watch-status").getAttribute("data-connection")).toBe("connecting");
    expect(screen.getByTestId("game")).toBeInTheDocument();
    await advance(WATCH_RECONNECT_MS - 1);
    expect(sockets).toHaveLength(1);
    await advance(1);
    expect(sockets).toHaveLength(2);

    const second = latest();
    second.emit("open");
    expect(screen.getByTestId("watch-status").getAttribute("data-connection")).toBe("open");
    expect(second.sent.map((text) => JSON.parse(text) as unknown)).toEqual([{ type: "hello", matchId: "m-watch", friendId: "f-ann" }]);
  });

  it("B50 stops after WATCH_RECONNECT_ATTEMPTS reconnects that never open", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    watch();
    for (let attempt = 1; attempt <= WATCH_RECONNECT_ATTEMPTS; attempt += 1) {
      latest().emit("close");
      await advance(WATCH_RECONNECT_MS);
      expect(sockets).toHaveLength(attempt + 1);
    }
    latest().emit("close");
    expect(connection("watch-screen")).toBe("closed");
    await advance(WATCH_RECONNECT_MS * 2);
    expect(sockets).toHaveLength(WATCH_RECONNECT_ATTEMPTS + 1);
  });

  it("B50 a finished game is not reopened", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    watch();
    const socket = latest();
    socket.emit("open");
    socket.frame({ type: "view", view: baseView({ viewer: "p1", result: { winner: "p1", reason: "concede" } }), legal: [] });
    socket.emit("close");

    expect(screen.getByTestId("watch-status").getAttribute("data-connection")).toBe("closed");
    await advance(WATCH_RECONNECT_MS);
    expect(sockets).toHaveLength(1);
  });

  it("R1441 probes a silent watch socket on tab visibility and replaces it if unresponsive", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date", "performance"] });
    watch();
    const first = latest();
    first.emit("open");
    first.frame({ type: "view", view: baseView({ viewer: "p1" }), legal: [] });
    expect(first.sent).toHaveLength(1);

    await advance(SILENT_SOCKET_MS + 1);
    wakeTab();
    expect(first.sent.map((text) => JSON.parse(text) as unknown)).toEqual([
      { type: "hello", matchId: "m-watch", friendId: "f-ann" },
      { type: "hello", matchId: "m-watch", friendId: "f-ann" },
    ]);
    expect(sockets).toHaveLength(1);

    await advance(WAKE_PROBE_MS);
    expect(first.closed).toBe(true);
    expect(sockets).toHaveLength(2);
    expect(connection("watch-screen")).toBe("connecting");

    const second = latest();
    second.emit("open");
    expect(connection("watch-screen")).toBe("open");
    expect(second.sent.map((text) => JSON.parse(text) as unknown)).toEqual([
      { type: "hello", matchId: "m-watch", friendId: "f-ann" },
    ]);

    first.frame({ type: "view", view: baseView({ viewer: "p1", turn: 9 }), legal: [] });
    expect(screen.getByTestId("game")).toBeInTheDocument();
  });

  it("R1441 keeps a quiet watch socket that answers the wake probe", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date", "performance"] });
    watch();
    const socket = latest();
    socket.emit("open");
    socket.frame({ type: "view", view: baseView({ viewer: "p1" }), legal: [] });
    expect(socket.sent).toHaveLength(1);

    await advance(SILENT_SOCKET_MS + 1);
    wakeTab();
    expect(socket.sent).toHaveLength(2);

    socket.frame({ type: "view", view: baseView({ viewer: "p1", turn: 2 }), legal: [] });
    await advance(WAKE_PROBE_MS * 2);

    expect(sockets).toHaveLength(1);
    expect(socket.closed).toBe(false);
    expect(connection("watch-screen")).toBe("open");

    wakeTab();
    expect(socket.sent).toHaveLength(2);
  });

  it("R1441 does not probe a watch socket heard within the ping interval", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date", "performance"] });
    watch();
    const socket = latest();
    socket.emit("open");
    socket.frame({ type: "view", view: baseView({ viewer: "p1" }), legal: [] });
    expect(socket.sent).toHaveLength(1);

    await advance(SILENT_SOCKET_MS - 1000);
    wakeTab();
    expect(socket.sent).toHaveLength(1);
    await advance(WAKE_PROBE_MS);
    expect(sockets).toHaveLength(1);
    expect(socket.closed).toBe(false);
  });

  it("R1441 does not probe or reopen a finished or refused game on wake", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date", "performance"] });
    watch();
    const socket = latest();
    socket.emit("open");
    socket.frame({
      type: "view",
      view: baseView({ viewer: "p1", result: { winner: "p1", reason: "concede" } }),
      legal: [],
    });
    expect(socket.sent).toHaveLength(1);

    await advance(SILENT_SOCKET_MS + 1);
    wakeTab();
    expect(socket.sent).toHaveLength(1);
    await advance(WAKE_PROBE_MS);
    expect(sockets).toHaveLength(1);

    socket.emit("close");
    expect(connection("watch-screen")).toBe("closed");
    await advance(SILENT_SOCKET_MS + 1);
    wakeTab();
    expect(sockets).toHaveLength(1);

    cleanup();
    sockets = [];
    watch();
    const refusedSocket = latest();
    refusedSocket.emit("open");
    refusedSocket.frame({ type: "error", code: "forbidden", message: "refused" });
    refusedSocket.emit("close");
    expect(connection("watch-screen")).toBe("refused");

    await advance(SILENT_SOCKET_MS + 1);
    wakeTab();
    expect(sockets).toHaveLength(1);
  });

  it("R1441 wake replacements respect WATCH_RECONNECT_ATTEMPTS", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date", "performance"] });
    watch();
    const first = latest();
    first.emit("open");
    first.frame({ type: "view", view: baseView({ viewer: "p1" }), legal: [] });

    for (let attempt = 1; attempt <= WATCH_RECONNECT_ATTEMPTS; attempt += 1) {
      await advance(SILENT_SOCKET_MS + 1);
      wakeTab();
      if (attempt === 1) {
        await advance(WAKE_PROBE_MS);
      }
      expect(sockets).toHaveLength(attempt + 1);
    }

    await advance(SILENT_SOCKET_MS + 1);
    wakeTab();
    expect(connection("watch-screen")).toBe("closed");
    expect(sockets).toHaveLength(WATCH_RECONNECT_ATTEMPTS + 1);
    expect(latest().closed).toBe(true);
    expect(sockets.every((s) => s.closed)).toBe(true);

    await advance(SILENT_SOCKET_MS + 1);
    wakeTab();
    expect(sockets).toHaveLength(WATCH_RECONNECT_ATTEMPTS + 1);
  });
});
