// The private lobby screen (R3730000 to R3730006): its controls, Ready, the countdown, its reads again,
// closed and removed, and the classic room's join card, against a stubbed server.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LOBBY_START_COUNTDOWN_SECONDS } from "@jackioh/server-config";

import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import { fakeSockets } from "../test/fakeSocket.ts";
import { settle } from "../test/settle.ts";
import { at, must, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { setSocialSocketFactory } from "../together/socialSocket.ts";
import type { LobbyRead, LobbySeat, LobbyView } from "./api.ts";
import { LOBBY_POLL_MS } from "./config.ts";
import { COOP_DECK_KEY } from "./coopDeck.ts";
import { LobbyScreen } from "./LobbyScreen.tsx";
import { lobbyTestid } from "./testids.ts";

const SLOW = { timeout: 4000 };
const CODE = "K7Q2XM";

const ANA = { profileId: "p-ana", username: "Ana#1", status: "online" as const };
const BO = { profileId: "p-bo", username: "Bo#2", status: "online" as const };

const DECKS = {
  catalogVersion: "v1",
  decks: [{ id: "d-pyre", name: "Pyre Control", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 }],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

function seat(over: Partial<LobbySeat> = {}): LobbySeat {
  return { role: "host", player: ANA, state: "choosing", deck: null, random: false, ...over };
}

function lobby(over: Partial<LobbyView> = {}): LobbyView {
  return {
    kind: "lobby",
    code: CODE,
    settings: { format: "duel", mode: "random" },
    expiresAt: Date.now() + 900_000,
    you: "host",
    yourPick: null,
    seats: [seat(), seat({ role: "guest", player: BO })],
    startedAt: null,
    matchId: null,
    seriesId: null,
    ...over,
  };
}

/** A server whose lobby reads answer `reads` in turn (the last one again), and every write 204. */
function serve(reads: readonly (LobbyRead | Reply)[], special?: (call: Call) => Reply | undefined): StubServer {
  let read = 0;
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    const { method, path } = call;
    if (method === "GET" && path === `/api/lobbies/${CODE}`) {
      const next = reads[Math.min(read, reads.length - 1)];
      read += 1;
      if (next !== undefined && "status" in next) return next;
      return { status: 200, body: next };
    }
    if (method === "POST" && path.startsWith(`/api/lobbies/${CODE}/`)) return { status: 204 };
    if (method === "GET" && path === "/api/decks") return { status: 200, body: DECKS };
    if (method === "GET" && path === "/api/social") return { status: 200, body: { incoming: [], outgoing: [], unread: {} } };
    if (method === "GET" && path === "/api/friends") {
      return { status: 200, body: { friends: [], incoming: [], outgoing: [], blocked: [] } };
    }
    return undefined;
  });
}

function show(): void {
  render(<LobbyScreen token="tok-lobby" code={CODE} />);
}

beforeEach(() => {
  signIn("tok-lobby");
  at(`/lobby/${CODE}`);
});

afterEach(() => {
  cleanup();
  resetLocaleForTests();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  window.sessionStorage.clear();
  window.localStorage.removeItem(COOP_DECK_KEY);
});

