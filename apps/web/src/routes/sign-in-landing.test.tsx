// Issue #479: a successful sign-in lands on the main menu (the landing page), and a failed one keeps
// the player on the sign-in screen, its sentence shown and the form as they left it. That holds for a
// sign-in a protected screen sent the player to as well: it lands on the main menu, not back there.
//
// Asserted through the real `App` from `main.tsx` (loaded with `await import`, because main.tsx
// mounts itself outside vitest), with the auth provider and the API answered by a stubbed `fetch`.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid, loginTestid } from "../auth/testids.ts";
import { SIGN_IN_FAILED_MESSAGE } from "../net/auth.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";

const { App } = await import("../main.tsx");

const API = "http://localhost:8787";
const SLOW = { timeout: 5_000 } as const;

const EMAIL = "player@example.test";
const PASSWORD = "secret-password-1";
const WRONG_PASSWORD = "not-the-password";

const SIGNED_IN = {
  access_token: "signed-in-token",
  refresh_token: "signed-in-refresh",
  expires_in: 3600,
  user: { email_confirmed_at: "2026-09-20T00:00:00Z" },
};

function json(status: number, body?: unknown): Response {
  const text = body === undefined ? "" : JSON.stringify(body);
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: String(status),
    headers: new Headers({ "content-type": "application/json" }),
    json: () => (text === "" ? Promise.reject(new SyntaxError("empty")) : Promise.resolve(JSON.parse(text))),
    text: () => Promise.resolve(text),
    clone: () => json(status, body),
  } as unknown as Response;
}

function me(status: "pending" | "active") {
  return {
    profile: { id: "p1", status },
    needsInviteCode: status === "pending",
    emailVerified: true,
    currentMatchId: null,
    currentSeriesId: null,
    email: EMAIL,
  };
}

type Stub = { passwords: string[] };

/**
 * The provider takes `PASSWORD` and refuses anything else as GoTrue does (`invalid_credentials`);
 * the API says who the session is. Everything else a screen asks for never answers.
 */
function stubFetch(status: "pending" | "active" = "active"): Stub {
  const stub: Stub = { passwords: [] };
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const url = typeof input === "string" ? input : String(input);
      if (url.includes("/auth/v1/token")) {
        const body = JSON.parse(String(init?.body ?? "{}")) as { password?: string };
        stub.passwords.push(body.password ?? "");
        return Promise.resolve(
          body.password === PASSWORD
            ? json(200, SIGNED_IN)
            : json(400, { error_code: "invalid_credentials", msg: "Invalid login credentials" }),
        );
      }
      if (url === `${API}/api/auth/me`) return Promise.resolve(json(200, me(status)));
      return new Promise<Response>(() => {});
    }),
  );
  return stub;
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

function signIn(password: string): void {
  fireEvent.change(screen.getByTestId(loginTestid.email), { target: { value: EMAIL } });
  fireEvent.change(screen.getByTestId(loginTestid.password), { target: { value: password } });
  fireEvent.submit(screen.getByTestId(loginTestid.form));
}

/** The landing page's corner "Sign in", as a player presses it. */
async function pressSignInOnTheMainMenu(): Promise<void> {
  fireEvent.click(await screen.findByTestId(landingTestid.signIn, undefined, SLOW));
  await screen.findByTestId(loginTestid.form, undefined, SLOW);
  expect(window.location.pathname).toBe(paths.login);
}

/** A failed sign-in: still on `/login`, the sentence shown, the form as the player left it. */
async function expectStillSigningIn(password: string): Promise<void> {
  expect((await screen.findByTestId(loginTestid.error, undefined, SLOW)).textContent).toBe(SIGN_IN_FAILED_MESSAGE);
  expect(window.location.pathname).toBe(paths.login);
  expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "signIn");
  expect(screen.getByTestId(loginTestid.email)).toHaveValue(EMAIL);
  expect(screen.getByTestId(loginTestid.password)).toHaveValue(password);
  expect(screen.getByTestId(loginTestid.submit)).toBeEnabled();
  expect(readSession()).toBeNull();
}

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", "https://project.supabase.co");
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "sb_publishable_test");
  window.localStorage.clear();
  window.sessionStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  window.localStorage.clear();
  window.sessionStorage.clear();
});

