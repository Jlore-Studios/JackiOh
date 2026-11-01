// #936 S8: `/together`, the Play together page (B54–B56, B59). The page renders what the social
// routes answer and sends intent (CLAUDE.md rule 7). The server is a stubbed `fetch`
// (`test/stubServer.ts`); the signed-in session is put where `net/session.ts` reads it.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { manualClock } from "../game/deckbuilder/testkit.ts";
import type { Friend, FriendRequest, PlayerRef, SharedReplay } from "../net/social.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { settle } from "../test/settle.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { at, meBody, must, refusal, signIn, stubServer, wholeText, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { UNDO_WINDOW_MS, type UndoTimer } from "../undo/hold.ts";
import { UNDO_TOAST, UNDO_TOAST_UNDO } from "../undo/UndoToast.tsx";
import { FRIENDS_POLL_MS } from "./config.ts";
import { stopFollowingChallenge } from "./follow.ts";
import { togetherTestid } from "./testids.ts";
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
  challengeForm: "together-challenge-form",
  challengeModeBo1: "together-challenge-mode-bo1",
  challengeDeck: "together-challenge-deck",
  challengeSend: "together-challenge-send",
  challengeClose: "together-challenge-close",
  watch: "together-watch",
  chatOpen: "together-chat-open",
  chatInput: "chat-input",
  coopCreate: "coop-create",
  sharedRetry: "together-shared-retry",
  sharedRow: "together-shared-row",
  friendsError: "together-friends-error",
  friendsRetry: "together-friends-retry",
} as const;

