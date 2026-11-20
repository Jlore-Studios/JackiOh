// R4039001: the inbox's Recent section, the notification feed the server keeps (R4039000), and the bell's dot.
// The server is a stubbed `fetch` (`test/stubServer.ts`) and the shell's challenge toast is rendered beside the
// bell, as `main.tsx` mounts it: it is the one social poll the bell reads. Each row says what happened in words
// and opens where it happened; opening the panel marks the feed read up to the newest row shown.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FeedNotification } from "../net/notifications.ts";
import { at, meBody, must, refusal, signIn, stubServer, type StubServer } from "../test/stubServer.ts";
import { fakeSockets, type FakeSocket } from "../test/fakeSocket.ts";
import { ChallengeToast } from "../together/ChallengeToast.tsx";
import { setSocialSocketFactory } from "../together/socialSocket.ts";
import { feedLine, feedPath, resetFeedForTests } from "./feed.ts";
import { InboxBell } from "./InboxBell.tsx";
import { forgetInvites } from "./invites.ts";
import { resetInboxForTests } from "./store.ts";
import { inboxTestid } from "./testids.ts";

const SOON = { timeout: 8_000 } as const;
const TEST_MS = 15_000;
const HOUR_MS = 60 * 60 * 1000;

const ME = { profileId: "profile-1", username: "Me#1" };
const ANN = { profileId: "f-ann", username: "Ann#2" };

function row(id: number, kind: FeedNotification["kind"], over: Partial<FeedNotification> = {}): FeedNotification {
  return { id, kind, actor: ANN, subject: `s-${String(id)}`, payload: {}, createdAt: Date.now() - 2 * HOUR_MS, read: true, ...over };
}

type World = {
  /** The feed, newest first; a page is `PAGE` rows. */
  feed: FeedNotification[];
  /** The social read's count of unread rows. */
  unread: number;
  /** A challenge waiting, for the badge. */
  challenge: boolean;
  /** The feed's read fails. */
  failing: boolean;
  /** How many of the next marks fail. */
  markFails: number;
};

const PAGE = 3;

function world(over: Partial<World> = {}): World {
  return { feed: [], unread: 0, challenge: false, failing: false, markFails: 0, ...over };
}

