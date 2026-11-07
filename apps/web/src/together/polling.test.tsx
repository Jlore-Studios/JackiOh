// #936 review (finding 8): how often the social screens ask the server again. The friends list is the
// social layer's dearest read, so off its screen it is read only while the friends panel is open
// (#1831); an open conversation keeps CHAT_POLL_MS (`ChatPanel.test.tsx`). R3214000: while the tab's
// social socket is live the poll sends nothing, and a push stands in for it.
// The server is a stubbed `fetch` (`test/stubServer.ts`), the social socket a fake one
// (`test/fakeSocket.ts`), and time is vitest's fake clock.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CHAT_POLL_MS, PRESENCE_HEARTBEAT_MS } from "@jackioh/server-config";

import { rememberRead } from "../net/readCache.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { FRIENDS_PATH, type Challenge, type Friend } from "../net/social.ts";
import { tokenFromProtocols } from "../net/socketProtocols.ts";
import { fakeSockets, type FakeSocket } from "../test/fakeSocket.ts";
import { settle } from "../test/settle.ts";
import { resetVisibility, setVisibility } from "../test/setup.ts";
import { must, stubServer, type StubServer } from "../test/stubServer.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";
import { FRIENDS_POLL_MS, SOCIAL_POLL_MS } from "./config.ts";
import { FriendsButton } from "./FriendsButton.tsx";
import { setSocialSocketFactory } from "./socialSocket.ts";
import { TogetherRoute } from "./TogetherRoute.tsx";
import { challengeToastTestid, friendsTestid, togetherTestid } from "./testids.ts";

const ANN: Friend = {
  profileId: "f-ann",
  username: "Ann#2",
  status: "online",
  matchId: null,
  watchable: false,
  since: "2026-10-01T00:00:00.000Z",
};

/** R3205000: a challenge this account sent, as `ChallengeToast.test.tsx` has one. */
const OUTGOING: Challenge = {
  challengeId: "ch-2",
  from: { profileId: "profile-1", username: "Me#1" },
  to: { profileId: "f-bob", username: "Bob#3" },
  roomCode: "MNP567",
  mode: "bo1",
  expiresAt: "2099-01-01T00:00:00.000Z",
};

/** R3214000: a challenge a friend sent, pushed over the social socket. */
const INCOMING: Challenge = {
  challengeId: "ch-1",
  from: { profileId: "f-ann", username: "Ann#2" },
  to: { profileId: "profile-1", username: "Me#1" },
  roomCode: "HJK234",
  mode: "bo1",
  expiresAt: "2099-01-01T00:00:00.000Z",
};

type Challenges = { incoming: Challenge[]; outgoing: Challenge[] };