// the world the stubbed server answers with

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
    if (path === "/api/auth/me") {
      const profile = { id: ME.profileId, status: state.status };
      return { status: 200, body: meBody({ profile, needsInviteCode: state.status === "pending", email: "me@example.test" }) };
    }
    if (method === "GET" && path === "/api/friends") {
      return {
        status: 200,
        body: { friends: state.friends, incoming: state.incoming, outgoing: state.outgoing, blocked: state.blocked },
      };
    }
    if (method === "GET" && path === "/api/challenges") return { status: 200, body: { incoming: [], outgoing: [] } };
    if (method === "GET" && path === "/api/replays/shared") return { status: 200, body: { shared: state.shared } };
    if (method === "GET" && path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
    if (method === "GET" && path.startsWith("/api/chat/")) return { status: 200, body: { messages: [], nextCursor: null } };
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
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

// reading the page

async function renderPage(
  state: World = world(),
  special?: (call: Call) => Reply | undefined,
  undoTimer?: UndoTimer,
): Promise<StubServer> {
  signIn("tok-together");
  const server = serve(state, special);
  render(<TogetherRoute {...(undoTimer === undefined ? {} : { undoTimer })} />);
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

/** The incoming request's row (`INCOMING`, from Eve). */
function incomingRequestRow(): HTMLElement {
  return must(
    screen.queryAllByTestId(togetherTestid.requestRow).find((row) => row.getAttribute("data-direction") === "incoming"),
    "the incoming request's row",
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
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  if (done()) return;
  const dialog = screen.queryByRole("alertdialog") ?? screen.queryByRole("dialog");
  if (dialog === null) return;
  const confirm = within(dialog)
    .queryAllByRole("button")
    .find((button) => /remove|block|yes|confirm|ok/i.test(button.textContent ?? ""));
  if (confirm !== undefined) fireEvent.click(confirm);
}

/** A key pressed wherever the focus is: the challenge dialog reads Escape and Tab at the window (ConfirmConcede.test.tsx). */
function press(key: string, options: { shiftKey?: boolean } = {}): void {
  fireEvent.keyDown(document.activeElement ?? document.body, { key, ...options });
}

const IN_MATCH = /in[\s_-]?(a\s)?(match|game)|playing/i;

// B54: the gate, as /play's

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
    signIn("tok-together");
    serve(world({ status: "pending" }));
    at("/together");
    render(<App />);
    await waitFor(() => {
      expect(window.location.pathname).toBe("/invite");
    }, SLOW);
    expect(screen.queryByTestId(TESTID.addInput)).toBeNull();
  });

  it("B54 a token the server no longer accepts is the same as having none", async () => {
    signIn("tok-together");
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
    signIn("tok-together");
    serve(world());
    at("/together");
    render(<App />);
    expect(await screen.findByTestId(TESTID.addInput, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe("/together");
  });
});

// B55: friends, requests, blocked, the add form

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
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    signIn("tok-together");
    const server = serve(world(), (call) =>
      call.method === "GET" && call.path === "/api/friends" ? refusal(500, "internal", "Something went wrong.") : undefined,
    );
    render(<TogetherRoute />);
    await waitFor(() => {
      expect(server.to("GET", "/api/friends").length).toBeGreaterThan(0);
    }, SLOW);
    await settle({ advance: FRIENDS_POLL_MS + 1 });
    expect(server.to("GET", "/api/friends")).toHaveLength(2);
    expect(rows()).toHaveLength(0);
    expect(screen.queryAllByTestId(TESTID.requestAccept)).toHaveLength(0);
    expect(screen.queryAllByTestId(TESTID.challenge)).toHaveLength(0);
  });

  it("#1691 a friends list that could not be read offers Try again, which lists the friends", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    signIn("tok-together");
    let refused = true;
    const server = serve(world(), (call) =>
      refused && call.method === "GET" && call.path === "/api/friends" ? refusal(500, "internal", "Something went wrong.") : undefined,
    );
    render(<TogetherRoute />);
    expect(await screen.findByTestId(TESTID.friendsError, undefined, SLOW)).toHaveTextContent("Something went wrong.");
    expect(screen.getByTestId(TESTID.friendsRetry)).toBeEnabled();
    expect(rows()).toHaveLength(0);

    refused = false;
    fireEvent.click(screen.getByTestId(TESTID.friendsRetry));
    await waitFor(() => {
      expect(rows()).toHaveLength(4);
    }, SLOW);
    expect(screen.queryByTestId(TESTID.friendsError)).toBeNull();
    expect(screen.queryByTestId(TESTID.friendsRetry)).toBeNull();
    expect(server.to("GET", "/api/friends")).toHaveLength(2);
  });

  it("#1691 a rate-limited friends list holds Try again", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    signIn("tok-together");
    serve(world(), (call) =>
      call.method === "GET" && call.path === "/api/friends"
        ? { status: 429, body: { error: { code: "rate_limited", message: "Slow down.", details: { retryAfterMs: 60_000 } } } }
        : undefined,
    );
    render(<TogetherRoute />);
    expect(await screen.findByTestId(TESTID.friendsError, undefined, SLOW)).toHaveTextContent(/Slow down\. You can try again in \d+ s\./);
    const retry = screen.getByTestId(TESTID.friendsRetry);
    expect(retry).toBeDisabled();
    expect(retry).toHaveTextContent(/Try again in \d+ s/);
  });

  it("B55 with no friends the page shows no friend row and still offers the add form", async () => {
    await renderPage(world({ friends: [] }));
    expect(rows()).toHaveLength(0);
    expect(screen.getByTestId(TESTID.addInput)).toBeInTheDocument();
    expect(screen.getByTestId(TESTID.addSubmit)).toBeInTheDocument();
  });

  it("#2115 the friends and the shared replays are rows skeletons while their reads are pending, then the rows", async () => {
    signIn("tok-together");
    serve(world({ friends: [ANN], shared: [SHARED] }));
    render(<TogetherRoute />);
    // Right after render no stubbed read has answered yet.
    expectSkeleton(screen.getByText("Loading your friends…"), "rows");
    expectSkeleton(screen.getByText("Loading…"), "rows");

    await waitFor(() => {
      expect(rows()).toHaveLength(1);
    }, SLOW);
    await screen.findByTestId(TESTID.sharedRow, undefined, SLOW);
    expect(screen.queryByText("Loading your friends…")).toBeNull();
    expect(screen.queryByText("Loading…")).toBeNull();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
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
    ["self", "That's you."],
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

  it("#2684 a first press on an incoming request's Block sends nothing and asks, naming the sender", async () => {
    const server = await renderPage();
    fireEvent.click(within(incomingRequestRow()).getByTestId(togetherTestid.requestBlock));
    await settle();
    expect(server.to("POST", "/api/blocks")).toHaveLength(0);
    const row = incomingRequestRow();
    expect(within(row).getByRole("alertdialog")).toHaveTextContent(`Block ${EVE.username}? Their request goes, and neither of you can reach the other.`);
    for (const testid of [togetherTestid.requestAccept, togetherTestid.requestDecline, togetherTestid.requestBlock]) {
      expect(within(row).queryByTestId(testid), testid).toBeNull();
    }
  });

  it("#2684 Block in the request's question sends one block of the sender", async () => {
    const server = await renderPage();
    fireEvent.click(within(incomingRequestRow()).getByTestId(togetherTestid.requestBlock));
    fireEvent.click(within(incomingRequestRow()).getByTestId(togetherTestid.confirm));
    await waitFor(() => {
      expect(server.to("POST", "/api/blocks")).toHaveLength(1);
    }, SLOW);
    expect(server.to("POST", "/api/blocks")[0]?.body).toEqual({ profileId: EVE.profileId });
    await settle();
    expect(server.to("POST", "/api/blocks")).toHaveLength(1);
  });

  it("#2684 Keep in the request's question sends nothing and puts back Accept, Decline and Block", async () => {
    const server = await renderPage();
    fireEvent.click(within(incomingRequestRow()).getByTestId(togetherTestid.requestBlock));
    fireEvent.click(within(incomingRequestRow()).getByTestId(togetherTestid.keep));
    await settle();
    expect(server.to("POST", "/api/blocks")).toHaveLength(0);
    expect(server.to("POST", "/api/friends/requests/rq-in/accept")).toHaveLength(0);
    expect(server.to("POST", "/api/friends/requests/rq-in/decline")).toHaveLength(0);
    const row = incomingRequestRow();
    expect(within(row).queryByRole("alertdialog")).toBeNull();
    for (const testid of [togetherTestid.requestAccept, togetherTestid.requestDecline, togetherTestid.requestBlock]) {
      expect(offers(row, testid), testid).toBe(true);
    }
  });
});