describe("issue #479 a successful sign-in lands on the main menu", () => {
  it("signing in from the main menu lands back on it, signed in", async () => {
    stubFetch();
    at(paths.landing);
    render(<App />);
    await pressSignInOnTheMainMenu();

    signIn(PASSWORD);
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.landing);
    }, SLOW);
    expect(readSession()?.accessToken).toBe(SIGNED_IN.access_token);
    // The main menu itself, and it knows the player is signed in.
    await waitFor(() => {
      expect(screen.getByTestId(landingTestid.root)).toHaveAttribute("data-account", "signed-in");
    }, SLOW);
    expect(screen.getByTestId(landingTestid.account)).toBeInTheDocument();
    expect(screen.queryByTestId(loginTestid.form)).toBeNull();
  });

  it("a sign-in opened straight from its address lands on the main menu too", async () => {
    stubFetch();
    at(paths.login);
    render(<App />);
    await screen.findByTestId(loginTestid.form, undefined, SLOW);

    signIn(PASSWORD);
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.landing);
    }, SLOW);
    expect(await screen.findByTestId(landingTestid.root, undefined, SLOW)).toBeInTheDocument();
  });

  it("a pending account lands on the main menu as well, and its online screens still lead to the code screen", async () => {
    stubFetch("pending");
    at(paths.landing);
    render(<App />);
    await pressSignInOnTheMainMenu();

    signIn(PASSWORD);
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.landing);
    }, SLOW);
    fireEvent.click(await screen.findByTestId(landingTestid.playOnline, undefined, SLOW));
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.invite);
    }, SLOW);
  });
});

describe("issue #479 a failed sign-in stays on the sign-in screen", () => {
  it("shows its sentence above the form and keeps what was typed; the next try signs in", async () => {
    const stub = stubFetch();
    at(paths.landing);
    render(<App />);
    await pressSignInOnTheMainMenu();

    signIn(WRONG_PASSWORD);
    await expectStillSigningIn(WRONG_PASSWORD);
    // Still there once everything has settled: nothing moves the player on afterwards.
    await new Promise((resolve) => setTimeout(resolve, 200));
    expect(window.location.pathname).toBe(paths.login);
    expect(screen.getByTestId(loginTestid.form)).toBeInTheDocument();

    // The form is still the way in.
    signIn(PASSWORD);
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.landing);
    }, SLOW);
    expect(stub.passwords).toEqual([WRONG_PASSWORD, PASSWORD]);
  });
});

describe("issue #479 a sign-in a protected screen sent the player to", () => {
  it.each([paths.decks, paths.play, paths.invite, paths.account, paths.leaderboard, paths.match("some-match")])(
    "from %s: a failure stays on the sign-in screen, and the sign-in lands on the main menu, not back there",
    async (protectedPath) => {
      stubFetch();
      at(protectedPath);
      render(<App />);
      await screen.findByTestId(loginTestid.form, undefined, SLOW);
      expect(window.location.pathname).toBe(paths.login);

      signIn(WRONG_PASSWORD);
      await expectStillSigningIn(WRONG_PASSWORD);

      signIn(PASSWORD);
      await waitFor(() => {
        expect(window.location.pathname).toBe(paths.landing);
      }, SLOW);
      expect(await screen.findByTestId(landingTestid.root, undefined, SLOW)).toBeInTheDocument();
      // And it stays there: nothing sends the player on to the screen that asked.
      await new Promise((resolve) => setTimeout(resolve, 200));
      expect(window.location.pathname).toBe(paths.landing);
    },
  );
});
