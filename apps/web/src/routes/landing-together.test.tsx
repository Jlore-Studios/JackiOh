// #936 S8 B54: since #1577 the landing page offers no Play together call to action (the three
// doors hold no link to `/together`); Play together lives at `/together` behind the gate.
// Also the S8 route table: /together, /watch/:matchId, /duos/:matchId and /dev/duos.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid, loginTestid } from "../auth/testids.ts";
import { paths } from "../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { stubServer } from "../test/stubServer.ts";
import LandingRoute from "./landing.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

const PLAY_TOGETHER = "landing-play-together";
const SLOW = { timeout: 5_000 } as const;

function serve(): void {
  stubServer((call) => {
    if (call.path === "/api/auth/me") {
      return {
        status: 200,
        body: {
          profile: { id: "profile-1", status: "active" },
          needsInviteCode: false,
          emailVerified: true,
          currentMatchId: null,
          email: "player@example.test",
        },
      };
    }
    if (call.method === "GET" && call.path === "/api/challenges") return { status: 200, body: { incoming: [], outgoing: [] } };
    if (call.method === "POST" && call.path === "/api/presence") return { status: 204 };
    return undefined;
  });
}

function signedIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "e2e-token" }));
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

function landing(): HTMLElement {
  return screen.getByTestId(landingTestid.root);
}

beforeEach(() => {
  window.localStorage.clear();
  resetUsernamePromptForTests();
  serve();
  at("/");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("B54 the landing offers no Play together call to action", () => {
  // #1577: the three doors replaced every play call to action; Play together lives at `/together`.
  it("B54 the landing offers no Play together call to action", () => {
    render(<LandingRoute />);
    expect(within(landing()).queryByTestId(PLAY_TOGETHER)).toBeNull();
  });

  it("B54 a signed-in visitor is shown no Play together either", async () => {
    signedIn();
    render(<LandingRoute />);
    await screen.findByTestId(landingTestid.account, undefined, SLOW);
    expect(within(landing()).queryByTestId(PLAY_TOGETHER)).toBeNull();
  });

  it("B54 the doors hold no link to /together", () => {
    render(<LandingRoute />);
    const ctas = within(landing()).getByRole("navigation", { name: "Play" });
    expect(within(ctas).queryByTestId(PLAY_TOGETHER)).toBeNull();
    expect(ctas.querySelector('a[href="/together"]'), "/together has no door").toBeNull();
  });

  it("B54 / renders the doors and no Play together through the route table", async () => {
    render(<App />);
    expect(await screen.findByTestId(landingTestid.play, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(PLAY_TOGETHER)).toBeNull();
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("B54 following the Play door while signed out ends at practice, needing no sign-in", async () => {
    render(<App />);
    fireEvent.click(await screen.findByTestId(landingTestid.play, undefined, SLOW), { button: 0 });
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.practice);
    }, SLOW);
    expect(screen.queryByTestId(loginTestid.form)).toBeNull();
  });

  // #1577: no door leads to /together anymore, so the signed-out bounce is visited directly:
  // Play together keeps the gate Play online asks for.
  it("B54 /together while signed out ends at sign-in, as Play together always did", async () => {
    at(paths.together);
    render(<App />);
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.login);
    }, SLOW);
  });
});

describe("B54 B56 B60 B65 the route table", () => {
  it("B54 paths.together is /together", () => {
    expect(paths.together).toBe("/together");
  });

  it("B56 paths.watch(matchId) is /watch/<matchId>", () => {
    expect(paths.watch("m-42")).toBe("/watch/m-42");
  });

  it("B60 paths.duos(matchId) is /duos/<matchId>", () => {
    expect(paths.duos("dm-7")).toBe("/duos/dm-7");
  });

  it("B65 paths.duosDev is /dev/duos", () => {
    expect(paths.duosDev).toBe("/dev/duos");
  });
});