// B56: what each friend row offers

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
    const clock = manualClock();
    const server = await renderPage(world(), undefined, clock);
    const remove = within(rowOf(ANN)).getByRole("button", { name: /^remove/i });
    fireEvent.click(remove);
    expect(server.to("DELETE", "/api/friends/f-ann"), "R2119000: held for the undo window").toHaveLength(0);
    await act(async () => {
      await clock.advance(UNDO_WINDOW_MS);
    });
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
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const server = await renderPage();
    for (const friend of [BOB, CY, DEE]) {
      const control = within(rowOf(friend)).queryByTestId(TESTID.challenge);
      if (control !== null) fireEvent.click(control);
    }
    await settle({ advance: FRIENDS_POLL_MS + 1 });
    expect(server.to("GET", "/api/friends")).toHaveLength(2);
    expect(server.to("POST", "/api/challenges")).toHaveLength(0);
  });
});

// R2119000: Remove is undone, not asked

describe("R2119000 removing a friend is undone, not asked", () => {
  const ANN_PATH = "/api/friends/f-ann";

  /** The page, Remove pressed on Ann's row; the server unfriends her, or gives `answer`. */
  async function removeAnn(answer?: Reply): Promise<{ server: StubServer; tick: (ms: number) => Promise<void> }> {
    const state = world();
    const clock = manualClock();
    const server = await renderPage(
      state,
      (call) => {
        if (call.method !== "DELETE" || call.path !== ANN_PATH) return undefined;
        if (answer !== undefined) return answer;
        state.friends = state.friends.filter((friend) => friend.profileId !== ANN.profileId);
        return { status: 204 };
      },
      clock,
    );
    fireEvent.click(within(rowOf(ANN)).getByRole("button", { name: /^remove/i }));
    return {
      server,
      tick: async (ms) => {
        await act(async () => {
          await clock.advance(ms);
        });
      },
    };
  }

  function listed(): (string | null)[] {
    return rows().map((row) => row.getAttribute("data-profile-id"));
  }

  it("R2119000 Remove takes the friend off at once behind \"Friend removed · Undo\", asks nothing and sends nothing", async () => {
    const { server, tick } = await removeAnn();
    expect(listed()).toEqual([BOB.profileId, CY.profileId, DEE.profileId]);
    expect(screen.getByTestId(UNDO_TOAST)).toHaveTextContent("Friend removed · Undo");
    expect(screen.queryByRole("alertdialog")).toBeNull();
    await tick(UNDO_WINDOW_MS - 1);
    expect(server.to("DELETE", ANN_PATH)).toHaveLength(0);
  });

  it("R2119000 Undo puts the friend back in their place and sends nothing", async () => {
    const { server, tick } = await removeAnn();
    fireEvent.click(screen.getByTestId(UNDO_TOAST_UNDO));
    expect(listed()).toEqual([ANN.profileId, BOB.profileId, CY.profileId, DEE.profileId]);
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
    await tick(UNDO_WINDOW_MS * 2);
    expect(server.to("DELETE", ANN_PATH)).toHaveLength(0);
  });

  it("R2119000 the window's end sends the unfriend exactly once", async () => {
    const { server, tick } = await removeAnn();
    const reads = server.to("GET", "/api/friends").length;
    await tick(UNDO_WINDOW_MS);
    await waitFor(() => {
      expect(server.to("DELETE", ANN_PATH)).toHaveLength(1);
    }, SLOW);
    expect(server.to("DELETE", ANN_PATH)[0]?.keepalive).toBe(false);
    await waitFor(() => {
      expect(server.to("GET", "/api/friends").length).toBeGreaterThan(reads);
    }, SLOW);
    await tick(UNDO_WINDOW_MS * 2);
    expect(server.to("DELETE", ANN_PATH)).toHaveLength(1);
    expect(listed()).toEqual([BOB.profileId, CY.profileId, DEE.profileId]);
  });

  it("R2119000 pagehide sends the unfriend at once, with keepalive", async () => {
    const { server } = await removeAnn();
    act(() => {
      window.dispatchEvent(new Event("pagehide"));
    });
    await waitFor(() => {
      expect(server.to("DELETE", ANN_PATH)).toHaveLength(1);
    }, SLOW);
    expect(server.to("DELETE", ANN_PATH)[0]?.keepalive).toBe(true);
    expect(screen.queryByTestId(UNDO_TOAST)).toBeNull();
  });

  it("R2119000 a failed unfriend puts the friend back and shows the server's words", async () => {
    const { tick } = await removeAnn(refusal(404, "not_found", "That player is not your friend."));
    await tick(UNDO_WINDOW_MS);
    await waitFor(() => {
      expect(listed()).toEqual([ANN.profileId, BOB.profileId, CY.profileId, DEE.profileId]);
    }, SLOW);
    expect(screen.getByTestId(togetherTestid.error)).toHaveTextContent("That player is not your friend.");
  });
});

