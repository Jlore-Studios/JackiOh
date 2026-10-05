// `/login`'s further ways in (issue #267): the email sign-in link and code (R664), the second step
// for an account with an authenticator app (R665) and the OAuth providers (R666). The provider and
// our server are a stubbed `fetch`; nothing leaves the process.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { challengeForRequest, storedVerifiers } from "../auth/pkce.ts";
import { clearConsumedAuthRedirect, recoverySession, releaseRecoverySession } from "../auth/redirect.ts";
import { loginTestid } from "../auth/testids.ts";
import { AUTH_MESSAGES, AUTH_NOTICES } from "../net/auth.ts";
import { paths } from "../net/navigate.ts";
import { readSession, rememberPendingReset } from "../net/session.ts";
import LoginRoute from "./login.tsx";

const URL_ = "https://project.supabase.co";
const KEY = "sb_publishable_test";
const EMAIL = "player@example.com";
const PASSWORD = "correct horse battery";
/** A GoTrue auth code: a UUID. */
const CODE = "3f9c6c1e-5a6b-4c2d-9e8f-0a1b2c3d4e5f";

function base64url(text: string): string {
  return btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function jwt(payload: Record<string, unknown>): string {
  return `${base64url(JSON.stringify({ alg: "none" }))}.${base64url(JSON.stringify(payload))}.sig`;
}

const AAL1 = jwt({ sub: "user-1", email: EMAIL, aal: "aal1", session_id: "s-1" });
const AAL2 = jwt({ sub: "user-1", email: EMAIL, aal: "aal2", session_id: "s-1" });
const VERIFIED_APP = [{ id: "factor-1", factor_type: "totp", status: "verified" }];

type Call = { url: URL; method: string; body: unknown; auth: string | null };
type Answer = { status: number; body?: unknown };

/**
 * Answers by `METHOD /path` (or `/path` for any method); anything else is a 404. `/api/auth/me`
 * answers 200 for a token in `accounts`, 401 for any other, as the real server verifies.
 */
function serve(answers: Record<string, Answer>, accounts: Record<string, string> = {}): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const url = new URL(String(input));
      const method = (init?.method ?? "GET").toUpperCase();
      const auth = new Headers(init?.headers).get("authorization");
      const body: unknown = typeof init?.body === "string" ? JSON.parse(init.body) : undefined;
      calls.push({ url, method, body, auth });
      if (url.pathname === "/api/auth/me") {
        const token = auth?.replace(/^Bearer /u, "") ?? "";
        const email = accounts[token];
        if (email === undefined) return Promise.resolve(Response.json({ error: { code: "unauthorized" } }, { status: 401 }));
        return Promise.resolve(
          Response.json({
            profile: { id: "profile-1", status: "active" },
            needsInviteCode: false,
            emailVerified: true,
            currentMatchId: null,
            email,
          }),
        );
      }
      const answer = answers[`${method} ${url.pathname}`] ?? answers[url.pathname] ?? { status: 404, body: {} };
      return Promise.resolve(
        new Response(answer.body === undefined ? null : JSON.stringify(answer.body), {
          status: answer.status,
          headers: { "content-type": "application/json" },
        }),
      );
    }),
  );
  return calls;
}

function paths_(calls: readonly Call[]): string[] {
  return calls.map((call) => `${call.method} ${call.url.pathname}`);
}

function setField(testId: string, value: string): void {
  fireEvent.change(screen.getByTestId(testId), { target: { value } });
}

function submitForm(): void {
  fireEvent.submit(screen.getByTestId(loginTestid.form));
}

async function settle(): Promise<void> {
  await act(async () => {
    for (let tick = 0; tick < 50; tick += 1) await Promise.resolve();
  });
}

/** The provider's token answer: `token`, and the user's factors when it has any. */
function tokens(token: string, factors?: unknown[]): Answer {
  return {
    status: 200,
    body: {
      access_token: token,
      refresh_token: "refresh-1",
      expires_in: 3600,
      user: { id: "user-1", email: EMAIL, ...(factors === undefined ? {} : { factors }) },
    },
  };
}

/** The authenticator app's challenge and verify, raising the session to `AAL2`. */
const MFA_ANSWERS: Record<string, Answer> = {
  "POST /auth/v1/factors/factor-1/challenge": { status: 200, body: { id: "challenge-1" } },
  "POST /auth/v1/factors/factor-1/verify": { status: 200, body: { access_token: AAL2, refresh_token: "refresh-2", expires_in: 3600 } },
};

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", URL_);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", KEY);
  window.localStorage.clear();
  window.sessionStorage.clear();
  clearConsumedAuthRedirect();
  releaseRecoverySession();
  window.history.replaceState(null, "", "/login");
});

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve, 0));
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  clearConsumedAuthRedirect();
  releaseRecoverySession();
});