function serve(state: World): StubServer {
  return stubServer((call) => {
    const { method, path, query } = call;
    if (path === "/api/auth/me") return { status: 200, body: meBody() };
    if (method === "GET" && path === "/api/social") {
      const incoming = state.challenge
        ? [{ challengeId: "ch-1", from: ANN, to: ME, roomCode: "HJK134", mode: "random", expiresAt: new Date(Date.now() + 90_000).toISOString() }]
        : [];
      return { status: 200, body: { incoming, outgoing: [], unread: {}, dailyWaiting: 0, notificationsUnread: state.unread } };
    }
    if (method === "GET" && path === "/api/friends") return { status: 200, body: { friends: [], incoming: [], outgoing: [], blocked: [] } };
    if (method === "GET" && path === "/api/replays/shared") return { status: 200, body: { shared: [] } };
    if (method === "GET" && path === "/api/daily") return { status: 200, body: { games: [], challenges: { incoming: [], outgoing: [] }, seeks: [], waiting: 0 } };
    if (method === "GET" && path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    if (method === "POST" && path === "/api/presence") return { status: 204 };
    if (method === "GET" && path === "/api/notifications") {
      if (state.failing) return refusal(500, "internal", "The notifications are out of reach.");
      const before = query.get("before");
      const older = before === null ? state.feed : state.feed.filter((each) => each.id < Number(before));
      const page = older.slice(0, PAGE);
      const nextBefore = older.length > PAGE ? (page.at(-1)?.id ?? null) : null;
      return { status: 200, body: { notifications: page, nextBefore, unread: state.feed.filter((each) => !each.read).length } };
    }
    if (method === "POST" && path === "/api/notifications/read") {
      if (state.markFails > 0) {
        state.markFails -= 1;
        return refusal(500, "internal", "The mark did not land.");
      }
      const upTo = (call.body as { upTo?: number } | undefined)?.upTo ?? Number.POSITIVE_INFINITY;
      state.feed = state.feed.map((each) => (each.id <= upTo ? { ...each, read: true } : each));
      state.unread = state.feed.filter((each) => !each.read).length;
      return { status: 200, body: { unread: state.unread } };
    }
    return undefined;
  });
}

function shell(): void {
  render(
    <>
      <ChallengeToast />
      <InboxBell />
    </>,
  );
}

async function openPanel(): Promise<HTMLElement> {
  fireEvent.click(await screen.findByTestId(inboxTestid.bell, undefined, SOON));
  return screen.findByTestId(inboxTestid.panel, undefined, SOON);
}

function recentRows(panel: HTMLElement): HTMLElement[] {
  return within(panel).queryAllByTestId(inboxTestid.recentRow);
}

beforeEach(() => {
  window.localStorage.clear();
  resetInboxForTests();
  resetFeedForTests();
  at("/collection");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  forgetInvites();
  window.localStorage.clear();
});

describe("R4039001 the inbox's Recent section", () => {
  it(
    "R4039001 Recent lists the feed newest first in words, a dot on each unread row, and Show older reads the page before",
    async () => {
      signIn("tok-feed");
      const server = serve(
        world({
          feed: [
            row(9, "dailyMove", { read: false }),
            row(8, "friendRequest", { read: false }),
            row(7, "replayShared"),
            row(6, "dailyEnded", { payload: { outcome: "win", reason: "disconnect" } }),
            row(5, "friendAccepted", { actor: null }),
          ],
          unread: 2,
        }),
      );
      shell();
      const panel = await openPanel();
      expect(within(panel).getByText("Waiting on you")).toBeInTheDocument();
      expect(within(panel).getByText("Recent")).toBeInTheDocument();
      await waitFor(() => {
        expect(recentRows(panel)).toHaveLength(PAGE);
      }, SOON);
      const shown = recentRows(panel);
      expect(shown.map((each) => each.getAttribute("data-kind"))).toEqual(["dailyMove", "friendRequest", "replayShared"]);
      expect(shown.map((each) => each.getAttribute("data-unread"))).toEqual(["true", "true", "false"]);
      expect(shown[0]).toHaveTextContent("Your move against Ann#2");
      expect(shown[0]).toHaveTextContent("2 h ago");
      expect(shown[1]).toHaveTextContent("Ann#2 sent you a friend request");
      expect(shown.map((each) => within(each).queryByTestId(inboxTestid.recentDot) !== null)).toEqual([true, true, false]);

      fireEvent.click(within(panel).getByTestId(inboxTestid.older));
      await waitFor(() => {
        expect(recentRows(panel)).toHaveLength(5);
      }, SOON);
      expect(server.to("GET", "/api/notifications").at(-1)?.query.get("before")).toBe("7");
      const all = recentRows(panel);
      expect(all[3]).toHaveTextContent("Your daily game with Ann#2 ended: you won on time");
      expect(all[4]).toHaveTextContent("Someone accepted your friend request");
      // The last page: nothing older is left.
      expect(within(panel).queryByTestId(inboxTestid.older)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "R4039001 opening the panel marks the feed read up to the newest row shown, and the bell's dot shows only with no badge",
    async () => {
      signIn("tok-feed");
      const state = world({ feed: [row(4, "dailyChallenge", { read: false }), row(3, "lobbyInvite", { read: false })], unread: 2 });
      const server = serve(state);
      shell();
      const bell = await screen.findByTestId(inboxTestid.bell, undefined, SOON);
      const dot = await screen.findByTestId(inboxTestid.dot, undefined, SOON);
      expect(bell).toContainElement(dot);
      expect(screen.queryByTestId(inboxTestid.badge)).toBeNull();
      expect(bell).toHaveAttribute("aria-label", "Inbox, new notifications");

      await openPanel();
      await waitFor(() => {
        expect(server.to("POST", "/api/notifications/read")).toHaveLength(1);
      }, SOON);
      expect(server.to("POST", "/api/notifications/read")[0]?.body).toEqual({ upTo: 4 });
      // The social read again: nothing unread, so no dot.
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.dot)).toBeNull();
      }, SOON);
      expect(bell).toHaveAttribute("aria-label", "Inbox");
    },
    TEST_MS,
  );

  it(
    "R4039001 opening the panel marks an older unread row below a page its acts already read",
    async () => {
      signIn("tok-feed");
      // R4233000: the acts the rows are about marked 5, 4 and 3 read; 2, older than the page, is not.
      const state = world({ feed: [row(5, "dailyMove"), row(4, "dailyEnded"), row(3, "friendRequest"), row(2, "lobbyInvite", { read: false })], unread: 1 });
      const server = serve(state);
      shell();
      await screen.findByTestId(inboxTestid.dot, undefined, SOON);

      await openPanel();
      await waitFor(() => {
        expect(server.to("POST", "/api/notifications/read")).toHaveLength(1);
      }, SOON);
      expect(server.to("POST", "/api/notifications/read")[0]?.body).toEqual({ upTo: 5 });
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.dot)).toBeNull();
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R4039001 a mark that fails is sent again at the next opening",
    async () => {
      signIn("tok-feed");
      const state = world({ feed: [row(4, "dailyChallenge", { read: false })], unread: 1, markFails: 1 });
      const server = serve(state);
      shell();
      await screen.findByTestId(inboxTestid.dot, undefined, SOON);
      await openPanel();
      await waitFor(() => {
        expect(server.to("POST", "/api/notifications/read")).toHaveLength(1);
      }, SOON);
      fireEvent.click(screen.getByTestId(inboxTestid.close));
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);
      expect(state.unread, "the failed mark left the row unread at the server").toBe(1);
      expect(screen.getByTestId(inboxTestid.dot)).toBeInTheDocument();

      await openPanel();
      await waitFor(() => {
        expect(server.to("POST", "/api/notifications/read")).toHaveLength(2);
      }, SOON);
      expect(server.to("POST", "/api/notifications/read")[1]?.body).toEqual({ upTo: 4 });
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.dot)).toBeNull();
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R4039001 a row pushed while the panel is open reads the page again and marks it",
    async () => {
      const fake = fakeSockets();
      setSocialSocketFactory(fake.factory);
      try {
        signIn("tok-feed");
        const state = world({ feed: [row(3, "friendAccepted")] });
        const server = serve(state);
        shell();
        const panel = await openPanel();
        await waitFor(() => {
          expect(recentRows(panel)).toHaveLength(1);
        }, SOON);
        expect(server.to("POST", "/api/notifications/read")).toHaveLength(0);
        const socket: FakeSocket = must(fake.sockets.at(-1), "the tab's social socket");
        socket.open();

        const pushed = row(4, "dailyMove", { read: false });
        state.feed = [pushed, ...state.feed];
        state.unread = 1;
        socket.deliver({ type: "notification", notification: pushed });
        await waitFor(() => {
          expect(recentRows(panel)).toHaveLength(2);
        }, SOON);
        await waitFor(() => {
          expect(server.to("POST", "/api/notifications/read")).toHaveLength(1);
        }, SOON);
        expect(server.to("POST", "/api/notifications/read")[0]?.body).toEqual({ upTo: 4 });
        expect(state.unread).toBe(0);
      } finally {
        setSocialSocketFactory(null);
      }
    },
    TEST_MS,
  );

  it(
    "R4039001 a badge takes the dot's place, and a page with nothing unread sends no mark",
    async () => {
      signIn("tok-feed");
      const server = serve(world({ feed: [row(2, "replayShared")], unread: 1, challenge: true }));
      shell();
      expect(await screen.findByTestId(inboxTestid.badge, undefined, SOON)).toHaveTextContent("1");
      expect(screen.queryByTestId(inboxTestid.dot)).toBeNull();
      const panel = await openPanel();
      await waitFor(() => {
        expect(recentRows(panel)).toHaveLength(1);
      }, SOON);
      expect(server.to("POST", "/api/notifications/read")).toHaveLength(0);
    },
    TEST_MS,
  );

  it(
    "R4039001 an empty feed says so, and a row's Open goes where it happened and closes the panel",
    async () => {
      signIn("tok-feed");
      const state = world();
      serve(state);
      shell();
      let panel = await openPanel();
      expect(await within(panel).findByTestId(inboxTestid.recentEmpty, undefined, SOON)).toHaveTextContent("Nothing yet.");
      fireEvent.click(within(panel).getByTestId(inboxTestid.close));
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);

      state.feed = [row(1, "dailyMove", { subject: "m-7" })];
      panel = await openPanel();
      const [only] = await within(panel).findAllByTestId(inboxTestid.recentRow, undefined, SOON);
      fireEvent.click(within(only as HTMLElement).getByTestId(inboxTestid.recentOpen));
      expect(window.location.pathname).toBe("/daily/m-7");
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R4039001 a failed read keeps the rows shown and says so under Recent, never in the panel's alert",
    async () => {
      signIn("tok-feed");
      const state = world({ feed: [row(3, "friendAccepted")] });
      serve(state);
      shell();
      let panel = await openPanel();
      await waitFor(() => {
        expect(recentRows(panel)).toHaveLength(1);
      }, SOON);
      fireEvent.click(within(panel).getByTestId(inboxTestid.close));
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);

      state.failing = true;
      panel = await openPanel();
      expect(await within(panel).findByTestId(inboxTestid.recentError, undefined, SOON)).toHaveTextContent("The notifications are out of reach.");
      expect(recentRows(panel)).toHaveLength(1);
      expect(within(panel).queryByTestId(inboxTestid.error)).toBeNull();
    },
    TEST_MS,
  );
});