// B59: Shared with me

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
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    const server = await renderPage(world({ ...ONLY_CY, shared: [] }));
    await waitFor(() => {
      expect(server.to("GET", "/api/replays/shared").length).toBeGreaterThan(0);
    }, SLOW);
    await settle({ advance: FRIENDS_POLL_MS + 1 });
    expect(server.to("GET", "/api/friends")).toHaveLength(2);
    expect(screen.queryAllByText(wholeText("Ann#2"))).toHaveLength(0);
    expect(server.calls.some((call) => call.path.startsWith("/api/replays/m-"))).toBe(false);
  });

  it("#1568 a Shared with me list that could not be read offers Try again, which reads it again and lists it", async () => {
    let reads = 0;
    const server = await renderPage(world(ONLY_CY), (call) => {
      if (call.method === "GET" && call.path === "/api/replays/shared") {
        reads += 1;
        if (reads === 1) return refusal(500, "internal", "The shared replays are out of reach.");
      }
      return undefined;
    });

    expect(await screen.findByText(/the shared replays are out of reach/i, undefined, SLOW)).toBeInTheDocument();
    fireEvent.click(await screen.findByTestId(TESTID.sharedRetry, undefined, SLOW));

    const row = await screen.findByTestId(TESTID.sharedRow, undefined, SLOW);
    expect(row).toHaveAttribute("data-match-id", SHARED.matchId);
    await waitFor(() => {
      expect(server.to("GET", "/api/replays/shared")).toHaveLength(2);
    }, SLOW);
  });
});

