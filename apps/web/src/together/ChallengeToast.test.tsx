// #936 S8 B57: `ChallengeToast`, mounted once in the shell. While signed in it reads
// `GET /api/challenges`; an incoming challenge shows a toast naming the challenger, whose Accept goes
// to `/play` with the room code filled in (the lobby's room link, R767: it fills the form and never
// joins by itself) and whose Decline declines the challenge.
//
// SPEC S8 names no test id for the toast, so its buttons are found by their role and name.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { forgetRoomLink, readRoomLink } from "../net/roomLink.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import type { Challenge, PlayerRef } from "../net/social.ts";
import { must, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { ChallengeToast } from "./ChallengeToast.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

/** SPEC S8 gives the toast's poll no interval of its own; a first read comes well inside this. */
const SOON = { timeout: 8_000 } as const;
const TEST_MS = 15_000;

const ME: PlayerRef = { profileId: "profile-1", username: "Me#1" };
const ANN: PlayerRef = { profileId: "f-ann", username: "Ann#2" };
const BOB: PlayerRef = { profileId: "f-bob", username: "Bob#3" };

const INCOMING: Challenge = {
  challengeId: "ch-1",
  from: ANN,
  to: ME,
  roomCode: "HJK234",
  mode: "bo1",
  expiresAt: "2099-01-01T00:00:00.000Z",
};

const OUTGOING: Challenge = {
  challengeId: "ch-2",
  from: ME,
  to: BOB,
  roomCode: "MNP567",
  mode: "random",
  expiresAt: "2099-01-01T00:00:00.000Z",
};

function meBody() {
  return {
    profile: { id: ME.profileId, status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId: null,
    email: "me@example.test",
  };
}

type Challenges = { incoming: Challenge[]; outgoing: Challenge[] };

function serve(challenges: Challenges, special?: (call: Call) => Reply | undefined): StubServer {
  return stubServer((call) => {
    const answer = special?.(call);
    if (answer !== undefined) return answer;
    if (call.path === "/api/auth/me") return { status: 200, body: meBody() };
    if (call.method === "GET" && call.path === "/api/challenges") return { status: 200, body: challenges };
    if (call.method === "POST" && call.path.endsWith("/decline")) {
      challenges.incoming = challenges.incoming.filter((challenge) => !call.path.includes(challenge.challengeId));
      return { status: 204 };
    }
    if (call.method === "POST" && call.path === "/api/presence") return { status: 204 };
    if (call.method === "GET" && call.path === "/api/friends") {
      return { status: 200, body: { friends: [], incoming: [], outgoing: [], blocked: [] } };
    }
    if (call.method === "GET" && call.path === "/api/chat/unread") return { status: 200, body: { unread: {} } };
    if (call.method === "GET" && call.path === "/api/duos/live") return { status: 200, body: { matchId: null } };
    return undefined;
  });
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "tok-toast" }));
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

function acceptButtons(): HTMLElement[] {
  return screen.queryAllByRole("button", { name: /^accept/i });
}

beforeEach(() => {
  window.localStorage.clear();
  forgetRoomLink();
  resetUsernamePromptForTests();
  at("/decks");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  forgetRoomLink();
  window.localStorage.clear();
});

describe("B57 the challenge toast", () => {
  it(
    "B57 an incoming challenge shows a toast naming the challenger, with Accept and Decline",
    async () => {
      signedIn();
      serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      expect(await screen.findByRole("button", { name: /^accept/i }, SOON)).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /^decline/i })).toBeInTheDocument();
      expect(document.body).toHaveTextContent(ANN.username);
    },
    TEST_MS,
  );

  it(
    "B57 Accept goes to /play with the room code filled in",
    async () => {
      signedIn();
      serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      fireEvent.click(await screen.findByRole("button", { name: /^accept/i }, SOON));
      await waitFor(() => {
        expect(window.location.pathname).toBe("/play");
      }, SOON);
      expect(readRoomLink()?.code).toBe(INCOMING.roomCode);
    },
    TEST_MS,
  );

  it(
    "B57 Accept neither declines the challenge nor joins the room by itself",
    async () => {
      signedIn();
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      fireEvent.click(await screen.findByRole("button", { name: /^accept/i }, SOON));
      await waitFor(() => {
        expect(window.location.pathname).toBe("/play");
      }, SOON);
      expect(server.to("POST", "/api/challenges/ch-1/decline")).toHaveLength(0);
      expect(server.calls.some((call) => call.method === "POST" && call.path.startsWith("/api/rooms"))).toBe(false);
    },
    TEST_MS,
  );

  it(
    "B57 Decline declines the challenge and the toast goes, leaving the player where they are",
    async () => {
      signedIn();
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      fireEvent.click(await screen.findByRole("button", { name: /^decline/i }, SOON));
      await waitFor(() => {
        expect(server.to("POST", "/api/challenges/ch-1/decline")).toHaveLength(1);
      }, SOON);
      await waitFor(() => {
        expect(acceptButtons()).toHaveLength(0);
      }, SOON);
      expect(window.location.pathname).toBe("/decks");
      expect(readRoomLink()).toBeNull();
    },
    TEST_MS,
  );

  it(
    "B57 no incoming challenge, no toast: the player's own outgoing challenge raises none",
    async () => {
      signedIn();
      const server = serve({ incoming: [], outgoing: [OUTGOING] });
      render(<ChallengeToast />);
      await waitFor(() => {
        expect(server.to("GET", "/api/challenges").length).toBeGreaterThan(0);
      }, SOON);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 100));
      });
      expect(acceptButtons()).toHaveLength(0);
      expect(document.body).not.toHaveTextContent(BOB.username);
    },
    TEST_MS,
  );

  it(
    "B57 signed out: the toast reads no challenges and shows nothing",
    async () => {
      const server = serve({ incoming: [INCOMING], outgoing: [] });
      render(<ChallengeToast />);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 500));
      });
      expect(server.to("GET", "/api/challenges")).toHaveLength(0);
      expect(acceptButtons()).toHaveLength(0);
      expect(document.body).not.toHaveTextContent(ANN.username);
    },
    TEST_MS,
  );

  it(
    "B57 a refused read of the challenges shows no toast",
    async () => {
      signedIn();
      const server = serve({ incoming: [INCOMING], outgoing: [] }, (call) =>
        call.path === "/api/challenges" ? { status: 500, body: { error: { code: "internal", message: "boom" } } } : undefined,
      );
      render(<ChallengeToast />);
      await waitFor(() => {
        expect(server.to("GET", "/api/challenges").length).toBeGreaterThan(0);
      }, SOON);
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 100));
      });
      expect(acceptButtons()).toHaveLength(0);
    },
    TEST_MS,
  );

  it(
    "B57 the shell mounts the toast: an incoming challenge shows on any signed-in screen, once",
    async () => {
      signedIn();
      serve({ incoming: [INCOMING], outgoing: [] });
      at("/");
      render(<App />);
      expect(await screen.findByRole("button", { name: /^accept/i }, SOON)).toBeInTheDocument();
      expect(acceptButtons()).toHaveLength(1);
      expect(must(acceptButtons()[0], "accept")).toBeInTheDocument();
    },
    TEST_MS,
  );
});