describe("the lobby screen", () => {
  it("R3730003 the host's settings are a control, and a guest reads them as text", async () => {
    const hosting = serve([lobby()]);
    show();
    const control = await screen.findByTestId(lobbyTestid.settings, undefined, SLOW);
    expect(control.tagName).toBe("SELECT");
    fireEvent.change(control, { target: { value: "bo1" } });
    await waitFor(() => {
      expect(hosting.to("POST", `/api/lobbies/${CODE}/settings`)[0]?.body).toEqual({ mode: "bo1" });
    }, SLOW);
    cleanup();

    const server = serve([lobby({ you: "guest" })]);
    show();
    const text = await screen.findByTestId(lobbyTestid.settings, undefined, SLOW);
    expect(text.tagName).toBe("OUTPUT");
    expect(text).toHaveTextContent("All random");
    expect(screen.queryByTestId(lobbyTestid.start)).toBeNull();
    expect(server.to("POST", `/api/lobbies/${CODE}/settings`)).toHaveLength(0);
  });

  it("R3730001 Ready sends the pick shown first when the server holds none, then Ready", async () => {
    const server = serve([lobby()]);
    show();
    fireEvent.click(await screen.findByTestId(lobbyTestid.ready, undefined, SLOW));
    await waitFor(() => {
      expect(server.to("POST", `/api/lobbies/${CODE}/ready`)).toHaveLength(1);
    }, SLOW);
    const picks = server.to("POST", `/api/lobbies/${CODE}/pick`);
    expect(picks).toHaveLength(1);
    expect(picks[0]?.body).toMatchObject({ mode: "random" });
    expect(server.to("POST", `/api/lobbies/${CODE}/ready`)[0]?.body).toEqual({ ready: true });
    const order = server.calls.filter((call) => call.method === "POST").map((call) => call.path);
    expect(order).toEqual([`/api/lobbies/${CODE}/pick`, `/api/lobbies/${CODE}/ready`]);
  });

  it("R2031000 R3730001 a random deck offers More cards from beside it, and the pick sent carries it", async () => {
    const server = serve([lobby({ settings: { format: "coop", difficulty: "easy" }, yourPick: { mode: "random" } })]);
    show();
    const lean = await screen.findByTestId(lobbyTestid.lean, undefined, SLOW);
    const pick = [...(lean as HTMLSelectElement).options].map((option) => option.value).find((value) => value === "newest");
    expect(pick).toBe("newest");
    fireEvent.change(lean, { target: { value: "newest" } });
    await waitFor(() => {
      expect(server.to("POST", `/api/lobbies/${CODE}/pick`)[0]?.body).toEqual({ random: true, leanSet: "newest" });
    }, SLOW);
  });

  it("R767 R3730006 Copy link shares the lobby's own link", async () => {
    serve([lobby()]);
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    show();
    fireEvent.click(await screen.findByTestId(lobbyTestid.copyLink, undefined, SLOW));
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith(`${window.location.origin}/lobby/${CODE}`);
    }, SLOW);
    Reflect.deleteProperty(navigator, "clipboard");
  });

  it("R3730002 the Start lights only when every seat is ready, and a started lobby counts down to its board", async () => {
    serve([lobby({ seats: [seat({ state: "ready" }), seat({ role: "guest", player: BO, state: "chosen" })] })]);
    show();
    expect(await screen.findByTestId(lobbyTestid.start, undefined, SLOW)).toBeDisabled();
    cleanup();

    serve([lobby({ seats: [seat({ state: "ready" }), seat({ role: "guest", player: BO, state: "ready" })] })]);
    show();
    await waitFor(() => {
      expect(screen.getByTestId(lobbyTestid.start)).toBeEnabled();
    }, SLOW);
    cleanup();

    vi.useFakeTimers({ shouldAdvanceTime: true });
    const started = lobby({ you: "guest", startedAt: Date.now(), matchId: "m-1", seats: [seat({ state: "ready" }), seat({ role: "guest", player: BO, state: "ready" })] });
    serve([started]);
    show();
    const countdown = await screen.findByTestId(lobbyTestid.countdown, undefined, SLOW);
    expect(countdown).toHaveTextContent(String(LOBBY_START_COUNTDOWN_SECONDS));
    expect(window.location.pathname).toBe(`/lobby/${CODE}`);
    const entries = window.history.length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(LOBBY_START_COUNTDOWN_SECONDS * 1000 + 500);
    });
    expect(window.location.pathname).toBe("/match/m-1");
    // The board replaces the lobby's entry, so Back does not land on a lobby that counts down again.
    expect(window.history.length).toBe(entries);
  });

  it("R3730000 the lobby reads itself again while the social socket is down", async () => {
    const server = serve([lobby({ seats: [seat(), seat({ role: "guest", player: null })] }), lobby()]);
    show();
    expect(await screen.findByTestId(lobbyTestid.invite, undefined, SLOW)).toBeInTheDocument();
    await waitFor(
      () => {
        expect(screen.queryByTestId(lobbyTestid.invite)).toBeNull();
      },
      { timeout: LOBBY_POLL_MS * 2 },
    );
    expect(server.to("GET", `/api/lobbies/${CODE}`).length).toBeGreaterThanOrEqual(2);
    const seats = screen.getAllByTestId(lobbyTestid.seat);
    expect(within(seats[1] ?? document.body).getByText("Bo#2")).toBeInTheDocument();
  }, LOBBY_POLL_MS * 4);

  it("R3730000 the lobby reads once when the social socket comes up, for what changed before it held one", async () => {
    const fake = fakeSockets();
    setSocialSocketFactory(fake.factory);
    try {
      vi.useFakeTimers();
      const server = serve([lobby({ seats: [seat(), seat({ role: "guest", player: null })] }), lobby()]);
      const reads = (): number => server.to("GET", `/api/lobbies/${CODE}`).length;
      show();
      await settle({ advance: 0 });
      expect(screen.getByTestId(lobbyTestid.invite)).toBeInTheDocument();
      expect(reads()).toBe(1);
      // The guest entered while the tab held no socket: the server pushed it to nobody.
      const socket = must(fake.sockets.at(-1), "the tab's social socket");
      socket.open();
      socket.deliver({ type: "social", incoming: [], outgoing: [], unread: {} });
      await settle({ advance: 0 });
      expect(reads(), "the socket came up: one read, with no push and no poll").toBe(2);
      expect(screen.queryByTestId(lobbyTestid.invite)).toBeNull();
      const seats = screen.getAllByTestId(lobbyTestid.seat);
      expect(within(seats[1] ?? document.body).getByText("Bo#2")).toBeInTheDocument();
      await settle({ advance: LOBBY_POLL_MS * 2 });
      expect(reads(), "live, the lobby sends no poll").toBe(2);
    } finally {
      setSocialSocketFactory(null);
    }
  });

  it("R3205000 R3730002 the lobby's own reads keep running while the tab is hidden", async () => {
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
    try {
      const server = serve([lobby()]);
      show();
      expect(await screen.findByTestId(lobbyTestid.leave, undefined, SLOW)).toBeInTheDocument();
      await waitFor(
        () => {
          expect(server.to("GET", `/api/lobbies/${CODE}`).length).toBeGreaterThanOrEqual(2);
        },
        { timeout: LOBBY_POLL_MS * 2 },
      );
    } finally {
      Reflect.deleteProperty(document, "visibilityState");
    }
  }, LOBBY_POLL_MS * 4);

  it("R3730004 a closed lobby says so, and a member who reads it as a visitor was removed", async () => {
    serve([{ status: 404, body: { error: { code: "not_found", message: "that lobby is not open" } } }]);
    show();
    expect(await screen.findByTestId(lobbyTestid.closed, undefined, SLOW)).toHaveTextContent("This lobby is closed.");
    cleanup();

    serve([lobby({ you: "guest" }), lobby({ you: null, seats: [seat(), seat({ role: "guest", player: null })] })]);
    show();
    fireEvent.click(await screen.findByTestId(lobbyTestid.leave, undefined, SLOW));
    expect(await screen.findByTestId(lobbyTestid.removed, undefined, SLOW)).toHaveTextContent("You were removed from this lobby.");
    expect(screen.getByTestId(lobbyTestid.enter)).toBeInTheDocument();
  });

  it("R3730006 with no lobby under the code, the classic room joins at once", async () => {
    const server = serve([{ kind: "room", code: CODE, mode: "random", host: { profileId: "p-ana", username: "Ana#1" } }], (call) =>
      call.method === "POST" && call.path === `/api/rooms/${CODE}/join`
        ? { status: 200, body: { matchId: "m-7", seriesId: null, code: CODE, seat: "p2", mode: "random" } }
        : undefined,
    );
    show();
    const card = await screen.findByTestId(lobbyTestid.joinCard, undefined, SLOW);
    expect(card).toHaveTextContent("Ana#1");
    fireEvent.click(within(card).getByTestId(lobbyTestid.joinRoom));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/match/m-7");
    }, SLOW);
    expect(server.to("POST", `/api/rooms/${CODE}/join`)[0]?.body).toMatchObject({ mode: "random" });
  });

  it("R2094000 R3730006 a co-op lobby starts on the device's last co-op pick, and a pick made is remembered", async () => {
    window.localStorage.setItem(COOP_DECK_KEY, "random");
    const server = serve([lobby({ settings: { format: "coop", difficulty: "easy" } })]);
    show();
    const picker = await screen.findByTestId<HTMLSelectElement>(lobbyTestid.deckSelect, undefined, SLOW);
    await waitFor(() => {
      expect(picker).toBeEnabled();
    }, SLOW);
    expect(picker.value).toBe("random");
    fireEvent.change(picker, { target: { value: "d-pyre" } });
    await waitFor(() => {
      expect(server.to("POST", `/api/lobbies/${CODE}/pick`)[0]?.body).toEqual({ deckId: "d-pyre" });
    }, SLOW);
    expect(window.localStorage.getItem(COOP_DECK_KEY)).toBe("d-pyre");
    cleanup();

    // A remembered deck the account no longer has falls back to the first saved deck.
    window.localStorage.setItem(COOP_DECK_KEY, "d-gone");
    serve([lobby({ settings: { format: "coop", difficulty: "easy" } })]);
    show();
    await waitFor(() => {
      expect(screen.getByTestId<HTMLSelectElement>(lobbyTestid.deckSelect).value).toBe("d-pyre");
    }, SLOW);
  });

  it("R2094000 R3730006 a classic co-op room's join card starts on the remembered deck, and remembers the pick it joins with", async () => {
    window.localStorage.setItem(COOP_DECK_KEY, "d-pyre");
    const server = serve([{ kind: "coopRoom", code: CODE, difficulty: "easy", host: { profileId: "p-ana", username: "Ana#1" } }], (call) =>
      call.method === "POST" && call.path === `/api/duos/rooms/${CODE}/join` ? { status: 200, body: { matchId: "m-9" } } : undefined,
    );
    show();
    const card = await screen.findByTestId(lobbyTestid.joinCard, undefined, SLOW);
    const picker = within(card).getByTestId<HTMLSelectElement>(lobbyTestid.deckSelect);
    await waitFor(() => {
      expect(picker.value).toBe("d-pyre");
    }, SLOW);
    fireEvent.change(picker, { target: { value: "random" } });
    expect(picker.value).toBe("random");
    fireEvent.click(within(card).getByTestId(lobbyTestid.joinRoom));
    await waitFor(() => {
      expect(window.location.pathname).toBe("/duos/m-9");
    }, SLOW);
    expect(server.to("POST", `/api/duos/rooms/${CODE}/join`)[0]?.body).toMatchObject({ random: true });
    expect(window.localStorage.getItem(COOP_DECK_KEY)).toBe("random");
  });

  it("R3730000 a seat's presence is in the player's words, as the friends list says it", async () => {
    await setLocale("zh-CN");
    serve([lobby({ seats: [seat(), seat({ role: "guest", player: { ...BO, status: "inMatch" } })] })]);
    show();
    const seats = await screen.findAllByTestId(lobbyTestid.seat, undefined, SLOW);
    expect(seats[0]).toHaveTextContent("在线");
    expect(seats[1]).toHaveTextContent("对局中");
  });
});