// the co-op panel lives on the page

describe("B60 the page holds the co-op panel", () => {
  it("B60 /together shows the co-op panel's create control", async () => {
    await renderPage();
    expect(await screen.findByTestId(TESTID.coopCreate, undefined, SLOW)).toBeInTheDocument();
  });
});

// the challenge dialog is a modal for the keyboard (#1569), as the card drill-down is (#1527)

describe("the challenge dialog's focus (#1569)", () => {
  /** Press Ann's Challenge button. `fireEvent.click` leaves the focus where it was, as Safari does. */
  function openChallenge(): HTMLElement {
    const challenge = within(rowOf(ANN)).getByTestId(TESTID.challenge);
    fireEvent.click(challenge);
    return challenge;
  }

  it("opening it takes the focus to its checked mode, then to Send once the decks are read (#1831), and Escape closes it from anywhere and gives the focus back to the Challenge button", async () => {
    await renderPage();
    const challenge = openChallenge();
    // Before the decks are read there is nothing to send: the focus starts on the checked mode.
    expect(screen.getByTestId(TESTID.challengeModeBo1)).toHaveFocus();
    const form = await screen.findByTestId(TESTID.challengeForm);
    expect(form).toHaveAttribute("role", "dialog");
    expect(form).toHaveAttribute("aria-modal", "true");
    await waitFor(() => {
      expect(screen.getByTestId(TESTID.challengeSend)).toHaveFocus();
    }, SLOW);

    (document.activeElement as HTMLElement | null)?.blur();
    press("Escape");
    expect(screen.queryByTestId(TESTID.challengeForm)).toBeNull();
    expect(challenge).toHaveFocus();
  });

  it("keeps Tab and Shift+Tab on its controls, the mode group one stop, and brings them back inside from the page behind", async () => {
    await renderPage();
    openChallenge();
    await screen.findByTestId(TESTID.challengeDeck, undefined, SLOW);
    const mode = screen.getByTestId(TESTID.challengeModeBo1);
    const deck = screen.getByTestId(TESTID.challengeDeck);
    const send = screen.getByTestId(TESTID.challengeSend);
    const cancel = screen.getByTestId(TESTID.challengeClose);

    // It opens on Send (#1831), so the walk starts there: Cancel, then round to the mode and the deck.
    await waitFor(() => {
      expect(send).toHaveFocus();
    }, SLOW);
    press("Tab");
    expect(cancel).toHaveFocus();
    press("Tab");
    expect(mode).toHaveFocus();
    press("Tab");
    expect(deck).toHaveFocus();
    press("Tab");
    expect(send).toHaveFocus();
    press("Tab", { shiftKey: true });
    expect(deck).toHaveFocus();
    press("Tab", { shiftKey: true });
    expect(mode).toHaveFocus();

    const behind = screen.getByTestId(TESTID.addInput);
    behind.focus();
    press("Tab");
    expect(mode).toHaveFocus();
    behind.focus();
    press("Tab", { shiftKey: true });
    expect(cancel).toHaveFocus();
  });

  it("closes on a click on the scrim, never on one inside it, and on Cancel, each time giving the focus back to the Challenge button", async () => {
    await renderPage();
    const challenge = openChallenge();
    const form = await screen.findByTestId(TESTID.challengeForm);

    fireEvent.click(form);
    expect(screen.getByTestId(TESTID.challengeForm)).toBeInTheDocument();

    fireEvent.click(must(document.querySelector<HTMLElement>(".together-dialog-scrim"), "the scrim"));
    expect(screen.queryByTestId(TESTID.challengeForm)).toBeNull();
    expect(challenge).toHaveFocus();

    openChallenge();
    await screen.findByTestId(TESTID.challengeForm);
    expect(challenge).not.toHaveFocus();
    fireEvent.click(screen.getByTestId(TESTID.challengeClose));
    expect(screen.queryByTestId(TESTID.challengeForm)).toBeNull();
    expect(challenge).toHaveFocus();
  });

  it("a sent challenge closes it and gives the focus back to the Challenge button", async () => {
    const server = await renderPage(world(), (call) =>
      call.method === "POST" && call.path === "/api/challenges"
        ? { status: 201, body: { challengeId: "ch-1", roomCode: "ROOM01" } }
        : undefined,
    );
    const challenge = openChallenge();
    await screen.findByTestId(TESTID.challengeDeck, undefined, SLOW);
    fireEvent.click(screen.getByTestId(TESTID.challengeSend));
    await waitFor(() => {
      expect(screen.queryByTestId(TESTID.challengeForm)).toBeNull();
    }, SLOW);
    expect(challenge).toHaveFocus();
    expect(server.to("POST", "/api/challenges")).toHaveLength(1);
    stopFollowingChallenge();
  });
});