describe("R664 signing in with an email code", () => {
  function toEmailCode(): void {
    fireEvent.click(screen.getByTestId(loginTestid.emailCodeStart));
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "emailCode");
  }

  it("R664 mails a link and code, then the typed code signs in", async () => {
    const calls = serve({ "/auth/v1/otp": { status: 200, body: {} }, "/auth/v1/verify": tokens(AAL1) });
    render(<LoginRoute />);
    toEmailCode();
    // No password on this form, and nothing to type a code into before one was asked for.
    expect(screen.queryByTestId(loginTestid.password)).toBeNull();
    expect(screen.queryByTestId(loginTestid.code)).toBeNull();

    setField(loginTestid.email, EMAIL);
    submitForm();
    expect((await screen.findByTestId(loginTestid.notice)).textContent).toBe(AUTH_NOTICES.emailCodeSent);
    expect(calls[0]?.body).toMatchObject({ email: EMAIL, create_user: false });

    setField(loginTestid.code, "123 456");
    submitForm();
    await settle();
    expect(calls.find((call) => call.url.pathname === "/auth/v1/verify")?.body).toEqual({
      type: "email",
      email: EMAIL,
      token: "123456",
    });
    expect(readSession()?.accessToken).toBe(AAL1);
    expect(window.location.pathname).toBe(paths.decks);
  });

  it("R664 an address with no confirmed account reads the same notice (R192)", async () => {
    serve({ "/auth/v1/otp": { status: 422, body: { error_code: "otp_disabled" } } });
    render(<LoginRoute />);
    toEmailCode();
    setField(loginTestid.email, EMAIL);
    submitForm();
    expect((await screen.findByTestId(loginTestid.notice)).textContent).toBe(AUTH_NOTICES.emailCodeSent);
  });

  it("R664 a wrong code is one sentence, and nothing is stored", async () => {
    serve({ "/auth/v1/otp": { status: 200, body: {} }, "/auth/v1/verify": { status: 403, body: { error_code: "otp_expired" } } });
    render(<LoginRoute />);
    toEmailCode();
    setField(loginTestid.email, EMAIL);
    submitForm();
    await screen.findByTestId(loginTestid.code);
    setField(loginTestid.code, "000000");
    submitForm();
    await settle();
    expect(screen.getByTestId(loginTestid.codeError).textContent).toBe(AUTH_MESSAGES.codeInvalid);
    expect(readSession()).toBeNull();
    expect(window.location.pathname).toBe(paths.login);
  });

  it("R664 changing the address goes back to asking for a code for it", async () => {
    serve({ "/auth/v1/otp": { status: 200, body: {} } });
    render(<LoginRoute />);
    toEmailCode();
    setField(loginTestid.email, EMAIL);
    submitForm();
    expect(await screen.findByTestId(loginTestid.code)).toBeTruthy();
    setField(loginTestid.email, "other@example.com");
    expect(screen.queryByTestId(loginTestid.code)).toBeNull();
  });

  it("R664 the link this browser asked for signs it in (its code exchanges only with this browser's verifier)", async () => {
    await challengeForRequest("magiclink");
    const [kept] = storedVerifiers();
    const calls = serve({ "/auth/v1/token": tokens(AAL1) });
    window.history.replaceState(null, "", `/login?code=${CODE}`);
    render(<LoginRoute />);
    await settle();
    expect(calls[0]?.body).toEqual({ auth_code: CODE, code_verifier: kept?.verifier });
    expect(readSession()?.accessToken).toBe(AAL1);
    expect(window.location.pathname).toBe(paths.decks);
    // Kept, so not revoked.
    expect(paths_(calls)).not.toContain("POST /auth/v1/logout");
  });

  it("R664 a sign-in link with no verifier here (asked for elsewhere) signs nothing in", async () => {
    const calls = serve({ "/auth/v1/token": tokens(AAL1) });
    window.history.replaceState(null, "", `/login?code=${CODE}`);
    render(<LoginRoute />);
    await settle();
    expect(calls).toHaveLength(0);
    expect(readSession()).toBeNull();
    expect(window.location.pathname).toBe(paths.login);
  });
});