function serve(challenges: () => Challenges = () => ({ incoming: [], outgoing: [] })): StubServer {
  return stubServer((call) => {
    const { method, path } = call;
    if (path === "/api/auth/me") {
      return {
        status: 200,
        body: { profile: { id: "profile-1", status: "active" }, needsInviteCode: false, emailVerified: true, currentMatchId: null, email: null },
      };
    }
    if (method === "GET" && path === "/api/friends") {
      return { status: 200, body: { friends: [ANN], incoming: [], outgoing: [], blocked: [] } };
    }
    if (method === "GET" && path === "/api/social") return { status: 200, body: { ...challenges(), unread: {} } };
    if (method === "GET" && path === "/api/replays/shared") return { status: 200, body: { shared: [] } };
    if (method === "GET" && path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    if (method === "GET" && path === "/api/decks") {
      return { status: 200, body: { catalogVersion: "v1", decks: [], trios: [], limits: { decks: 20, trios: 10, nameLength: 40 } } };
    }
    if (method === "GET" && path === "/api/catalog") return { status: 200, body: { version: "v1", defs: {} } };
    if (method === "POST" && path === "/api/presence") return { status: 204 };
    return undefined;
  });
}

/** How many times each social read was made so far: the friends list, and the social poll's one read (R3210000). */
function reads(server: StubServer): { friends: number; social: number } {
  return {
    friends: server.to("GET", "/api/friends").length,
    social: server.to("GET", "/api/social").length,
  };
}

beforeEach(() => {
  window.localStorage.clear();
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-polling" }));
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  resetVisibility();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("#936 the social reads' cadences", () => {
  it("the friends list is read once a heartbeat, and the challenges and unread counts each social poll", () => {
    expect(FRIENDS_POLL_MS).toBe(PRESENCE_HEARTBEAT_MS);
    expect(SOCIAL_POLL_MS).toBeGreaterThan(CHAT_POLL_MS);
    expect(SOCIAL_POLL_MS).toBeLessThan(FRIENDS_POLL_MS);
  });

  it("Play together reads its friends list again only every FRIENDS_POLL_MS, not at the chat's pace", async () => {
    const server = serve();
    render(<TogetherRoute />);
    await settle({ advance: 0 });
    const first = reads(server);
    expect(first.friends).toBe(1);
    expect(first.social).toBeGreaterThan(0);

    await settle({ advance: SOCIAL_POLL_MS });
    const second = reads(server);
    expect(second.friends, "not again within a social poll").toBe(first.friends);
    expect(second.social).toBeGreaterThan(first.social);

    await settle({ advance: FRIENDS_POLL_MS - SOCIAL_POLL_MS });
    expect(reads(server).friends).toBe(first.friends + 1);
  });

  it("#1831 a friends button reads the friends list only while its panel is open", async () => {
    const server = serve();
    render(
      <>
        <ChallengeToast />
        <FriendsButton placement="game" />
      </>,
    );
    await settle({ advance: 0 });
    const closed = reads(server);
    expect(closed.social).toBe(1);
    expect(screen.getByTestId(friendsTestid.button)).toBeInTheDocument();

    await settle({ advance: FRIENDS_POLL_MS * 2 });
    expect(reads(server).friends, "a board being played asks for no friends list").toBe(0);
    expect(reads(server).social).toBe(closed.social + Math.floor((FRIENDS_POLL_MS * 2) / SOCIAL_POLL_MS));

    fireEvent.click(screen.getByTestId(friendsTestid.button));
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(1);
    await settle({ advance: SOCIAL_POLL_MS });
    expect(reads(server).friends, "the open panel keeps the heartbeat's pace").toBe(1);
    await settle({ advance: FRIENDS_POLL_MS - SOCIAL_POLL_MS });
    expect(reads(server).friends).toBe(2);

    fireEvent.click(screen.getByTestId(friendsTestid.close));
    expect(screen.queryByTestId(friendsTestid.panel)).toBeNull();
    await settle({ advance: FRIENDS_POLL_MS * 2 });
    expect(reads(server).friends, "closed again, the list is not read").toBe(2);
  });

  it("#1831 the toast and Play together share one social read per SOCIAL_POLL_MS", async () => {
    const server = serve();
    render(
      <>
        <ChallengeToast />
        <TogetherRoute />
      </>,
    );
    await settle({ advance: 0 });
    expect(reads(server)).toMatchObject({ social: 1 });

    await settle({ advance: SOCIAL_POLL_MS });
    expect(reads(server)).toMatchObject({ social: 2 });
  });

  it("R3210000 the social poll sends one request a tick and neither of the two reads it replaced", async () => {
    const server = serve();
    render(
      <>
        <ChallengeToast />
        <TogetherRoute />
      </>,
    );
    await settle({ advance: 0 });
    expect(reads(server).social).toBe(1);
    expect(server.to("GET", "/api/challenges")).toHaveLength(0);
    expect(server.to("GET", "/api/chat/unread")).toHaveLength(0);

    await settle({ advance: SOCIAL_POLL_MS });
    expect(reads(server).social).toBe(2);
    expect(server.to("GET", "/api/challenges")).toHaveLength(0);
    expect(server.to("GET", "/api/chat/unread")).toHaveLength(0);
  });

  it("the challenge toast reads the challenges every SOCIAL_POLL_MS, not at the chat's pace", async () => {
    const server = serve();
    render(<ChallengeToast />);
    await settle({ advance: 0 });
    const first = reads(server).social;
    expect(first).toBe(1);

    await settle({ advance: CHAT_POLL_MS });
    expect(reads(server).social).toBe(first);
    await settle({ advance: SOCIAL_POLL_MS - CHAT_POLL_MS });
    expect(reads(server).social).toBe(first + 1);
    expect(reads(server).friends, "the toast never reads the friends list").toBe(0);
  });
});

describe("R2125000 the friends list opens on its last answer", () => {
  it("R2125000 the friends list draws its kept answer at once, then the read's", async () => {
    const kept: Friend = { ...ANN, profileId: "f-kept", username: "Kept#9" };
    rememberRead("tok-polling", FRIENDS_PATH, { friends: [kept], incoming: [], outgoing: [], blocked: [] });
    serve();
    render(<TogetherRoute />);

    expect(screen.getAllByTestId(togetherTestid.friendRow)[0]).toHaveTextContent("Kept");

    await settle({ advance: 0 });
    const rows = screen.getAllByTestId(togetherTestid.friendRow);
    expect(rows).toHaveLength(1);
    expect(rows[0]).toHaveTextContent("Ann");
  });

  it("R2125000 another account's kept friends never draw", async () => {
    const kept: Friend = { ...ANN, profileId: "f-kept", username: "Kept#9" };
    rememberRead("tok-other", FRIENDS_PATH, { friends: [kept], incoming: [], outgoing: [], blocked: [] });
    serve();
    render(<TogetherRoute />);

    expect(screen.queryAllByTestId(togetherTestid.friendRow)).toHaveLength(0);

    await settle({ advance: 0 });
    expect(screen.getAllByTestId(togetherTestid.friendRow)[0]).toHaveTextContent("Ann");
  });
});

describe("R3205000 a hidden tab's social reads", () => {
  it("R3205000 the unread counts and the challenges pause while the tab is hidden and read once when it shows", async () => {
    const server = serve();
    render(<ChallengeToast />);
    await settle({ advance: 0 });
    expect(reads(server)).toMatchObject({ social: 1 });

    setVisibility("hidden");
    await settle({ advance: 3 * SOCIAL_POLL_MS });
    expect(reads(server)).toMatchObject({ social: 1 });

    setVisibility("visible");
    await settle({ advance: 0 });
    expect(reads(server)).toMatchObject({ social: 2 });

    await settle({ advance: SOCIAL_POLL_MS });
    expect(reads(server)).toMatchObject({ social: 3 });
  });

  it("R3205000 Play together's friends list pauses while the tab is hidden and reads once when it shows", async () => {
    const server = serve();
    render(<TogetherRoute />);
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(1);

    setVisibility("hidden");
    await settle({ advance: 3 * FRIENDS_POLL_MS });
    expect(reads(server).friends).toBe(1);

    setVisibility("visible");
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(2);

    await settle({ advance: FRIENDS_POLL_MS });
    expect(reads(server).friends).toBe(3);
  });

  it("R3205000 an open friends panel's list pauses while the tab is hidden and reads once when it shows", async () => {
    const server = serve();
    render(
      <>
        <ChallengeToast />
        <FriendsButton placement="game" />
      </>,
    );
    await settle({ advance: 0 });
    fireEvent.click(screen.getByTestId(friendsTestid.button));
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(1);

    setVisibility("hidden");
    await settle({ advance: 3 * FRIENDS_POLL_MS });
    expect(reads(server).friends).toBe(1);

    setVisibility("visible");
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(2);

    await settle({ advance: FRIENDS_POLL_MS });
    expect(reads(server).friends).toBe(3);
  });

  it("R3205000 a challenge of the tab's own keeps the social poll reading while hidden, and it pauses once none is open", async () => {
    let open: Challenges = { incoming: [], outgoing: [OUTGOING] };
    const server = serve(() => open);
    render(<ChallengeToast />);
    await settle({ advance: 0 });
    const first = reads(server).social;

    setVisibility("hidden");
    await settle({ advance: 2 * SOCIAL_POLL_MS });
    expect(reads(server).social, "the room expires and the follow rides this poll").toBe(first + 2);

    open = { incoming: [], outgoing: [] };
    await settle({ advance: SOCIAL_POLL_MS });
    expect(reads(server).social).toBe(first + 3);

    await settle({ advance: 3 * SOCIAL_POLL_MS });
    expect(reads(server).social, "no challenge left open: the poll paused").toBe(first + 3);
  });
});

describe("R3214000 the social socket stands in for the polls", () => {
  let sockets: FakeSocket[] = [];
  const socket = (): FakeSocket => must(sockets.at(-1), "the tab's social socket");
  /** The socket opens and the server sends its first read: it is live. */
  const goLive = async (challenges: Challenges = { incoming: [], outgoing: [] }): Promise<void> => {
    socket().open();
    socket().deliver({ type: "social", ...challenges, unread: {} });
    await settle({ advance: 0 });
  };

  beforeEach(() => {
    const fake = fakeSockets();
    sockets = fake.sockets;
    setSocialSocketFactory(fake.factory);
  });

  afterEach(() => {
    setSocialSocketFactory(null);
  });

  it("R3214000 while the connection is up no social poll is sent, and a pushed challenge shows", async () => {
    const server = serve();
    render(<ChallengeToast />);
    await settle({ advance: 0 });
    expect(sockets).toHaveLength(1);
    expect(new URL(socket().url).pathname).toBe("/ws/social");
    expect(tokenFromProtocols(socket().protocols)).toBe("tok-polling");
    const before = reads(server).social;
    expect(screen.queryByTestId(challengeToastTestid.toast)).toBeNull();

    await goLive({ incoming: [INCOMING], outgoing: [] });
    expect(screen.getByTestId(challengeToastTestid.toast)).toHaveAttribute("data-challenge-id", INCOMING.challengeId);

    await settle({ advance: 5 * SOCIAL_POLL_MS });
    expect(reads(server).social).toBe(before);
    expect(server.to("GET", "/api/challenges")).toHaveLength(0);
    expect(server.to("GET", "/api/chat/unread")).toHaveLength(0);

    socket().deliver({ type: "social", incoming: [], outgoing: [], unread: {} });
    await settle({ advance: 0 });
    expect(screen.queryByTestId(challengeToastTestid.toast), "the pushed read took it away").toBeNull();
  });

  it("R3214000 after the connection drops the social poll reads at once and resumes its pace", async () => {
    const server = serve();
    render(<ChallengeToast />);
    await settle({ advance: 0 });
    await goLive();
    await settle({ advance: 2 * SOCIAL_POLL_MS });
    const before = reads(server).social;

    socket().drop(1006);
    await settle({ advance: 0 });
    expect(reads(server).social, "the drop reads at once").toBe(before + 1);
    await settle({ advance: SOCIAL_POLL_MS });
    expect(reads(server).social, "then at its pace").toBe(before + 2);
    expect(sockets, "the socket reopened on its backoff").toHaveLength(2);

    await goLive();
    await settle({ advance: 3 * SOCIAL_POLL_MS });
    expect(reads(server).social, "live again: no poll").toBe(before + 2);
  });

  it("R3214000 a friends push reads the open friends list again", async () => {
    const server = serve();
    render(<TogetherRoute />);
    await settle({ advance: 0 });
    await goLive();
    const before = reads(server).friends;
    expect(before).toBe(1);

    socket().deliver({ type: "friends" });
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(before + 1);
  });

  it("R3214000 a friends push reads the open friends panel's list again, and a closed panel reads none", async () => {
    const server = serve();
    render(
      <>
        <ChallengeToast />
        <FriendsButton placement="game" />
      </>,
    );
    await settle({ advance: 0 });
    await goLive();
    socket().deliver({ type: "friends" });
    await settle({ advance: 0 });
    expect(reads(server).friends, "closed, the panel reads nothing").toBe(0);

    fireEvent.click(screen.getByTestId(friendsTestid.button));
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(1);
    socket().deliver({ type: "friends" });
    await settle({ advance: 0 });
    expect(reads(server).friends).toBe(2);
  });
});