describe("R2094000 a co-op link opens Play together filled", () => {
  afterEach(() => {
    window.sessionStorage.clear();
    at("/together");
  });

  it("R2094000 opening /together?coop=abc234 signed in shows the co-op panel with ABC234 filled and Join focused", async () => {
    window.sessionStorage.clear();
    at("/together?coop=abc234");
    await renderPage();
    const input = await screen.findByTestId("coop-join-input", undefined, SLOW);
    expect(input).toHaveValue("ABC234");
    expect(window.location.pathname).toBe("/together");
    expect(window.location.search).toBe("");
    await waitFor(() => {
      expect(screen.getByTestId("coop-join-submit")).toHaveFocus();
    }, SLOW);
  });

  it("R2094000 an invalid code opens nothing filled", async () => {
    window.sessionStorage.clear();
    at("/together?coop=abc123");
    await renderPage();
    const input = await screen.findByTestId("coop-join-input", undefined, SLOW);
    expect(input).toHaveValue("");
    expect(window.location.search).toBe("");
    expect(window.sessionStorage.getItem("jackioh.together.coopLink")).toBeNull();
  });

  it("R2094000 a chat Join on the same screen fills the already-mounted join form and focuses Join", async () => {
    window.sessionStorage.clear();
    at("/together");
    const origin = window.location.origin;
    await renderPage(world(), (call) =>
      call.method === "GET" && call.path === `/api/chat/${ANN.profileId}`
        ? {
            status: 200,
            body: {
              messages: [
                {
                  id: "msg-1",
                  from: ANN.profileId,
                  to: ME.profileId,
                  body: `Join my co-op game here: ${origin}/together?coop=ABC234 (room code ABC234).`,
                  createdAt: "2026-10-14T09:01:00.000Z",
                  read: true,
                },
              ],
              nextCursor: null,
            },
          }
        : undefined,
    );
    fireEvent.click(must(within(rowOf(ANN)).queryByTestId(TESTID.chatOpen), "chat"));
    fireEvent.click(await screen.findByTestId("chat-join", undefined, SLOW));
    const input = await screen.findByTestId("coop-join-input", undefined, SLOW);
    await waitFor(() => {
      expect(input).toHaveValue("ABC234");
    }, SLOW);
    expect(window.location.pathname).toBe("/together");
    expect(window.location.search).toBe("");
    await waitFor(() => {
      expect(screen.getByTestId("coop-join-submit")).toHaveFocus();
    }, SLOW);
  });
});

describe("R2615000 watching has no switch", () => {
  it("R2615000 the Watching panel says friends may always watch and how to stop one, and offers no switch", async () => {
    const server = await renderPage();
    const panel = must(screen.getByRole("heading", { name: "Watching" }).closest("section"), "the Watching panel");
    expect(within(panel).getByTestId(togetherTestid.watchingNote)).toHaveTextContent(
      "Your friends can always watch your online games, a few seconds behind. To stop someone watching, remove them as a friend or block them.",
    );
    expect(within(panel).queryByRole("checkbox")).toBeNull();
    expect(within(panel).queryByRole("switch")).toBeNull();
    // Nothing on the page reads or writes a watching setting.
    expect(server.calls.some((call) => call.path === "/api/settings")).toBe(false);
  });
});
