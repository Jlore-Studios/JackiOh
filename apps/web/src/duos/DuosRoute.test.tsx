// `/duos/<matchId>`'s connection line: "Reconnect now" shows while the socket is reconnecting and opens
// one at once (the 1v1 match screen's own, `routes/match.test.tsx`). The socket is a fake `WebSocket`
// the test drives; the catalog is a stubbed `fetch` (`test/stubServer.ts`). The route is shown on its
// holding panel, before any view: the board's chrome draws the same line.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerView } from "@jackioh/shared";

import { VOIDED_CLOSE_CODE, type SocketLike } from "../game/net.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { tokenFromProtocols } from "../net/socketProtocols.ts";
import { baseView, duosTeamView, duosView, pendingFor } from "../test/fixtures.ts";
import { must, stubServer } from "../test/stubServer.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { DuosRoute } from "./DuosRoute.tsx";
import { duosTestid } from "./testids.ts";

let sockets: FakeDuosSocket[] = [];

class FakeDuosSocket implements SocketLike {
  readyState = 0;
  onopen: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;

  constructor(
    readonly url: string,
    readonly protocols: readonly string[] = [],
  ) {
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

describe("the co-op route before its first view", () => {
  it("#1599 a co-op match voided before the first view says it is gone and offers Play together, not Joining", () => {
    render(<DuosRoute matchId="dm-1" token="tok-duos" />);
    expect(screen.getByTestId(duosTestid.connecting)).toHaveTextContent("Joining the co-op match…");

    act(() => {
      latest().onclose?.({ code: VOIDED_CLOSE_CODE, reason: "", wasClean: true });
    });
    expect(screen.getByTestId(duosTestid.gone)).toHaveTextContent("This co-op match is no longer available.");
    expect(screen.getByTestId(duosTestid.connecting)).not.toHaveTextContent("Joining the co-op match…");
    expect(screen.queryByTestId("gate-loading")).toBeNull();
    expect(screen.queryByTestId(duosTestid.refused)).toBeNull();
    expect(screen.queryByTestId(duosTestid.reconnect)).toBeNull();
    expect(screen.getByRole("link", { name: "Back to Play together" })).toHaveAttribute("href", "/together");
    expect(sockets).toHaveLength(1);
  });
});

describe("the co-op route's token", () => {
  it("R1115000 the socket carries the gate's token, and a renewed one reopens a socket refused with 4401", () => {
    const { rerender } = render(<DuosRoute matchId="dm-1" token="tok-old" />);
    expect(sockets).toHaveLength(1);
    expect(tokenFromProtocols(latest().protocols)).toBe("tok-old");
    expect(latest().url).not.toContain("tok-");
    act(() => {
      latest().onclose?.({ code: 4401, reason: "sign in first", wasClean: false });
    });
    expect(screen.getByTestId(duosTestid.refused)).toBeInTheDocument();
    rerender(<DuosRoute matchId="dm-1" token="tok-new" />);
    expect(sockets).toHaveLength(2);
    expect(tokenFromProtocols(latest().protocols)).toBe("tok-new");
    expect(latest().url).not.toContain("tok-");
  });
});

describe("the co-op route's tab title (#1792)", () => {
  function open(): void {
    act(() => {
      latest().readyState = 1;
      latest().onopen?.({});
    });
  }
  /** A `view` frame as the server pushes one; the title reads the view alone, so nothing is legal. */
  function push(view: DuosView): void {
    act(() => {
      latest().onmessage?.({ data: JSON.stringify({ type: "view", view, legal: [] }) });
    });
  }
  /** A1's seat (duel 0) at `round`: its own duel's view as given, and its team's ended flags. */
  function a1(round: number, own: Partial<PlayerView>, ended: boolean[] = [false, false]): DuosView {
    return { ...duosView(round), own: baseView(own), team: { ...duosTeamView("a"), ended } };
  }

  it("#1792 says Your turn while the seat owes its mulligan, not once it is ready, nor while round 0 waits on the other duel", () => {
    render(<DuosRoute matchId="dm-1" token="tok-duos" />);
    open();
    push(
      a1(0, {
        turn: 0,
        phase: "mulligan",
        mulligan: { youReady: false, opponentReady: false },
        pending: pendingFor("mulligan", [], { min: 0, max: 0 }),
      }),
    );
    expect(document.title).toBe("Your turn · JackiOh");
    push(
      a1(0, {
        turn: 0,
        phase: "mulligan",
        mulligan: { youReady: true, opponentReady: false, kept: [] },
        pending: { forYou: false, pendingFor: "p2" },
      }),
    );
    expect(document.title).toBe("Duos · JackiOh");
    // Its duel's mulligans are in and it is that duel's active Player 1, but the round has not opened.
    push(a1(0, { turn: 0 }));
    expect(document.title).toBe("Duos · JackiOh");
  });

  it("#1792 says Your turn for a prompt the seat holds, even once it has ended its turn, and not for the enemy's on its own turn", () => {
    render(<DuosRoute matchId="dm-1" token="tok-duos" />);
    open();
    const flood = pendingFor("discover", [{ key: "mode:core-043", label: "Flood", defId: "core-043" }]);
    push(a1(3, {}));
    expect(document.title).toBe("Your turn · JackiOh");
    push(a1(3, { pending: { forYou: false, pendingFor: "p2" } }));
    expect(document.title).toBe("Duos · JackiOh");
    push(a1(3, { active: "p2", pending: flood }));
    expect(document.title).toBe("Your turn · JackiOh");
    push(a1(3, {}, [true, false]));
    expect(document.title).toBe("Duos · JackiOh");
    push(a1(3, { pending: flood }, [true, false]));
    expect(document.title).toBe("Your turn · JackiOh");
  });
});