describe("R665 the second step for an account with an authenticator app", () => {
  function signInWithPassword(): void {
    setField(loginTestid.email, EMAIL);
    setField(loginTestid.password, PASSWORD);
    submitForm();
  }

  it("R665 a password is not enough: the session is held until the app's code raises it to aal2", async () => {
    const calls = serve({ "/auth/v1/token": tokens(AAL1, VERIFIED_APP), ...MFA_ANSWERS });
    render(<LoginRoute />);
    signInWithPassword();
    await settle();

    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "mfa");
    expect(screen.getByTestId(loginTestid.notice).textContent).toBe(AUTH_NOTICES.mfaRequired);
    expect(readSession()).toBeNull();
    expect(window.location.pathname).toBe(paths.login);

    setField(loginTestid.code, "123456");
    submitForm();
    await settle();
    expect(paths_(calls)).toEqual([
      "POST /auth/v1/token",
      "POST /auth/v1/factors/factor-1/challenge",
      "POST /auth/v1/factors/factor-1/verify",
    ]);
    expect(calls[1]?.auth).toBe(`Bearer ${AAL1}`);
    expect(readSession()?.accessToken).toBe(AAL2);
    expect(window.location.pathname).toBe(paths.decks);
  });

  it("R665 a wrong code says so and keeps the step open", async () => {
    serve({
      "/auth/v1/token": tokens(AAL1, VERIFIED_APP),
      "POST /auth/v1/factors/factor-1/challenge": { status: 200, body: { id: "challenge-1" } },
      "POST /auth/v1/factors/factor-1/verify": { status: 422, body: { error_code: "mfa_verification_failed" } },
    });
    render(<LoginRoute />);
    signInWithPassword();
    await settle();
    setField(loginTestid.code, "999999");
    submitForm();
    await settle();
    expect(screen.getByTestId(loginTestid.codeError).textContent).toBe(AUTH_MESSAGES.mfaCodeInvalid);
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "mfa");
    expect(readSession()).toBeNull();
  });

  it("R665 cancelling revokes the held session and stores nothing", async () => {
    const calls = serve({ "/auth/v1/token": tokens(AAL1, VERIFIED_APP), "/auth/v1/logout": { status: 204 } });
    render(<LoginRoute />);
    signInWithPassword();
    await settle();
    fireEvent.click(screen.getByTestId(loginTestid.mfaCancel));
    await settle();
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "signIn");
    expect(calls.filter((call) => call.url.pathname === "/auth/v1/logout").map((call) => call.auth)).toEqual([`Bearer ${AAL1}`]);
    expect(readSession()).toBeNull();
  });

  it("R665 an email code or a sign-in link for such an account asks for the app's code too", async () => {
    await challengeForRequest("magiclink");
    serve({ "/auth/v1/token": tokens(AAL1, VERIFIED_APP), ...MFA_ANSWERS });
    window.history.replaceState(null, "", `/login?code=${CODE}`);
    render(<LoginRoute />);
    await settle();
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "mfa");
    expect(readSession()).toBeNull();
    setField(loginTestid.code, "123456");
    submitForm();
    await settle();
    expect(readSession()?.accessToken).toBe(AAL2);
  });

  it("R665 a reset link for such an account asks for the code before the server is asked whose it is", async () => {
    await challengeForRequest("recovery");
    rememberPendingReset(EMAIL);
    const calls = serve({ "/auth/v1/token": tokens(AAL1, VERIFIED_APP), ...MFA_ANSWERS }, { [AAL2]: EMAIL });
    window.history.replaceState(null, "", `/login?code=${CODE}`);
    render(<LoginRoute />);
    await settle();
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "mfa");
    expect(paths_(calls)).not.toContain("GET /api/auth/me");

    setField(loginTestid.code, "123456");
    submitForm();
    await settle();
    // Checked with the raised token, then held for the reset screen, never stored as the session.
    expect(calls.filter((call) => call.url.pathname === "/api/auth/me").map((call) => call.auth)).toEqual([`Bearer ${AAL2}`]);
    expect(recoverySession()?.session.accessToken).toBe(AAL2);
    expect(window.location.pathname).toBe(paths.resetPassword);
    expect(readSession()).toBeNull();
  });

  it("R665 an account without one signs in at once, as before", async () => {
    serve({ "/auth/v1/token": tokens(AAL1) });
    render(<LoginRoute />);
    signInWithPassword();
    await settle();
    expect(readSession()?.accessToken).toBe(AAL1);
    expect(window.location.pathname).toBe(paths.decks);
  });
});

describe("R666 OAuth providers, behind configuration", () => {
  it("R666 offers no provider until the build names one", () => {
    serve({});
    render(<LoginRoute />);
    expect(screen.queryAllByTestId(loginTestid.oauth)).toHaveLength(0);
  });

  it("R666 offers exactly the named providers, and leaves for one with a PKCE challenge kept here", async () => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "github,google,unknown");
    serve({});
    render(<LoginRoute />);
    const buttons = screen.getAllByTestId(loginTestid.oauth);
    expect(buttons.map((button) => button.getAttribute("data-provider"))).toEqual(["github", "google"]);
    expect(buttons.map((button) => button.textContent)).toEqual(["GitHub", "Google"]);

    fireEvent.click(buttons[1] as HTMLElement);
    await waitFor(() => {
      expect(storedVerifiers().map((entry) => entry.flow)).toContain("oauth");
    });
  });

  it("R666 the provider's code signs this browser in", async () => {
    await challengeForRequest("oauth");
    serve({ "/auth/v1/token": tokens(AAL1) });
    window.history.replaceState(null, "", `/login?code=${CODE}`);
    render(<LoginRoute />);
    await settle();
    expect(readSession()?.accessToken).toBe(AAL1);
    expect(window.location.pathname).toBe(paths.decks);
  });

  it("R666 a provider that sends the player back with an error says so in our words", async () => {
    await challengeForRequest("oauth");
    serve({});
    window.history.replaceState(null, "", "/login?error=access_denied&error_description=PROVIDER-TEXT");
    render(<LoginRoute />);
    expect(screen.getByTestId(loginTestid.oauthError).textContent).toBe(AUTH_MESSAGES.oauthFailed);
    expect(screen.queryByTestId(loginTestid.linkError)).toBeNull();
    expect(document.body.textContent).not.toContain("PROVIDER-TEXT");
  });
});
