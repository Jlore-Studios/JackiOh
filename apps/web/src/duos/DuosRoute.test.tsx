// `/duos/<matchId>`'s connection line: "Reconnect now" shows while the socket is reconnecting and opens
// one at once (the 1v1 match screen's own, `routes/match.test.tsx`). The socket is a fake `WebSocket`
// the test drives; the catalog is a stubbed `fetch` (`test/stubServer.ts`). The route is shown on its
// holding panel, before any view: the board's chrome draws the same line.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SocketLike } from "../game/net.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { must, stubServer } from "../test/stubServer.ts";
import { DuosRoute } from "./DuosRoute.tsx";
import { duosTestid } from "./testids.ts";

let sockets: FakeDuosSocket[] = [];

class FakeDuosSocket implements SocketLike {
  readyState = 0;
  onopen: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;

  constructor(readonly url: string) {
    sockets.push(this);
  }

  send(): void {}

  close(): void {
    this.readyState = 3;
  }
}

/** The newest socket the route opened. */
function latest(): FakeDuosSocket {
  return must(sockets.at(-1), "a socket");
}

beforeEach(() => {
  sockets = [];
  vi.stubGlobal("WebSocket", FakeDuosSocket);
  window.localStorage.clear();
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-duos" }));
  stubServer((call) =>
    call.method === "GET" && call.path === "/api/catalog" ? { status: 200, body: { version: "v1", defs: {} } } : undefined,
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("the co-op route's connection line", () => {
  it("offers Reconnect now only while reconnecting, and a click opens a socket at once", () => {
    render(<DuosRoute matchId="dm-1" token="tok-duos" />);
    expect(sockets).toHaveLength(1);
    expect(screen.queryByTestId(duosTestid.reconnect)).toBeNull();

    // The way a dead network drops a socket: an unclean close that backs off.
    act(() => {
      latest().onclose?.({ code: 1006, reason: "", wasClean: false });
    });
    expect(screen.getByTestId(duosTestid.connection)).toHaveAttribute("data-connection", "reconnecting");
    const button = screen.getByTestId(duosTestid.reconnect);
    expect(screen.getByTestId(duosTestid.connecting)).toContainElement(button);
    expect(button).toHaveTextContent("Reconnect now");

    // Before any backoff could have run.
    fireEvent.click(button);
    expect(sockets).toHaveLength(2);

    act(() => {
      latest().readyState = 1;
      latest().onopen?.({});
    });
    expect(screen.getByTestId(duosTestid.connection)).toHaveAttribute("data-connection", "open");
    expect(screen.queryByTestId(duosTestid.reconnect)).toBeNull();
  });
});

describe("the co-op route's token", () => {
  it("R1115000 the socket carries the gate's token, and a renewed one reopens a socket refused with 4401", () => {
    const { rerender } = render(<DuosRoute matchId="dm-1" token="tok-old" />);
    expect(sockets).toHaveLength(1);
    expect(latest().url).toContain("token=tok-old");
    act(() => {
      latest().onclose?.({ code: 4401, reason: "sign in first", wasClean: false });
    });
    expect(screen.getByTestId(duosTestid.refused)).toBeInTheDocument();
    rerender(<DuosRoute matchId="dm-1" token="tok-new" />);
    expect(sockets).toHaveLength(2);
    expect(latest().url).toContain("token=tok-new");
  });
});
