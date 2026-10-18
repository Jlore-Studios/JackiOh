// #936 S8: `/together`, the Play together page (B54–B56, B59). The page renders what the social
// routes answer and sends intent (CLAUDE.md rule 7): the friends list with each friend's status and
// the controls B56 offers by it, the requests both ways, the blocked list, the add form showing the
// server's refusal in its words, "Shared with me", and the co-op panel.
//
// The server is a stubbed `fetch` (`test/stubServer.ts`); the signed-in session is put where
// `net/session.ts` reads it, as the gate tests do.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { Friend, FriendRequest, PlayerRef, SharedReplay } from "../net/social.ts";
import { must, refusal, stubServer, wholeText, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { TogetherRoute } from "./TogetherRoute.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

/** A stubbed round trip plus a lazy route chunk can outrun the 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const TESTID = {
  addInput: "together-add-input",
  addSubmit: "together-add-submit",
  friendRow: "together-friend-row",
  requestAccept: "together-request-accept",
  requestDecline: "together-request-decline",
  challenge: "together-challenge",
  watch: "together-watch",
  chatOpen: "together-chat-open",
  chatInput: "chat-input",
  coopCreate: "coop-create",
} as const;

// ---------------------------------------------------------------------------------------------
// the world the stubbed server answers with
// ---------------------------------------------------------------------------------------------

const ME: PlayerRef = { profileId: "profile-1", username: "Me#1" };
const SINCE = "2026-10-01T00:00:00.000Z";

/** Online and idle: Challenge, Chat, no Watch. */
const ANN: Friend = { profileId: "f-ann", username: "Ann#2", status: "online", matchId: null, watchable: false, since: SINCE };
/** In a live unranked match that allows spectators: Watch and Chat, no Challenge. */
const BOB: Friend = { profileId: "f-bob", username: "Bob#3", status: "inMatch", matchId: "m-bob", watchable: true, since: SINCE };
/** Offline: Chat only. */
const CY: Friend = { profileId: "f-cy", username: "Cy#4", status: "offline", matchId: null, watchable: false, since: SINCE };
/** In a match that is full or opted out of watching: Chat only. */
const DEE: Friend = { profileId: "f-dee", username: "Dee#5", status: "inMatch", matchId: "m-dee", watchable: false, since: SINCE };

const EVE: PlayerRef = { profileId: "p-eve", username: "Eve#6" };
const FAY: PlayerRef = { profileId: "p-fay", username: "Fay#7" };
const GUS: PlayerRef = { profileId: "p-gus", username: "Gus#8" };

const INCOMING: FriendRequest = { requestId: "rq-in", from: EVE, to: ME, createdAt: "2026-10-14T09:00:00.000Z" };
const OUTGOING: FriendRequest = { requestId: "rq-out", from: ME, to: FAY, createdAt: "2026-10-14T09:01:00.000Z" };

const SHARED: SharedReplay = {
  matchId: "m-shared-1",
  sharer: { profileId: ANN.profileId, username: ANN.username },
  createdAt: "2026-10-14T08:00:00.000Z",
  endedAt: "2026-10-14T07:50:00.000Z",
};

type World = {
  status: "active" | "pending";
  friends: Friend[];
  incoming: FriendRequest[];
  outgoing: FriendRequest[];
  blocked: PlayerRef[];
  shared: SharedReplay[];
};

function world(over: Partial<World> = {}): World {
  return {
    status: "active",
    friends: [ANN, BOB, CY, DEE],
    incoming: [INCOMING],
    outgoing: [OUTGOING],
    blocked: [GUS],
    shared: [SHARED],
    ...over,
  };
}

function meBody(status: World["status"]) {
  return {
    profile: { id: ME.profileId, status },
    needsInviteCode: status === "pending",
    emailVerified: true,
    currentMatchId: null,
    email: "me@example.test",
  };
}

const DECKS = {
  catalogVersion: "v1",
  decks: [{ id: "d-aggro", name: "Aggro", cards: [], catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0 }],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

/** The social and co-op reads every part of the page may make, and a plain answer to each write. */
function serve(state: World, special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    const { method, path } = call;
    if (path === "/api/auth/me") return { status: 200, body: meBody(state.status) };
    if (method === "GET" && path === "/api/friends") {
      return {
        status: 200,
        body: { friends: state.friends, incoming: state.incoming, outgoing: state.outgoing, blocked: state.blocked },
      };
    }
    if (method === "GET" && path === "/api/challenges") return { status: 200, body: { incoming: [], outgoing: [] } };
    if (method === "GET" && path === "/api/replays/shared") return { status: 200, body: { shared: state.shared } };
    if (method === "GET" && path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
    if (method === "GET" && path.startsWith("/api/chat/")) return { status: 200, body: { messages: [] } };
    if (method === "GET" && path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    if (method === "GET" && path === "/api/decks") return { status: 200, body: DECKS };
    if (method === "GET" && path === "/api/catalog") return { status: 200, body: { version: "v1", defs: {} } };
    if (method === "GET" && path === "/api/collection") return { status: 200, body: { catalogVersion: "v1", entries: [] } };
    if (method === "POST" && path === "/api/presence") return { status: 204 };
    if (method === "POST" && path === "/api/friends/requests") return { status: 201, body: { requestId: "rq-new" } };
    if (method === "POST" && /^\/api\/friends\/requests\/[^/]+\/accept$/.test(path)) {
      const friend: Friend = { ...EVE, status: "online", matchId: null, watchable: false, since: SINCE };
      return { status: 200, body: { friend } };
    }
    if (method === "POST" && path.endsWith("/decline")) return { status: 204 };
    if (method === "POST" && path === "/api/blocks") return { status: 204 };
    if (method === "DELETE") return { status: 204 };
    return undefined;
  });
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-together" }));
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

beforeEach(() => {
  window.localStorage.clear();
  resetUsernamePromptForTests();
  at("/together");
  // A confirmation the page may ask before a removal or a block is answered yes.
  const yes = vi.fn(() => true);
  vi.stubGlobal("confirm", yes);
  if (window.confirm !== yes) Object.defineProperty(window, "confirm", { value: yes, configurable: true, writable: true });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

// ---------------------------------------------------------------------------------------------
// reading the page
// ---------------------------------------------------------------------------------------------

async function renderPage(state: World = world(), special?: (call: Call) => Reply | undefined): Promise<StubServer> {
  signedIn();
  const server = serve(state, special);
  render(<TogetherRoute />);
  if (state.friends.length > 0) {
    await waitFor(() => {
      expect(screen.getAllByTestId(TESTID.friendRow)).toHaveLength(state.friends.length);
    }, SLOW);
  } else {
    await screen.findByTestId(TESTID.addInput, undefined, SLOW);
  }
  return server;
}

function rows(): HTMLElement[] {
  return screen.queryAllByTestId(TESTID.friendRow);
}

function rowOf(friend: PlayerRef): HTMLElement {
  return must(
    rows().find((row) => (row.textContent ?? "").includes(friend.username)),
    `the friend row of ${friend.username}`,
  );
}

/** A control the row offers: present, and neither disabled nor aria-disabled. */
function offers(row: HTMLElement, testid: string): boolean {
  const control = within(row).queryByTestId(testid);
  if (control === null) return false;
  if ((control as HTMLButtonElement).disabled === true) return false;
  return control.getAttribute("aria-disabled") !== "true";
}

/** The one button named `name` nearest an element whose own text holds `text`. */
function buttonNear(text: string, name: RegExp): HTMLElement {
  for (const anchor of screen.getAllByText(wholeText(text))) {
    let node: HTMLElement | null = anchor;
    for (let depth = 0; node !== null && depth < 6; depth += 1) {
      const found = within(node).queryAllByRole("button", { name });
      if (found.length === 1) return must(found[0], "the button");
      if (found.length > 1) break;
      node = node.parentElement;
    }
  }
  throw new Error(`no single button ${String(name)} near "${text}"`);
}

/**
 * The control nearest an element whose own text holds `text`, in a container that holds no friend
 * row, request or co-op control (so it is that entry's own control, not another part of the page's).
 */
function controlNear(text: string): HTMLElement {
  const foreign = `[data-testid="${TESTID.friendRow}"], [data-testid^="together-request-"], [data-testid^="coop-"]`;
  for (const anchor of screen.getAllByText(wholeText(text))) {
    let node: HTMLElement | null = anchor;
    for (let depth = 0; node !== null && depth < 6; depth += 1) {
      if (node.matches("button, a[href]")) return node;
      if (node.querySelector(foreign) !== null) break;
      const control = node.querySelector<HTMLElement>("button, a[href]");
      if (control !== null) return control;
      node = node.parentElement;
    }
  }
  throw new Error(`no control beside "${text}"`);
}

/** A link is followed by its href; anything else by clicking it and watching the address bar. */
async function expectLeadsTo(control: HTMLElement, path: string): Promise<void> {
  const link = control.closest("a");
  if (link !== null && link.hasAttribute("href")) {
    expect(new URL(link.href, window.location.origin).pathname).toBe(path);
    return;
  }
  fireEvent.click(control);
  await waitFor(() => {
    expect(window.location.pathname).toBe(path);
  }, SLOW);
}

/** Click, and if the page asks for a confirmation in a dialog of its own, confirm it. */
async function clickConfirmed(control: HTMLElement, done: () => boolean): Promise<void> {
  fireEvent.click(control);
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 50));
  });
  if (done()) return;
  const dialog = screen.queryByRole("alertdialog") ?? screen.queryByRole("dialog");
  if (dialog === null) return;
  const confirm = within(dialog)
    .queryAllByRole("button")
    .find((button) => /remove|block|yes|confirm|ok/i.test(button.textContent ?? ""));
  if (confirm !== undefined) fireEvent.click(confirm);
}

const IN_MATCH = /in[\s_-]?(a\s)?(match|game)|playing/i;

// ---------------------------------------------------------------------------------------------
// B54: the gate, as /play's
// ---------------------------------------------------------------------------------------------

describe("B54 /together is gated as /play is", () => {
  it("B54 anonymous: /together sends the browser to /login", async () => {
    serve(world());
    at("/together");
    render(<App />);
    await waitFor(() => {
      expect(window.location.pathname).toBe("/login");
    }, SLOW);
    expect(screen.queryByTestId(TESTID.addInput)).toBeNull();
  });

  it("B54 a pending account visiting /together lands on /invite, never on the page", async () => {
    signedIn();
    serve(world({ status: "pending" }));
    at("/together");
    render(<App />);
    await waitFor(() => {
      expect(window.location.pathname).toBe("/invite");
    }, SLOW);
    expect(screen.queryByTestId(TESTID.addInput)).toBeNull();
  });

  it("B54 a token the server no longer accepts is the same as having none", async () => {
    signedIn();
    stubServer((call) =>
      call.path === "/api/auth/me" ? refusal(401, "unauthorized", "Sign in first.") : undefined,
    );
    at("/together");
    render(<App />);
    await waitFor(() => {
      expect(window.location.pathname).toBe("/login");
    }, SLOW);
  });

  it("B54 an active account stays on /together and gets the page", async () => {
    signedIn();
    serve(world());
    at("/together");
    render(<App />);
    expect(await screen.findByTestId(TESTID.addInput, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe("/together");
  });
});

// ---------------------------------------------------------------------------------------------
// B55: friends, requests, blocked, the add form
// ---------------------------------------------------------------------------------------------

describe("B55 the friends list", () => {
  it("B55 lists one row per friend, each with the friend's name#tag", async () => {
    await renderPage();
    const names = rows().map((row) => row.textContent ?? "");
    for (const friend of [ANN, BOB, CY, DEE]) {
      expect(names.some((text) => text.includes(friend.username)), friend.username).toBe(true);
    }
  });

  it("B55 each row shows the friend's status", async () => {
    await renderPage();
    expect(rowOf(ANN).outerHTML).toMatch(/online/i);
    expect(rowOf(CY).outerHTML).toMatch(/offline/i);
    expect(rowOf(BOB).outerHTML).toMatch(IN_MATCH);
    expect(rowOf(DEE).outerHTML).toMatch(IN_MATCH);
  });

  it("B55 a blocked player and a pending request are not friend rows", async () => {
    await renderPage();
    for (const other of [EVE, FAY, GUS]) {
      expect(rows().some((row) => (row.textContent ?? "").includes(other.username)), other.username).toBe(false);
    }
  });

  it("B55 a friends list the server refuses to read draws no friend row and no request control", async () => {
    signedIn();
    const server = serve(world(), (call) =>
      call.method === "GET" && call.path === "/api/friends" ? refusal(500, "internal", "Something went wrong.") : undefined,
    );
    render(<TogetherRoute />);
    await waitFor(() => {
      expect(server.to("GET", "/api/friends").length).toBeGreaterThan(0);
    }, SLOW);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });
    expect(rows()).toHaveLength(0);
    expect(screen.queryAllByTestId(TESTID.requestAccept)).toHaveLength(0);
    expect(screen.queryAllByTestId(TESTID.challenge)).toHaveLength(0);
  });

  it("B55 with no friends the page shows no friend row and still offers the add form", async () => {
    await renderPage(world({ friends: [] }));
    expect(rows()).toHaveLength(0);
    expect(screen.getByTestId(TESTID.addInput)).toBeInTheDocument();
    expect(screen.getByTestId(TESTID.addSubmit)).toBeInTheDocument();
  });
});

describe("B55 the add form", () => {
  it("B55 sends the typed name#tag as POST /api/friends/requests {username}", async () => {
    const server = await renderPage();
    fireEvent.change(screen.getByTestId(TESTID.addInput), { target: { value: "Hal#9" } });
    fireEvent.click(screen.getByTestId(TESTID.addSubmit));
    await waitFor(() => {
      expect(server.to("POST", "/api/friends/requests")).toHaveLength(1);
    }, SLOW);
    expect(server.to("POST", "/api/friends/requests")[0]?.body).toEqual({ username: "Hal#9" });
  });

  it("B55 an unknown name shows the server's refusal in its own words", async () => {
    const message = "Nobody goes by that name here.";
    await renderPage(world(), (call) =>
      call.method === "POST" && call.path === "/api/friends/requests" ? refusal(404, "not_found", message) : undefined,
    );
    fireEvent.change(screen.getByTestId(TESTID.addInput), { target: { value: "Nobody#1" } });
    fireEvent.click(screen.getByTestId(TESTID.addSubmit));
    await waitFor(() => {
      expect(document.body).toHaveTextContent(message);
    }, SLOW);
  });

  it.each([
    ["self", "That’s you."],
    ["already_friends", "You are already friends."],
    ["already_requested", "You already asked them."],
    ["friend_cap", "Your friends list is full."],
  ])("B55 a 409 %s refusal is shown in the server's words", async (code, message) => {
    await renderPage(world(), (call) =>
      call.method === "POST" && call.path === "/api/friends/requests" ? refusal(409, code, message) : undefined,
    );
    fireEvent.change(screen.getByTestId(TESTID.addInput), { target: { value: "Ann#2" } });
    fireEvent.click(screen.getByTestId(TESTID.addSubmit));
    await waitFor(() => {
      expect(document.body).toHaveTextContent(message);
    }, SLOW);
  });

  it("B55 a rate-limited request (429) is shown in the server's words", async () => {
    const message = "Too many friend requests. Try again later.";
    await renderPage(world(), (call) =>
      call.method === "POST" && call.path === "/api/friends/requests" ? refusal(429, "rate_limited", message) : undefined,
    );
    fireEvent.change(screen.getByTestId(TESTID.addInput), { target: { value: "Hal#9" } });
    fireEvent.click(screen.getByTestId(TESTID.addSubmit));
    await waitFor(() => {
      expect(document.body).toHaveTextContent(message);
    }, SLOW);
  });

  it("B55 a refused request adds no friend row", async () => {
    await renderPage(world(), (call) =>
      call.method === "POST" && call.path === "/api/friends/requests" ? refusal(404, "not_found", "No such player.") : undefined,
    );
    fireEvent.change(screen.getByTestId(TESTID.addInput), { target: { value: "Nobody#1" } });
    fireEvent.click(screen.getByTestId(TESTID.addSubmit));
    await waitFor(() => {
      expect(document.body).toHaveTextContent("No such player.");
    }, SLOW);
    expect(rows()).toHaveLength(4);
    expect(rows().some((row) => (row.textContent ?? "").includes("Nobody#1"))).toBe(false);
  });
});

describe("B55 requests both ways and the blocked list", () => {
  it("B55 an incoming request offers Accept, which posts to its accept route", async () => {
    const server = await renderPage();
    const accepts = screen.getAllByTestId(TESTID.requestAccept);
    expect(accepts).toHaveLength(1);
    fireEvent.click(must(accepts[0], "accept"));
    await waitFor(() => {
      expect(server.to("POST", "/api/friends/requests/rq-in/accept")).toHaveLength(1);
    }, SLOW);
  });

  it("B55 an incoming request offers Decline, which posts to its decline route", async () => {
    const server = await renderPage();
    const declines = screen.getAllByTestId(TESTID.requestDecline);
    expect(declines).toHaveLength(1);
    fireEvent.click(must(declines[0], "decline"));
    await waitFor(() => {
      expect(server.to("POST", "/api/friends/requests/rq-in/decline")).toHaveLength(1);
    }, SLOW);
    expect(server.to("POST", "/api/friends/requests/rq-in/accept")).toHaveLength(0);
  });

  it("B55 an outgoing request offers no Accept or Decline: only the incoming one does", async () => {
    await renderPage(world({ incoming: [] }));
    expect(screen.queryAllByTestId(TESTID.requestAccept)).toHaveLength(0);
    expect(screen.queryAllByTestId(TESTID.requestDecline)).toHaveLength(0);
    expect(document.body).toHaveTextContent(FAY.username);
  });

  it("B55 an outgoing request offers Cancel, which deletes the request", async () => {
    const server = await renderPage();
    fireEvent.click(buttonNear(FAY.username, /^cancel/i));
    await waitFor(() => {
      expect(server.to("DELETE", "/api/friends/requests/rq-out")).toHaveLength(1);
    }, SLOW);
  });

  it("B55 a blocked player is listed with Unblock, which deletes the block", async () => {
    const server = await renderPage();
    fireEvent.click(buttonNear(GUS.username, /^unblock/i));
    await waitFor(() => {
      expect(server.to("DELETE", "/api/blocks/p-gus")).toHaveLength(1);
    }, SLOW);
  });

  it("B55 a blocked player is offered no Challenge, Watch or Chat", async () => {
    await renderPage(world({ friends: [ANN] }));
    for (const testid of [TESTID.challenge, TESTID.watch, TESTID.chatOpen]) {
      const offered = screen.queryAllByTestId(testid).filter((control) => !rowOf(ANN).contains(control));
      expect(offered, testid).toHaveLength(0);
    }
  });
});

// ---------------------------------------------------------------------------------------------
// B56: what each friend row offers
// ---------------------------------------------------------------------------------------------

describe("B56 each friend row's controls", () => {
  it("B56 an online, idle friend is offered Challenge", async () => {
    await renderPage();
    expect(offers(rowOf(ANN), TESTID.challenge)).toBe(true);
  });

  it("B56 an offline friend is not offered Challenge", async () => {
    await renderPage();
    expect(offers(rowOf(CY), TESTID.challenge)).toBe(false);
  });

  it("B56 a friend in a match is not offered Challenge", async () => {
    await renderPage();
    expect(offers(rowOf(BOB), TESTID.challenge)).toBe(false);
    expect(offers(rowOf(DEE), TESTID.challenge)).toBe(false);
  });

  it("B56 a watchable friend is offered Watch", async () => {
    await renderPage();
    expect(offers(rowOf(BOB), TESTID.watch)).toBe(true);
  });

  it("B56 a friend who is not watchable is not offered Watch: idle, offline, or in a full or opted-out match", async () => {
    await renderPage();
    expect(offers(rowOf(ANN), TESTID.watch)).toBe(false);
    expect(offers(rowOf(CY), TESTID.watch)).toBe(false);
    expect(offers(rowOf(DEE), TESTID.watch)).toBe(false);
  });

  it("B56 Watch goes to /watch/<the friend's match>", async () => {
    await renderPage();
    await expectLeadsTo(must(within(rowOf(BOB)).queryByTestId(TESTID.watch), "watch"), "/watch/m-bob");
  });

  it("B56 every friend is offered Chat, whatever their status", async () => {
    await renderPage();
    for (const friend of [ANN, BOB, CY, DEE]) {
      expect(offers(rowOf(friend), TESTID.chatOpen), friend.username).toBe(true);
    }
  });

  it("B56 Chat opens the conversation with that friend", async () => {
    const server = await renderPage();
    expect(screen.queryByTestId(TESTID.chatInput)).toBeNull();
    fireEvent.click(must(within(rowOf(CY)).queryByTestId(TESTID.chatOpen), "chat"));
    expect(await screen.findByTestId(TESTID.chatInput, undefined, SLOW)).toBeInTheDocument();
    await waitFor(() => {
      expect(server.to("GET", "/api/chat/f-cy").length).toBeGreaterThan(0);
    }, SLOW);
    expect(server.to("GET", "/api/chat/f-ann")).toHaveLength(0);
  });

  it("B56 every friend is offered Remove, which deletes the friendship", async () => {
    const server = await renderPage();
    const remove = within(rowOf(ANN)).getByRole("button", { name: /^remove/i });
    await clickConfirmed(remove, () => server.to("DELETE", "/api/friends/f-ann").length > 0);
    await waitFor(() => {
      expect(server.to("DELETE", "/api/friends/f-ann")).toHaveLength(1);
    }, SLOW);
    expect(server.to("DELETE", "/api/friends/f-bob")).toHaveLength(0);
  });

  it("B56 every friend is offered Block, which posts the block", async () => {
    const server = await renderPage();
    const block = within(rowOf(CY)).getByRole("button", { name: /^block/i });
    await clickConfirmed(block, () => server.to("POST", "/api/blocks").length > 0);
    await waitFor(() => {
      expect(server.to("POST", "/api/blocks")).toHaveLength(1);
    }, SLOW);
    expect(server.to("POST", "/api/blocks")[0]?.body).toEqual({ profileId: "f-cy" });
  });

  it("B56 a friend in a match (busy) or offline sends no challenge from its row", async () => {
    const server = await renderPage();
    for (const friend of [BOB, CY, DEE]) {
      const control = within(rowOf(friend)).queryByTestId(TESTID.challenge);
      if (control !== null) fireEvent.click(control);
    }
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });
    expect(server.to("POST", "/api/challenges")).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------------------------
// B59: Shared with me
// ---------------------------------------------------------------------------------------------

describe("B59 Shared with me", () => {
  // Ann shares a replay but is no friend row here, so her name on the page is the shared entry's.
  const ONLY_CY: Partial<World> = { friends: [CY] };

  it("B59 lists the replays friends shared under Shared with me, each naming its sharer", async () => {
    await renderPage(world(ONLY_CY));
    expect(await screen.findByText(/shared with me/i, undefined, SLOW)).toBeInTheDocument();
    expect((await screen.findAllByText(wholeText("Ann#2"), undefined, SLOW)).length).toBeGreaterThan(0);
  });

  it("B59 reads the list from GET /api/replays/shared", async () => {
    const server = await renderPage(world(ONLY_CY));
    await waitFor(() => {
      expect(server.to("GET", "/api/replays/shared").length).toBeGreaterThan(0);
    }, SLOW);
  });

  it("B59 opening a shared replay asks for that match's replay (or goes to it)", async () => {
    const server = await renderPage(world(ONLY_CY));
    await screen.findAllByText(wholeText("Ann#2"), undefined, SLOW);
    const open = controlNear(SHARED.sharer.username);
    const href = open.closest("a")?.getAttribute("href") ?? null;
    if (href !== null && href.includes(SHARED.matchId)) return;
    fireEvent.click(open);
    await waitFor(() => {
      const asked = server.calls.some((call) => call.method === "GET" && call.path === `/api/replays/${SHARED.matchId}`);
      const went = `${window.location.pathname}${window.location.search}`.includes(SHARED.matchId);
      expect(asked || went).toBe(true);
    }, SLOW);
  });

  it("B59 with nothing shared, no shared replay is listed and none is asked for", async () => {
    const server = await renderPage(world({ ...ONLY_CY, shared: [] }));
    await waitFor(() => {
      expect(server.to("GET", "/api/replays/shared").length).toBeGreaterThan(0);
    }, SLOW);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });
    expect(screen.queryAllByText(wholeText("Ann#2"))).toHaveLength(0);
    expect(server.calls.some((call) => call.path.startsWith("/api/replays/m-"))).toBe(false);
  });
});

// ---------------------------------------------------------------------------------------------
// the co-op panel lives on the page
// ---------------------------------------------------------------------------------------------

describe("B60 the page holds the co-op panel", () => {
  it("B60 /together shows the co-op panel's create control", async () => {
    await renderPage();
    expect(await screen.findByTestId(TESTID.coopCreate, undefined, SLOW)).toBeInTheDocument();
  });
});
