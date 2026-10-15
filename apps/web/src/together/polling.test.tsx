// #936 review (finding 8): how often the social screens ask the server again. The friends list is the
// social layer's dearest read, so it is read again only every FRIENDS_POLL_MS (a heartbeat, the pace a
// status changes at), and on a match screen only while the board rail's chat list is open; the unread
// counts and the challenges every SOCIAL_POLL_MS; an open conversation keeps CHAT_POLL_MS
// (`ChatPanel.test.tsx`).
//
// The server is a stubbed `fetch` (`test/stubServer.ts`) and time is vitest's fake clock.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CHAT_POLL_MS, PRESENCE_HEARTBEAT_MS } from "@jackioh/server-config";

import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { Friend } from "../net/social.ts";
import { stubServer, type StubServer } from "../test/stubServer.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";
import { FRIENDS_POLL_MS, SOCIAL_POLL_MS } from "./config.ts";
import { InMatchChat } from "./InMatchChat.tsx";
import { TogetherRoute } from "./TogetherRoute.tsx";

const ANN: Friend = {
  profileId: "f-ann",
  username: "Ann#2",
  status: "online",
  matchId: null,
  watchable: false,
  since: "2026-10-01T00:00:00.000Z",
};

function serve(): StubServer {
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
    if (method === "GET" && path === "/api/challenges") return { status: 200, body: { incoming: [], outgoing: [] } };
    if (method === "GET" && path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
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

/** Advance fake time by `ms`, then let the stubbed round trips and React's updates settle. */
async function settle(ms = 0): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
    for (let tick = 0; tick < 50; tick += 1) await Promise.resolve();
  });
}

/** How many times each social read was made so far. */
function reads(server: StubServer): { friends: number; challenges: number; unread: number } {
  return {
    friends: server.to("GET", "/api/friends").length,
    challenges: server.to("GET", "/api/challenges").length,
    unread: server.to("GET", "/api/chat/unread").length,
  };
}

beforeEach(() => {
  window.localStorage.clear();
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-polling" }));
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
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
    await settle();
    const first = reads(server);
    expect(first.friends).toBe(1);
    expect(first.challenges).toBeGreaterThan(0);
    expect(first.unread).toBeGreaterThan(0);

    await settle(SOCIAL_POLL_MS);
    const second = reads(server);
    expect(second.friends, "not again within a social poll").toBe(first.friends);
    expect(second.challenges).toBeGreaterThan(first.challenges);
    expect(second.unread).toBeGreaterThan(first.unread);

    await settle(FRIENDS_POLL_MS - SOCIAL_POLL_MS);
    expect(reads(server).friends).toBe(first.friends + 1);
  });

  it("the match board's chat toggle reads no friends list while closed, and reads it only while its list is open", async () => {
    const server = serve();
    render(<InMatchChat />);
    await settle();
    const closed = reads(server);
    expect(closed.unread).toBe(1);

    await settle(FRIENDS_POLL_MS * 2);
    expect(reads(server).friends, "a board being played asks for no friends list").toBe(0);
    expect(reads(server).unread).toBe(closed.unread + Math.floor((FRIENDS_POLL_MS * 2) / SOCIAL_POLL_MS));

    fireEvent.click(screen.getByRole("button", { name: /chat/i }));
    await settle();
    expect(reads(server).friends).toBe(1);
    await settle(SOCIAL_POLL_MS);
    expect(reads(server).friends, "the open list keeps the heartbeat's pace").toBe(1);
    await settle(FRIENDS_POLL_MS - SOCIAL_POLL_MS);
    expect(reads(server).friends).toBe(2);

    fireEvent.click(screen.getByRole("button", { name: /chat/i }));
    await settle(FRIENDS_POLL_MS * 2);
    expect(reads(server).friends, "closed again, the list is not read").toBe(2);
  });

  it("the challenge toast reads the challenges every SOCIAL_POLL_MS, not at the chat's pace", async () => {
    const server = serve();
    render(<ChallengeToast />);
    await settle();
    const first = reads(server).challenges;
    expect(first).toBe(1);

    await settle(CHAT_POLL_MS);
    expect(reads(server).challenges).toBe(first);
    await settle(SOCIAL_POLL_MS - CHAT_POLL_MS);
    expect(reads(server).challenges).toBe(first + 1);
    expect(reads(server).friends, "the toast never reads the friends list").toBe(0);
  });
});
