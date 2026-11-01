// `/duos/<matchId>`'s connection line, as the 1v1 match screen's (`routes/match.test.tsx`), over a fake
// `WebSocket` and a stubbed `fetch` (`test/stubServer.ts`). It is read on the holding panel, before any
// view: the board's chrome draws the same line.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerView } from "@jackioh/shared";
import { SERIES_POLL_SECONDS } from "@jackioh/server-config";

import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { VOIDED_CLOSE_CODE, type SocketLike } from "../game/net.ts";
import { PLAY_LEAN_SET_KEY } from "../game/playLean.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { tokenFromProtocols } from "../net/socketProtocols.ts";
import { baseView, duosTeamView, duosView, pendingFor } from "../test/fixtures.ts";
import { at, must, refusal, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import type { DuosAgainStatus } from "../net/duos.ts";
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

describe("R2092000 Play again on the co-op result", () => {
  const AGAIN = "/api/duos/matches/dm-1/again";
  const FINISHED = duosView(3, { outcome: "win", winner: "b", reason: "concede" });
  const HERE: DuosAgainStatus = { youOffered: false, teammateOffered: false, teammateHere: true, matchId: null };

  function serve(
    status: () => DuosAgainStatus,
    offer: (call: Call) => Reply = () => ({ status: 200, body: { matchId: null } }),
  ): StubServer {
    return stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/catalog") return { status: 200, body: { version: "v1", defs: {} } };
      if (call.method === "GET" && call.path === AGAIN) return { status: 200, body: status() };
      if (call.method === "POST" && call.path === AGAIN) return offer(call);
      return undefined;
    });
  }

  function finished() {
    const route = render(<DuosRoute matchId="dm-1" token="tok-duos" />);
    act(() => {
      latest().readyState = 1;
      latest().onopen?.({});
    });
    act(() => {
      latest().onmessage?.({ data: JSON.stringify({ type: "view", view: FINISHED, legal: [] }) });
    });
    return route;
  }

  async function flush(): Promise<void> {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  }

  beforeEach(() => {
    at("/duos/dm-1");
  });

  afterEach(() => {
    vi.useRealTimers();
    at("/");
  });

  it("R2092000 a live game's result offers Play again, a press sends one offer with the device's lean pick, and the status waits for the teammate", async () => {
    window.localStorage.setItem(PLAY_LEAN_SET_KEY, "Classic");
    let offered = false;
    const server = serve(
      () => ({ ...HERE, youOffered: offered }),
      () => {
        offered = true;
        return { status: 200, body: { matchId: null } };
      },
    );
    finished();
    await flush();

    const play = screen.getByTestId(duosTestid.resultAgain);
    expect(play).toHaveTextContent("Play again");
    fireEvent.click(play);
    expect(play).toBeDisabled();
    expect(play).toHaveAttribute("aria-busy", "true");
    await flush();

    const posts = server.to("POST", AGAIN);
    expect(posts).toHaveLength(1);
    expect(posts[0]?.body).toEqual({ leanSet: "Classic" });
    const line = screen.getByTestId(duosTestid.resultAgainStatus);
    expect(line).toHaveAttribute("role", "status");
    expect(line).toHaveTextContent("Waiting for your teammate…");
    expect(screen.getByTestId(duosTestid.resultAgain)).not.toBeDisabled();
  });

  it("R2092000 with the teammate's offer read, the panel says the teammate wants to play again, and a press makes the offer", async () => {
    const server = serve(() => ({ ...HERE, teammateOffered: true }));
    finished();
    await flush();

    const line = screen.getByTestId(duosTestid.resultAgainStatus);
    expect(line).toHaveTextContent("Your teammate wants to play again");
    expect(screen.getByTestId(duosTestid.resultAgain)).toHaveTextContent("Play again");
    fireEvent.click(screen.getByTestId(duosTestid.resultAgain));
    await flush();
    const posts = server.to("POST", AGAIN);
    expect(posts).toHaveLength(1);
    expect(posts[0]?.body).toEqual({});
  });

  it("R2092000 an offer answered with the new match takes the board to /duos/<id>, whose socket is the new match's, not the old one's", async () => {
    serve(
      () => ({ ...HERE, teammateOffered: true }),
      () => ({ status: 200, body: { matchId: "dm-2" } }),
    );
    const route = finished();
    await flush();

    fireEvent.click(screen.getByTestId(duosTestid.resultAgain));
    await flush();
    expect(window.location.pathname).toBe("/duos/dm-2");

    const old = latest();
    route.rerender(<DuosRoute matchId="dm-2" token="tok-duos" />);
    expect(latest()).not.toBe(old);
    expect(latest().url).toContain("matchId=dm-2");
    expect(old.readyState).toBe(3);
    expect(screen.queryByTestId(duosTestid.result)).toBeNull();
    expect(screen.getByTestId(duosTestid.connecting)).toBeInTheDocument();
  });

  it("R2092000 a status read naming the new match takes the board there too", async () => {
    const server = serve(() => ({ ...HERE, youOffered: true, teammateOffered: true, matchId: "dm-3" }));
    finished();
    await flush();
    expect(window.location.pathname).toBe("/duos/dm-3");
    expect(server.to("POST", AGAIN)).toHaveLength(0);
  });

  it("R2092000 a teammate who has left hides Play again and is named in the status", async () => {
    serve(() => ({ ...HERE, teammateHere: false }));
    finished();
    await flush();
    expect(screen.queryByTestId(duosTestid.resultAgain)).toBeNull();
    expect(screen.getByTestId(duosTestid.resultAgainStatus)).toHaveTextContent("Your teammate has left");
  });

  it("R2092000 a refused offer shows the server's words", async () => {
    const words = "The cards changed since this game. Start a new one.";
    const server = serve(() => ({ ...HERE }), () => refusal(409, "update_required", words));
    finished();
    await flush();

    fireEvent.click(screen.getByTestId(duosTestid.resultAgain));
    await flush();
    const line = screen.getByTestId(duosTestid.resultAgainStatus);
    expect(line).toHaveAttribute("role", "alert");
    expect(line).toHaveTextContent(words);
    expect(screen.getByTestId(duosTestid.resultAgain)).not.toBeDisabled();
    expect(window.location.pathname).toBe("/duos/dm-1");
    expect(server.to("POST", AGAIN)).toHaveLength(1);
  });

  it("R2092000 folding the result to Result and opening it again keeps the waiting status", async () => {
    let offered = false;
    const server = serve(
      () => ({ ...HERE, youOffered: offered }),
      () => {
        offered = true;
        return { status: 200, body: { matchId: null } };
      },
    );
    finished();
    await flush();

    fireEvent.click(screen.getByTestId(duosTestid.resultAgain));
    await flush();
    expect(screen.getByTestId(duosTestid.resultAgainStatus)).toHaveTextContent("Waiting for your teammate…");

    fireEvent.click(screen.getByTestId(duosTestid.resultBoard));
    expect(screen.queryByTestId(duosTestid.result)).toBeNull();

    fireEvent.click(screen.getByTestId(duosTestid.resultReopen));
    expect(screen.getByTestId(duosTestid.resultAgainStatus)).toHaveTextContent("Waiting for your teammate…");
    expect(server.to("POST", AGAIN)).toHaveLength(1);
  });

  it("R2092000 the status is read again every SERIES_POLL_SECONDS while the result is up", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    let wants = false;
    const server = serve(() => ({ ...HERE, teammateOffered: wants }));
    finished();
    await flush();
    expect(server.to("GET", AGAIN)).toHaveLength(1);
    expect(screen.queryByTestId(duosTestid.resultAgainStatus)).toBeNull();

    wants = true;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SERIES_POLL_SECONDS * MS_PER_SECOND + 1);
    });
    await flush();
    expect(server.to("GET", AGAIN)).toHaveLength(2);
    expect(screen.getByTestId(duosTestid.resultAgainStatus)).toHaveTextContent("Your teammate wants to play again");
  });
});