describe("R4039001 a row in words and where it opens", () => {
  it("R4039001 each kind says what happened, with the actor's name, and opens where it happened", () => {
    const cases: [FeedNotification, string, string | null][] = [
      [row(1, "dailyMove", { subject: "m-1" }), "Your move against Ann#2", "/daily/m-1"],
      [row(2, "dailyEnded", { subject: "m-2", payload: { outcome: "win", reason: "concede" } }), "Your daily game with Ann#2 ended: you won", "/daily/m-2"],
      [row(3, "dailyEnded", { subject: "m-3", payload: { outcome: "loss", reason: "disconnect" } }), "Your daily game with Ann#2 ended: you lost on time", "/daily/m-3"],
      [row(4, "dailyEnded", { subject: "m-4", payload: { outcome: "loss", reason: "heroDeath" } }), "Your daily game with Ann#2 ended: you lost", "/daily/m-4"],
      [row(5, "dailyEnded", { subject: "m-5", payload: { outcome: "draw", reason: "heroDeath" } }), "Your daily game with Ann#2 ended: it was a draw", "/daily/m-5"],
      [row(6, "dailyChallenge", { payload: { mode: "random", perMoveDays: 3 } }), "Ann#2 challenged you to a daily game", "/daily"],
      [row(7, "dailyAnswered", { payload: { answer: "accepted", matchId: "m-8" } }), "Ann#2 accepted your daily challenge", "/daily/m-8"],
      [row(8, "dailyAnswered", { payload: { answer: "declined" } }), "Ann#2 declined your daily challenge", "/daily"],
      [row(9, "friendRequest"), "Ann#2 sent you a friend request", "/together?section=requests"],
      [row(10, "friendAccepted"), "Ann#2 accepted your friend request", "/together?section=friends"],
      [row(11, "lobbyInvite", { subject: "K7Q2XM", payload: { format: "duel" } }), "Ann#2 invited you to their lobby", "/lobby/K7Q2XM"],
      [row(12, "replayShared", { subject: "m-12" }), "Ann#2 shared a replay with you", "/together?section=shared&replay=m-12"],
      [row(13, "friendRequest", { actor: null }), "Someone sent you a friend request", "/together?section=requests"],
    ];
    for (const [each, line, path] of cases) {
      expect(feedLine(each), each.kind).toBe(line);
      expect(feedPath(each), each.kind).toBe(path);
    }
  });
});