describe("#1568 decks that could not be read", () => {
  const FAILED = "Your decks could not be loaded: The decks are out of reach.";

  /** A server whose first read of the decks is refused, counting the reads. */
  function serveFailingDecks(reads: readonly (LobbyRead | Reply)[], count: { decks: number }, special?: (call: Call) => Reply | undefined): StubServer {
    return serve(reads, (call) => {
      if (call.method === "GET" && call.path === "/api/decks") {
        count.decks += 1;
        if (count.decks === 1) return refusal(500, "internal", "The decks are out of reach.");
      }
      return special?.(call);
    });
  }

  it("#1568 the lobby's deck says why, with Try again, which reads the decks again and offers them", async () => {
    const count = { decks: 0 };
    serveFailingDecks([lobby({ settings: { format: "duel", mode: "bo1" } })], count);
    show();
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(FAILED);
    expect(screen.getByTestId(lobbyTestid.decksRetry)).toHaveAccessibleDescription(FAILED);
    fireEvent.click(screen.getByTestId(lobbyTestid.decksRetry));
    await waitFor(() => {
      const picker = screen.getByTestId<HTMLSelectElement>(lobbyTestid.deckSelect);
      expect([...picker.options].map((option) => option.textContent)).toEqual(["Pyre Control"]);
    }, SLOW);
    expect(count.decks).toBe(2);
    expect(screen.queryByTestId(lobbyTestid.decksRetry)).toBeNull();
  });

  it("#1568 a classic room's join card says why too, and Try again offers the decks", async () => {
    const count = { decks: 0 };
    serveFailingDecks([{ kind: "room", code: CODE, mode: "bo1", host: { profileId: "p-ana", username: "Ana#1" } }], count);
    show();
    expect(await screen.findByRole("alert", undefined, SLOW)).toHaveTextContent(FAILED);
    fireEvent.click(screen.getByTestId(lobbyTestid.decksRetry));
    await waitFor(() => {
      const picker = screen.getByTestId<HTMLSelectElement>(lobbyTestid.deckSelect);
      expect([...picker.options].map((option) => option.textContent)).toEqual(["Pyre Control"]);
    }, SLOW);
    expect(count.decks).toBe(2);
  });

  it("#1568 an All Random lobby needs no deck, so a failed read asks nothing of it", async () => {
    const count = { decks: 0 };
    serveFailingDecks([lobby()], count);
    show();
    expect(await screen.findByTestId(lobbyTestid.ready, undefined, SLOW)).toBeEnabled();
    await waitFor(() => {
      expect(count.decks).toBe(1);
    }, SLOW);
    expect(screen.queryByTestId(lobbyTestid.decksRetry)).toBeNull();
  });
});
