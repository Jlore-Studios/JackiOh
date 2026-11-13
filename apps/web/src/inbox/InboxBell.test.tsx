// R3729002: the inbox's bell and panel. The server is a stubbed `fetch` (`test/stubServer.ts`) and the shell's
// toast is rendered beside the bell, as `main.tsx` mounts it: it is the one social poll the bell reads
// (`together/socialPoll.ts`). Each answer in the panel calls its own route and reads the lists again; a
// refusal shows in the server's words.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { DailyGameSummary, DailyInvite, DailyList } from "../net/daily.ts";
import type { Challenge, FriendRequest, PlayerRef, SharedReplay } from "../net/social.ts";
import { readSession } from "../net/session.ts";
import { forgetRoomLink } from "../net/roomLink.ts";
import { settle } from "../test/settle.ts";
import { at, meBody, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { ChallengeToast } from "../together/ChallengeToast.tsx";
import { CHALLENGE_CHOICE_KEY } from "../together/challengeChoice.ts";
import { SOCIAL_POLL_MS } from "../together/config.ts";
import { challengeToastTestid } from "../together/testids.ts";
import { INBOX_MIN_GAP_MS, INBOX_POLL_MS, INBOX_SEEN_KEY } from "./config.ts";
import { forgetInvites, noteInvite } from "./invites.ts";
import { InboxBell } from "./InboxBell.tsx";
import { replayKey } from "./order.ts";
import { resetInboxForTests } from "./store.ts";
import { inboxTestid } from "./testids.ts";

const SOON = { timeout: 8_000 } as const;
const TEST_MS = 15_000;

const ME: PlayerRef = { profileId: "profile-1", username: "Me#1" };
const ANN: PlayerRef = { profileId: "f-ann", username: "Ann#2" };
const BOB: PlayerRef = { profileId: "f-bob", username: "Bob#3" };

const JOINED_MATCH = "m-9";

function challenge(challengeId: string, over: Partial<Challenge> = {}): Challenge {
  return {
    challengeId,
    from: ANN,
    to: ME,
    roomCode: `HJK${challengeId.slice(-1)}34`,
    mode: "random",
    expiresAt: new Date(Date.now() + 90_000).toISOString(),
    ...over,
  };
}

/** A day and six hours and a half, so the move's time left reads "1d 6h left" for a while whatever the test's pace. */
const DAILY_LEFT_MS = (24 + 6) * 60 * 60 * 1000 + 30 * 60 * 1000;

const REQUEST: FriendRequest = { requestId: "rq-1", from: BOB, to: ME, createdAt: "2026-10-14T09:00:00.000Z" };

const SHARED: SharedReplay = {
  matchId: "m-shared-1",
  sharer: ANN,
  createdAt: "2026-10-14T08:00:00.000Z",
  endedAt: "2026-10-14T07:50:00.000Z",
};

function game(matchId: string, over: Partial<DailyGameSummary> = {}): DailyGameSummary {
  return {
    matchId,
    mode: "bo1",
    perMoveDays: 3,
    seat: "p1",
    opponent: BOB,
    portraits: { me: "vanilla", them: "gary" },
    turn: 4,
    life: { me: 30, them: 22 },
    hasMove: true,
    theirMove: false,
    deadline: Date.now() + DAILY_LEFT_MS,
    over: false,
    result: null,
    endedAt: null,
    createdAt: Date.now() - 1000,
    ...over,
  };
}

function dailyChallenge(id: string): DailyInvite {
  return { id, kind: "challenge", from: ANN, to: ME, mode: "random", perMoveDays: 3, expiresAt: Date.now() + 60 * 60 * 1000, createdAt: Date.now() - 1000 };
}

type World = {
  incoming: Challenge[];
  requests: FriendRequest[];
  shared: SharedReplay[];
  daily: DailyList | null;
  dailyWaiting: number;
};

function world(over: Partial<World> = {}): World {
  return { incoming: [], requests: [], shared: [], daily: { games: [], challenges: { incoming: [], outgoing: [] }, seeks: [], waiting: 0 }, dailyWaiting: 0, ...over };
}

const DECK = { catalogVersion: "v1", portrait: null, createdAt: 0, updatedAt: 0, cards: [] };
const DECKS = {
  catalogVersion: "v1",
  decks: [
    { ...DECK, id: "d-aggro", name: "Aggro" },
    { ...DECK, id: "d-control", name: "Control" },
  ],
  trios: [],
  limits: { decks: 20, trios: 10, nameLength: 40 },
};

/** The reads the bell makes, and a plain answer to each answer the panel sends; `state` changes as the stub is answered. */
function serve(state: World, special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    const { method, path } = call;
    if (path === "/api/auth/me") return { status: 200, body: meBody({ profile: { id: ME.profileId, status: "active" }, email: "me@example.test" }) };
    if (method === "GET" && path === "/api/social") {
      return { status: 200, body: { incoming: state.incoming, outgoing: [], unread: {}, dailyWaiting: state.dailyWaiting } };
    }
    if (method === "GET" && path === "/api/friends") return { status: 200, body: { friends: [], incoming: state.requests, outgoing: [], blocked: [] } };
    if (method === "GET" && path === "/api/replays/shared") return { status: 200, body: { shared: state.shared } };
    if (method === "GET" && path === "/api/daily") return state.daily === null ? refusal(500, "internal", "The daily games are out of reach.") : { status: 200, body: state.daily };
    if (method === "GET" && path === "/api/decks") return { status: 200, body: DECKS };
    if (method === "GET" && path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    if (method === "POST" && path === "/api/presence") return { status: 204 };
    if (method === "POST" && /^\/api\/rooms\/[^/]+\/join$/.test(path)) {
      const code = path.split("/")[3] ?? "";
      return { status: 200, body: { matchId: JOINED_MATCH, seriesId: null, code, seat: "p2", mode: (call.body as { mode?: string } | undefined)?.mode ?? "bo1" } };
    }
    if (method === "POST" && /^\/api\/challenges\/[^/]+\/decline$/.test(path)) {
      state.incoming = state.incoming.filter((each) => !path.includes(each.challengeId));
      return { status: 204 };
    }
    if (method === "POST" && /^\/api\/friends\/requests\/[^/]+\/(accept|decline)$/.test(path)) {
      state.requests = state.requests.filter((each) => !path.includes(each.requestId));
      return path.endsWith("/accept")
        ? { status: 200, body: { friend: { ...BOB, status: "online", matchId: null, watchable: false, since: "2026-10-14T09:05:00.000Z" } } }
        : { status: 204 };
    }
    if (method === "POST" && /^\/api\/daily\/challenges\/[^/]+\/decline$/.test(path)) {
      if (state.daily !== null) state.daily = { ...state.daily, challenges: { incoming: [], outgoing: [] } };
      return { status: 204 };
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

async function bell(): Promise<HTMLElement> {
  return screen.findByTestId(inboxTestid.bell, undefined, SOON);
}

async function openPanel(): Promise<HTMLElement> {
  fireEvent.click(await bell());
  return screen.findByTestId(inboxTestid.panel, undefined, SOON);
}

function row(panel: HTMLElement, kind: string): HTMLElement {
  const found = panel.querySelector<HTMLElement>(`[data-testid="${inboxTestid.item}"][data-kind="${kind}"]`);
  if (found === null) throw new Error(`no ${kind} row`);
  return found;
}

function kinds(panel: HTMLElement): (string | null)[] {
  return within(panel)
    .queryAllByTestId(inboxTestid.item)
    .map((each) => each.getAttribute("data-kind"));
}

/** How many times the bell's lists were read. */
function reads(server: StubServer, path: string): number {
  return server.to("GET", path).length;
}

beforeEach(() => {
  window.localStorage.clear();
  forgetRoomLink();
  resetInboxForTests();
  at("/collection");
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  forgetInvites();
  forgetRoomLink();
  window.localStorage.clear();
});

describe("R3729002 the bell", () => {
  it(
    "R3729002 it shows once the first social read is in, with no badge when nothing waits",
    async () => {
      signIn("tok-inbox");
      serve(world());
      shell();
      // No read of the challenges yet: no bell, as the Friends button has none.
      expect(screen.queryByTestId(inboxTestid.bell)).toBeNull();
      const button = await bell();
      expect(button).toHaveAccessibleName("Inbox");
      expect(button).toHaveClass("top-bar-button", "top-bar-button--round");
      expect(screen.queryByTestId(inboxTestid.badge)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "R3729002 the badge is the social read's dailyWaiting until the daily list has been read",
    async () => {
      signIn("tok-inbox");
      serve(world({ dailyWaiting: 2, daily: null }));
      shell();
      const badge = await screen.findByTestId(inboxTestid.badge, undefined, SOON);
      expect(badge).toHaveTextContent("2");
      expect(await bell()).toHaveAccessibleName("Inbox, 2 waiting for you");

      // The list in, the games on the player's move are items of their own and the count is theirs, not counted twice.
      cleanup();
      resetInboxForTests();
      serve(world({ dailyWaiting: 5, daily: { games: [game("g-1")], challenges: { incoming: [], outgoing: [] }, seeks: [], waiting: 5 } }));
      shell();
      await waitFor(() => {
        expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("1");
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R3729002 the badge counts the challenge, the request and the replay, and opening the panel marks the replay seen",
    async () => {
      signIn("tok-inbox");
      serve(world({ incoming: [challenge("ch-1")], requests: [REQUEST], shared: [SHARED] }));
      shell();
      await waitFor(() => {
        expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("3");
      }, SOON);
      expect(await bell()).toHaveAccessibleName("Inbox, 3 waiting for you");

      const panel = await openPanel();
      expect(kinds(panel)).toEqual(["challenge", "request", "replay"]);
      await waitFor(() => {
        expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("2");
      }, SOON);
      expect(JSON.parse(window.localStorage.getItem(INBOX_SEEN_KEY) ?? "[]")).toEqual([replayKey(SHARED)]);

      // Another visit: the replay was seen, so only what needs an answer counts.
      cleanup();
      resetInboxForTests();
      shell();
      await waitFor(() => {
        expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("2");
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R3729002 it is hidden when signed out, and when the server refuses the session",
    async () => {
      const server = serve(world({ incoming: [challenge("ch-1")] }));
      shell();
      await settle({ macrotasks: 5 });
      expect(readSession()).toBeNull();
      expect(screen.queryByTestId(inboxTestid.bell)).toBeNull();
      // Signed out the bell reads nothing of its own.
      expect(reads(server, "/api/daily")).toBe(0);
      expect(reads(server, "/api/friends")).toBe(0);
      expect(reads(server, "/api/replays/shared")).toBe(0);
      cleanup();

      signIn("tok-inbox");
      serve(world(), (call) => (call.path === "/api/social" ? refusal(403, "forbidden", "Not an active account.") : undefined));
      shell();
      await settle({ macrotasks: 5 });
      expect(screen.queryByTestId(inboxTestid.bell)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "R3729002 nothing waiting shows the empty state",
    async () => {
      signIn("tok-inbox");
      serve(world());
      shell();
      const panel = await openPanel();
      expect(await within(panel).findByTestId(inboxTestid.empty, undefined, SOON)).toHaveTextContent("Nothing is waiting for you.");
      expect(within(panel).queryByTestId(inboxTestid.item)).toBeNull();
    },
    TEST_MS,
  );

  it(
    "R3729002 the panel is the bell's dialog; Escape closes it and the focus returns to the bell",
    async () => {
      signIn("tok-inbox");
      serve(world());
      shell();
      const button = await bell();
      const panel = await openPanel();
      expect(panel).toHaveAttribute("role", "dialog");
      expect(button).toHaveAttribute("aria-expanded", "true");
      expect(button).toHaveAttribute("aria-controls", panel.id);
      expect(panel).toContainElement(document.activeElement as HTMLElement);
      (document.activeElement as HTMLElement | null)?.blur();
      fireEvent.keyDown(document.body, { key: "Escape" });
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);
      expect(button).toHaveFocus();
      expect(button).toHaveAttribute("aria-expanded", "false");
    },
    TEST_MS,
  );
});

describe("R3729002 a live challenge in the inbox", () => {
  it(
    "R3729002 it shows who and what, and the time left, ticking",
    async () => {
      signIn("tok-inbox");
      serve(world({ incoming: [challenge("ch-1", { mode: "bo3" })] }));
      shell();
      const panel = await openPanel();
      const line = row(panel, "challenge");
      expect(line).toHaveAttribute("data-key", "challenge:ch-1");
      expect(line).toHaveTextContent("Ann#2 challenges you to Conquest.");
      const left = within(line).getByTestId(inboxTestid.left);
      expect(left.textContent).toMatch(/^1:\d\d left$/);
      const before = left.textContent;
      await waitFor(() => {
        expect(within(row(panel, "challenge")).getByTestId(inboxTestid.left).textContent).not.toBe(before);
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R3729002 Later on the toast leaves the challenge here; Accept joins with the mode alone and goes to the game",
    async () => {
      signIn("tok-inbox");
      const server = serve(world({ incoming: [challenge("ch-1")] }));
      shell();
      fireEvent.click(await screen.findByTestId(challengeToastTestid.later, undefined, SOON));
      expect(screen.queryAllByTestId(challengeToastTestid.toast)).toHaveLength(0);

      const panel = await openPanel();
      const before = reads(server, "/api/daily");
      fireEvent.click(within(row(panel, "challenge")).getByTestId(inboxTestid.accept));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      const joins = server.to("POST", "/api/rooms/HJK134/join");
      expect(joins).toHaveLength(1);
      expect(joins[0]?.body).toEqual({ mode: "random" });
      expect(server.to("POST", "/api/challenges/ch-1/decline")).toHaveLength(0);
      // The answer reads the lists again.
      await waitFor(() => {
        expect(reads(server, "/api/daily")).toBeGreaterThan(before);
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R3729002 a Best of 1 challenge joins with the remembered deck, once the decks are read",
    async () => {
      signIn("tok-inbox");
      window.localStorage.setItem(CHALLENGE_CHOICE_KEY, JSON.stringify({ mode: "bo1", deckId: "d-control", trioId: null }));
      const server = serve(world({ incoming: [challenge("ch-1", { mode: "bo1" })] }));
      shell();
      const panel = await openPanel();
      fireEvent.click(await within(panel).findByTestId(inboxTestid.accept, undefined, SOON));
      await waitFor(() => {
        expect(window.location.pathname).toBe(`/match/${JOINED_MATCH}`);
      }, SOON);
      expect(server.to("POST", "/api/rooms/HJK134/join")[0]?.body).toEqual({ mode: "bo1", deckId: "d-control" });
    },
    TEST_MS,
  );

  it(
    "R3729002 with no saved deck there is only Open in lobby, which opens the room's lobby",
    async () => {
      signIn("tok-inbox");
      serve(world({ incoming: [challenge("ch-1", { mode: "bo1" })] }), (call) =>
        call.method === "GET" && call.path === "/api/decks" ? { status: 200, body: { ...DECKS, decks: [] } } : undefined,
      );
      shell();
      const panel = await openPanel();
      const lobby = await within(panel).findByTestId(inboxTestid.lobby, undefined, SOON);
      expect(within(panel).queryByTestId(inboxTestid.accept)).toBeNull();
      fireEvent.click(lobby);
      await waitFor(() => {
        expect(window.location.pathname).toBe("/lobby/HJK134");
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R3729002 a refused join shows the server's words in the panel, and Open in lobby is offered",
    async () => {
      signIn("tok-inbox");
      const server = serve(world({ incoming: [challenge("ch-1")] }), (call) =>
        call.method === "POST" && call.path.endsWith("/join") ? refusal(409, "already_in_match", "finish your current match first") : undefined,
      );
      shell();
      const panel = await openPanel();
      expect(within(panel).queryByTestId(inboxTestid.lobby)).toBeNull();
      fireEvent.click(within(row(panel, "challenge")).getByTestId(inboxTestid.accept));
      expect(await within(panel).findByTestId(inboxTestid.error, undefined, SOON)).toHaveTextContent("finish your current match first");
      expect(window.location.pathname).toBe("/collection");
      expect(server.to("POST", "/api/rooms/HJK134/join")).toHaveLength(1);
      // The row stays, to try again or to open the lobby.
      expect(within(row(panel, "challenge")).getByTestId(inboxTestid.accept)).not.toBeDisabled();
      expect(within(row(panel, "challenge")).getByTestId(inboxTestid.lobby)).toBeInTheDocument();
    },
    TEST_MS,
  );

  it(
    "R3729002 Decline declines it, the row goes and the badge counts one fewer",
    async () => {
      signIn("tok-inbox");
      const server = serve(world({ incoming: [challenge("ch-1"), challenge("ch-2")] }));
      shell();
      const panel = await openPanel();
      expect(within(panel).getAllByTestId(inboxTestid.item)).toHaveLength(2);
      fireEvent.click(within(row(panel, "challenge")).getByTestId(inboxTestid.decline));
      await waitFor(() => {
        expect(server.to("POST", "/api/challenges/ch-1/decline").length + server.to("POST", "/api/challenges/ch-2/decline").length).toBe(1);
      }, SOON);
      await waitFor(() => {
        expect(within(panel).getAllByTestId(inboxTestid.item)).toHaveLength(1);
      }, SOON);
      await waitFor(() => {
        expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("1");
      }, SOON);
      expect(window.location.pathname).toBe("/collection");
    },
    TEST_MS,
  );

  it(
    "R3729002 a challenge that lapses leaves the inbox and the badge without another read",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "Date"] });
      signIn("tok-inbox");
      const server = serve(world({ incoming: [challenge("ch-1", { expiresAt: new Date(Date.now() + 5_000).toISOString() })] }));
      shell();
      await settle({ advance: 1 });
      await waitFor(() => {
        expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("1");
      }, SOON);
      const socialReads = reads(server, "/api/social");
      await settle({ advance: 6_000 });
      expect(screen.queryByTestId(inboxTestid.badge)).toBeNull();
      expect(reads(server, "/api/social")).toBeLessThanOrEqual(socialReads + 1);
    },
    TEST_MS,
  );
});

describe("R3729002 the other kinds in the inbox", () => {
  it(
    "R3729002 a friend request is accepted or declined in the row, and the lists are read again",
    async () => {
      signIn("tok-inbox");
      const state = world({ requests: [REQUEST, { ...REQUEST, requestId: "rq-2", createdAt: "2026-10-15T09:00:00.000Z" }] });
      const server = serve(state);
      shell();
      const panel = await openPanel();
      expect(within(panel).getAllByTestId(inboxTestid.item).map((each) => each.getAttribute("data-key"))).toEqual(["request:rq-1", "request:rq-2"]);
      expect(within(row(panel, "request")).getByText(/wants to be friends/)).toBeInTheDocument();

      const friendsBefore = reads(server, "/api/friends");
      fireEvent.click(within(row(panel, "request")).getByTestId(inboxTestid.accept));
      await waitFor(() => {
        expect(server.to("POST", "/api/friends/requests/rq-1/accept")).toHaveLength(1);
      }, SOON);
      await waitFor(() => {
        expect(within(panel).getAllByTestId(inboxTestid.item)).toHaveLength(1);
      }, SOON);
      expect(reads(server, "/api/friends")).toBeGreaterThan(friendsBefore);

      fireEvent.click(within(row(panel, "request")).getByTestId(inboxTestid.decline));
      await waitFor(() => {
        expect(server.to("POST", "/api/friends/requests/rq-2/decline")).toHaveLength(1);
      }, SOON);
      expect(await within(panel).findByTestId(inboxTestid.empty, undefined, SOON)).toBeInTheDocument();
    },
    TEST_MS,
  );

  it(
    "R3729002 a refused answer shows the server's words in the panel and the row stays",
    async () => {
      signIn("tok-inbox");
      serve(world({ requests: [REQUEST] }), (call) =>
        call.method === "POST" && call.path.endsWith("/accept") ? refusal(409, "friend_limit", "You have as many friends as a list can hold.") : undefined,
      );
      shell();
      const panel = await openPanel();
      fireEvent.click(within(row(panel, "request")).getByTestId(inboxTestid.accept));
      expect(await within(panel).findByTestId(inboxTestid.error, undefined, SOON)).toHaveTextContent("You have as many friends as a list can hold.");
      expect(row(panel, "request")).toBeInTheDocument();
    },
    TEST_MS,
  );

  it(
    "R3729002 a shared replay's Watch goes to Play together with the replay named",
    async () => {
      signIn("tok-inbox");
      serve(world({ shared: [SHARED] }));
      shell();
      const panel = await openPanel();
      expect(within(row(panel, "replay")).getByText(/Shared by/)).toBeInTheDocument();
      fireEvent.click(within(row(panel, "replay")).getByTestId(inboxTestid.watch));
      expect(window.location.pathname).toBe("/together");
      expect(window.location.search).toBe("?replay=m-shared-1");
      expect(window.location.hash).toBe("#shared");
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);
    },
    TEST_MS,
  );

  it(
    "R3729002 a lobby invite the tab was sent shows Join and Dismiss; Join goes to the lobby, Dismiss drops it",
    async () => {
      signIn("tok-inbox");
      serve(world());
      shell();
      await bell();
      act(() => {
        noteInvite({ code: "K7Q2XM", format: "duel", from: ANN }, Date.now() - 5_000);
        noteInvite({ code: "COOPCD", format: "coop", from: BOB });
      });
      await waitFor(() => {
        expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("2");
      }, SOON);
      const panel = await openPanel();
      const invites = within(panel).getAllByTestId(inboxTestid.item);
      // The newest invite first.
      expect(invites.map((each) => each.getAttribute("data-key"))).toEqual(["invite:COOPCD", "invite:K7Q2XM"]);
      expect(invites[0]).toHaveTextContent("Bob#3 invites you to a co-op lobby");
      expect(invites[1]).toHaveTextContent("Ann#2 invites you to a 1v1 lobby");

      fireEvent.click(within(invites[0] as HTMLElement).getByTestId(inboxTestid.dismiss));
      expect(within(panel).getAllByTestId(inboxTestid.item)).toHaveLength(1);
      expect(window.location.pathname).toBe("/collection");

      fireEvent.click(within(row(panel, "invite")).getByTestId(inboxTestid.join));
      expect(window.location.pathname).toBe("/lobby/K7Q2XM");
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);
      // Joined: the invite is answered, so it is not listed again.
      fireEvent.click(await bell());
      expect(await screen.findByTestId(inboxTestid.empty, undefined, SOON)).toBeInTheDocument();
    },
    TEST_MS,
  );

  it(
    "R3729002 a daily game on the player's move shows its time left and Play opens the game; a daily challenge opens the Daily screen and declines inline",
    async () => {
      signIn("tok-inbox");
      const state = world({
        daily: { games: [game("g-1")], challenges: { incoming: [dailyChallenge("dc-1")], outgoing: [] }, seeks: [], waiting: 1 },
        dailyWaiting: 1,
      });
      const server = serve(state);
      shell();
      const panel = await openPanel();
      await waitFor(() => {
        expect(kinds(panel)).toEqual(["dailyMove", "dailyChallenge"]);
      }, SOON);

      const move = row(panel, "dailyMove");
      expect(move).toHaveTextContent("Bob#3");
      expect(move).toHaveTextContent(/Your move · 1d 6h left/);
      const play = within(move).getByTestId(inboxTestid.play);
      expect(play).toHaveAttribute("href", "/daily/g-1");

      const challengeRow = row(panel, "dailyChallenge");
      expect(challengeRow).toHaveTextContent("Ann#2 challenged you");
      expect(challengeRow).toHaveTextContent("All random · 3 days per move");
      expect(within(challengeRow).getByTestId(inboxTestid.open)).toHaveAttribute("href", "/daily");
      expect(within(challengeRow).queryByTestId(inboxTestid.accept)).toBeNull();

      fireEvent.click(within(challengeRow).getByTestId(inboxTestid.decline));
      await waitFor(() => {
        expect(server.to("POST", "/api/daily/challenges/dc-1/decline")).toHaveLength(1);
      }, SOON);
      await waitFor(() => {
        expect(kinds(panel)).toEqual(["dailyMove"]);
      }, SOON);

      fireEvent.click(within(row(panel, "dailyMove")).getByTestId(inboxTestid.play));
      expect(window.location.pathname).toBe("/daily/g-1");
    },
    TEST_MS,
  );
});

describe("R3729002 what the bell reads", () => {
  it(
    "R3729002 the lists are read at the first social read, not again within INBOX_MIN_GAP_MS, and again at the poll",
    async () => {
      vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "Date"] });
      signIn("tok-inbox");
      const server = serve(world());
      shell();
      await settle({ advance: 1 });
      await waitFor(() => {
        expect(reads(server, "/api/daily")).toBe(1);
      }, SOON);
      expect(reads(server, "/api/friends")).toBe(1);
      expect(reads(server, "/api/replays/shared")).toBe(1);

      // Social reads land every SOCIAL_POLL_MS, and a read of the lists is asked for each, but none is sent inside the gap.
      await settle({ advance: INBOX_MIN_GAP_MS - SOCIAL_POLL_MS });
      expect(reads(server, "/api/social")).toBeGreaterThan(2);
      expect(reads(server, "/api/daily")).toBe(1);

      await settle({ advance: INBOX_POLL_MS });
      expect(reads(server, "/api/daily")).toBeGreaterThan(1);
      expect(reads(server, "/api/friends")).toBe(reads(server, "/api/daily"));
    },
    TEST_MS,
  );

  it(
    "R3729002 a list that fails to read keeps the one before it, and the inbox does not empty",
    async () => {
      signIn("tok-inbox");
      let failing = false;
      const server = serve(world({ requests: [REQUEST] }), (call) =>
        failing && call.path === "/api/friends" ? refusal(500, "internal", "The friends are out of reach.") : undefined,
      );
      shell();
      const first = await openPanel();
      await within(first).findByTestId(inboxTestid.item, undefined, SOON);
      // Close it, let the next read fail, and open it again: opening reads the lists again.
      fireEvent.keyDown(document.body, { key: "Escape" });
      await waitFor(() => {
        expect(screen.queryByTestId(inboxTestid.panel)).toBeNull();
      }, SOON);
      failing = true;
      const before = reads(server, "/api/friends");
      const panel = await openPanel();
      await waitFor(() => {
        expect(reads(server, "/api/friends")).toBeGreaterThan(before);
      }, SOON);
      await settle({ macrotasks: 3 });
      expect(kinds(panel)).toEqual(["request"]);
      expect(screen.getByTestId(inboxTestid.badge)).toHaveTextContent("1");
    },
    TEST_MS,
  );
});
