// #936 S8 B54: the landing page's Play together call to action (`landing-play-together`), a
// secondary CTA beside Play online that is a real link to `/together` and navigates in place, as the
// other CTAs do (routes/landing.test.tsx). Also the S8 route table: /together, /watch/:matchId,
// /duos/:matchId and /dev/duos.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
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

describe("B54 the Play together call to action", () => {
  it("B54 the landing page shows Play together, a real link to /together", () => {
    render(<LandingRoute />);
    const cta = within(landing()).getByTestId(PLAY_TOGETHER);
    const link = cta.closest("a");
    expect(link, "an <a href>, so middle-click and open-in-new-tab work").not.toBeNull();
    expect(link?.getAttribute("href")).toBe("/together");
    expect(cta).toHaveTextContent(/play together/i);
  });

  it("B54 a signed-in visitor is shown Play together too", async () => {
    signedIn();
    render(<LandingRoute />);
    await screen.findByTestId(landingTestid.account, undefined, SLOW);
    expect(within(landing()).getByTestId(PLAY_TOGETHER)).toBeInTheDocument();
  });

  it("B54 Play together sits beside Play online among the landing's play calls to action", () => {
    render(<LandingRoute />);
    const ctas = within(landing()).getByRole("navigation", { name: "Play" });
    expect(within(ctas).getByTestId(landingTestid.playOnline)).toBeInTheDocument();
    expect(within(ctas).getByTestId(PLAY_TOGETHER)).toBeInTheDocument();
  });

  it("B54 Play together is a secondary call to action, not the primary one", () => {
    render(<LandingRoute />);
    const primary = within(landing()).getByTestId(landingTestid.playAi);
    const together = within(landing()).getByTestId(PLAY_TOGETHER);
    expect(together).not.toBe(primary);
    expect(together.className).not.toBe(primary.className);
  });

  it("B54 a plain left click on Play together navigates in place to /together", () => {
    render(<LandingRoute />);
    const notPrevented = fireEvent.click(within(landing()).getByTestId(PLAY_TOGETHER), { button: 0 });
    expect(window.location.pathname).toBe("/together");
    expect(notPrevented).toBe(false);
  });

  it.each([
    ["a ctrl-click", { button: 0, ctrlKey: true }],
    ["a cmd-click", { button: 0, metaKey: true }],
    ["a middle click", { button: 1 }],
  ] as const)("B54 %s on Play together is left to the browser: no in-page navigation", (_name, init) => {
    render(<LandingRoute />);
    const notPrevented = fireEvent.click(within(landing()).getByTestId(PLAY_TOGETHER), init);
    expect(notPrevented).toBe(true);
    expect(window.location.pathname).toBe("/");
  });

  it("B54 / renders Play together through the route table", async () => {
    render(<App />);
    expect(await screen.findByTestId(PLAY_TOGETHER, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe("/");
  });

  it("B54 following Play together while signed out ends at sign-in, as Play online does", async () => {
    render(<App />);
    fireEvent.click(await screen.findByTestId(PLAY_TOGETHER, undefined, SLOW), { button: 0 });
    await waitFor(() => {
      expect(window.location.pathname).toBe("/login");
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
