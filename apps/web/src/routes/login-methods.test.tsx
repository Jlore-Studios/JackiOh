// `/login`'s further ways in (issue #267): the email sign-in link and code (R664), the second step
// for an account with an authenticator app (R665), a username in place of the email (R1443, issue
// #645), the OAuth providers (R666) and Google's own button (R2983000), against a stubbed `fetch`.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GIS_BUTTON, GIS_SCRIPT_URL, type GisButtonConfiguration, type GisIdConfiguration } from "../auth/googleIdentity.ts";
import { challengeForRequest, storedVerifiers } from "../auth/pkce.ts";
import { clearConsumedAuthRedirect, recoverySession, releaseRecoverySession } from "../auth/redirect.ts";
import { loginTestid } from "../auth/testids.ts";
import { AUTH_MESSAGES, AUTH_NOTICES } from "../net/auth.ts";
import { paths } from "../net/navigate.ts";
import { readSession, rememberPendingReset } from "../net/session.ts";
import { settle } from "../test/settle.ts";
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

/** Answers by `METHOD /path` (or `/path`), else 404; `/api/auth/me` is 200 only for a token in `accounts`. */
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
    expect(window.location.pathname).toBe(paths.landing);
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
    expect(window.location.pathname).toBe(paths.landing);
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
    expect(window.location.pathname).toBe(paths.landing);
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

  it("R665 canceling revokes the held session and stores nothing", async () => {
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
    expect(window.location.pathname).toBe(paths.landing);
  });
});

describe("R1443 signing in with a username", () => {
  const NAME = "Max#3";

  /** Our server's answer to a username sign-in: the session, never the address. */
  function ok(secondFactor: string | null): Answer {
    return {
      status: 200,
      body: { session: { accessToken: AAL1, refreshToken: "refresh-1", expiresIn: 3600 }, emailVerified: true, secondFactor },
    };
  }

  function signInAs(name: string): void {
    setField(loginTestid.email, name);
    setField(loginTestid.password, PASSWORD);
    submitForm();
  }

  it("R1443 a username signs in through our server, and the field says it takes one", async () => {
    const calls = serve({ "POST /api/auth/username-signin": ok(null) });
    render(<LoginRoute />);
    const field = screen.getByLabelText("Email or username");
    expect(field).toHaveAttribute("type", "text");
    // As text it keeps an email field's keyboard, and a phone keyboard rewrites nothing typed.
    expect(field).toHaveAttribute("inputmode", "email");
    expect(field).toHaveAttribute("autocapitalize", "none");
    expect(field).toHaveAttribute("autocorrect", "off");
    expect(field).toHaveAttribute("spellcheck", "false");
    signInAs(` ${NAME} `);
    await settle();
    expect(paths_(calls)).toEqual(["POST /api/auth/username-signin"]);
    expect(calls[0]?.body).toEqual({ username: NAME, password: PASSWORD });
    expect(calls[0]?.auth).toBeNull();
    expect(readSession()?.accessToken).toBe(AAL1);
    expect(readSession()?.refreshToken).toBe("refresh-1");
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("R1443 a refused username reads one sentence of our own and stores nothing", async () => {
    const calls = serve({
      "POST /api/auth/username-signin": { status: 401, body: { error: { code: "unauthorized", message: "server text" } } },
    });
    render(<LoginRoute />);
    signInAs(NAME);
    await settle();
    expect(screen.getByTestId(loginTestid.error).textContent).toBe(AUTH_MESSAGES.usernameCredentials);
    expect(document.body.textContent).not.toContain("server text");
    // A resend of a confirmation needs an address, which a username is not.
    expect(screen.queryByTestId(loginTestid.resend)).toBeNull();
    expect(readSession()).toBeNull();
    expect(paths_(calls)).toEqual(["POST /api/auth/username-signin"]);
  });

  it("R1443 a rate limit says so, never the refusal", async () => {
    serve({
      "POST /api/auth/username-signin": {
        status: 429,
        body: { error: { code: "rate_limited", message: "server text", details: { retryAfterMs: 1000 } } },
      },
    });
    render(<LoginRoute />);
    signInAs(NAME);
    await settle();
    expect(screen.getByTestId(loginTestid.error).textContent).toBe(AUTH_MESSAGES.usernameRateLimited);
    expect(document.body.textContent).not.toContain("server text");
    expect(readSession()).toBeNull();
  });

  it("R1443 an account with an authenticator app is asked for its code after a username sign-in", async () => {
    const calls = serve({ "POST /api/auth/username-signin": ok("factor-1"), ...MFA_ANSWERS });
    render(<LoginRoute />);
    signInAs(NAME);
    await settle();
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "mfa");
    expect(readSession()).toBeNull();

    setField(loginTestid.code, "123456");
    submitForm();
    await settle();
    expect(paths_(calls)).toEqual([
      "POST /api/auth/username-signin",
      "POST /auth/v1/factors/factor-1/challenge",
      "POST /auth/v1/factors/factor-1/verify",
    ]);
    expect(calls[1]?.auth).toBe(`Bearer ${AAL1}`);
    expect(readSession()?.accessToken).toBe(AAL2);
    expect(window.location.pathname).toBe(paths.landing);
  });

  it("R1443 anything with an @ still goes straight to the provider", async () => {
    const calls = serve({ "/auth/v1/token": tokens(AAL1), "POST /api/auth/username-signin": ok(null) });
    render(<LoginRoute />);
    signInAs(EMAIL);
    await settle();
    expect(paths_(calls)).toEqual(["POST /auth/v1/token"]);
    expect(calls[0]?.body).toEqual({ email: EMAIL, password: PASSWORD });
    expect(readSession()?.accessToken).toBe(AAL1);
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
    expect(buttons.map((button) => button.textContent)).toEqual(["Continue with GitHub", "Continue with Google"]);
    // #1890: Google's button carries its own mark, as its branding asks; no other provider's does.
    expect(buttons.map((button) => button.querySelector("svg") !== null)).toEqual([false, true]);

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
    expect(window.location.pathname).toBe(paths.landing);
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

describe("R2983000 Google's own button", () => {
  const CLIENT_ID = "1234567890-abcdefghijklmnop.apps.googleusercontent.com";

  type StubGis = {
    initialize: ReturnType<typeof vi.fn<(config: GisIdConfiguration) => void>>;
    renderButton: ReturnType<typeof vi.fn<(parent: HTMLElement, options: GisButtonConfiguration) => void>>;
  };

  function gisScripts(): HTMLScriptElement[] {
    return [...document.querySelectorAll<HTMLScriptElement>(`script[src="${GIS_SCRIPT_URL}"]`)];
  }

  /** `google.accounts.id` as GIS defines it once loaded; its button is one element in the slot. */
  function stubGis(): StubGis {
    const id: StubGis = {
      initialize: vi.fn<(config: GisIdConfiguration) => void>(),
      renderButton: vi.fn<(parent: HTMLElement, options: GisButtonConfiguration) => void>((parent) => {
        parent.append(document.createElement("iframe"));
      }),
    };
    vi.stubGlobal("google", { accounts: { id } });
    return id;
  }

  /** The configuration GIS was armed with the `nth` time (from 0). */
  function armed(id: StubGis, nth: number): GisIdConfiguration {
    const config = id.initialize.mock.calls[nth]?.[0];
    if (config === undefined) throw new Error(`GIS was not armed ${String(nth + 1)} times`);
    return config;
  }

  async function hex(raw: string): Promise<string> {
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(raw));
    return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  }

  function googleOauthButton(): HTMLElement | undefined {
    return screen.queryAllByTestId(loginTestid.oauth).find((button) => button.getAttribute("data-provider") === "google");
  }

  beforeEach(() => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "github,google");
  });

  afterEach(() => {
    cleanup();
    // A load left in flight settles, so the next test starts with none.
    for (const script of gisScripts()) {
      script.dispatchEvent(new Event("error"));
      script.remove();
    }
  });

  it("R2983000 without a client ID keeps R666's button and loads no GIS script", async () => {
    serve({});
    render(<LoginRoute />);
    await settle();
    expect(googleOauthButton()).toBeDefined();
    expect(screen.queryByTestId(loginTestid.googleButton)).toBeNull();
    expect(gisScripts()).toHaveLength(0);
  });

  it("R2983000 a client ID that is not one, or a build without Google, loads no GIS script either", async () => {
    vi.stubEnv("VITE_GOOGLE_CLIENT_ID", "GOCSPX-not-a-client-id");
    serve({});
    render(<LoginRoute />);
    await settle();
    expect(googleOauthButton()).toBeDefined();
    expect(gisScripts()).toHaveLength(0);
    cleanup();

    vi.stubEnv("VITE_GOOGLE_CLIENT_ID", CLIENT_ID);
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "github");
    render(<LoginRoute />);
    await settle();
    expect(screen.queryByTestId(loginTestid.googleButton)).toBeNull();
    expect(gisScripts()).toHaveLength(0);
  });

  it("R2983000 with a client ID loads GIS and draws Google's button in place of R666's", async () => {
    vi.stubEnv("VITE_GOOGLE_CLIENT_ID", CLIENT_ID);
    serve({});
    render(<LoginRoute />);
    await settle();
    // Loading: one script, and R666's Google button meanwhile.
    expect(gisScripts()).toHaveLength(1);
    expect(googleOauthButton()).toBeDefined();
    const slot = screen.getByTestId(loginTestid.googleButton);
    expect(slot.childElementCount).toBe(0);

    const id = stubGis();
    act(() => {
      gisScripts()[0]?.dispatchEvent(new Event("load"));
    });
    await waitFor(() => {
      expect(id.renderButton).toHaveBeenCalledTimes(1);
    });
    expect(id.renderButton).toHaveBeenCalledWith(slot, GIS_BUTTON);
    const config = armed(id, 0);
    expect(config).toMatchObject({ client_id: CLIENT_ID, context: "signin", ux_mode: "popup", use_fedcm_for_button: true });
    expect(config.nonce).toMatch(/^[0-9a-f]{64}$/u);
    await waitFor(() => {
      expect(googleOauthButton()).toBeUndefined();
    });
    // Only GitHub's R666 button is left beside Google's own.
    expect(screen.getAllByTestId(loginTestid.oauth).map((button) => button.getAttribute("data-provider"))).toEqual(["github"]);
    expect(slot.childElementCount).toBe(1);
    expect(gisScripts()).toHaveLength(1);
  });

  it("R2983000 the credential is exchanged with the raw nonce GIS was given the hash of, and signs in as R666's code does", async () => {
    vi.stubEnv("VITE_GOOGLE_CLIENT_ID", CLIENT_ID);
    const id = stubGis();
    const calls = serve({ "/auth/v1/token": tokens(AAL1) });
    render(<LoginRoute />);
    await waitFor(() => {
      expect(id.renderButton).toHaveBeenCalled();
    });
    // Already loaded: no script is added.
    expect(gisScripts()).toHaveLength(0);
    const config = armed(id, 0);
    const idToken = jwt({ iss: "https://accounts.google.com", sub: "google-1", nonce: config.nonce });

    await act(async () => {
      config.callback({ credential: idToken });
      await Promise.resolve();
    });
    await settle();

    const exchange = calls.find((call) => call.url.pathname === "/auth/v1/token");
    expect(exchange?.method).toBe("POST");
    expect(exchange?.url.searchParams.get("grant_type")).toBe("id_token");
    const body = exchange?.body as { provider: string; id_token: string; nonce: string };
    expect(body.provider).toBe("google");
    expect(body.id_token).toBe(idToken);
    expect(await hex(body.nonce)).toBe(config.nonce);
    expect(body.nonce).not.toBe(config.nonce);

    expect(readSession()?.accessToken).toBe(AAL1);
    expect(window.location.pathname).toBe(paths.landing);
    // The token and the raw nonce went in the body only: no URL holds either.
    for (const url of [...calls.map((call) => call.url.href), window.location.href]) {
      expect(url).not.toContain(body.nonce);
      expect(url).not.toContain(idToken);
    }
    // §9.4: the server's gate decides the account (pending until its invite), not this screen.
    expect(paths_(calls).filter((path) => path.includes(" /api/"))).toEqual([]);
    // The raw nonce is kept in memory only.
    for (const storage of [window.localStorage, window.sessionStorage]) {
      for (let at = 0; at < storage.length; at += 1) {
        const key = storage.key(at) ?? "";
        expect(storage.getItem(key) ?? "", key).not.toContain(body.nonce);
      }
    }
  });

  it("R2983000 a refused exchange says oauthFailed, keeps nothing, and the next attempt gets a new nonce", async () => {
    vi.stubEnv("VITE_GOOGLE_CLIENT_ID", CLIENT_ID);
    const id = stubGis();
    const calls = serve({ "/auth/v1/token": { status: 400, body: { error_code: "bad_oauth_callback", msg: "PROVIDER-TEXT" } } });
    render(<LoginRoute />);
    await waitFor(() => {
      expect(id.renderButton).toHaveBeenCalled();
    });
    const first = armed(id, 0);

    await act(async () => {
      first.callback({ credential: jwt({ sub: "google-1", nonce: first.nonce }) });
      await Promise.resolve();
    });
    await settle();

    expect(screen.getByTestId(loginTestid.error).textContent).toBe(AUTH_MESSAGES.oauthFailed);
    expect(document.body.textContent).not.toContain("PROVIDER-TEXT");
    expect(readSession()).toBeNull();
    expect(window.location.pathname).toBe(paths.login);
    expect(paths_(calls)).toEqual(["POST /auth/v1/token"]);
    await waitFor(() => {
      expect(id.initialize).toHaveBeenCalledTimes(2);
    });
    const second = armed(id, 1);
    expect(second.nonce).toMatch(/^[0-9a-f]{64}$/u);
    expect(second.nonce).not.toBe(first.nonce);
    expect(id.renderButton).toHaveBeenCalledTimes(2);
    // Drawn again in the same slot, which holds one button.
    expect(screen.getByTestId(loginTestid.googleButton).childElementCount).toBe(1);
  });

  it("R2983000 a GIS script that fails to load leaves R666's PKCE redirect", async () => {
    vi.stubEnv("VITE_GOOGLE_CLIENT_ID", CLIENT_ID);
    serve({});
    render(<LoginRoute />);
    await settle();
    expect(gisScripts()).toHaveLength(1);
    act(() => {
      gisScripts()[0]?.dispatchEvent(new Event("error"));
    });
    await waitFor(() => {
      expect(screen.queryByTestId(loginTestid.googleButton)).toBeNull();
    });
    const button = googleOauthButton();
    expect(button).toBeDefined();
    fireEvent.click(button as HTMLElement);
    await waitFor(() => {
      expect(storedVerifiers().map((entry) => entry.flow)).toContain("oauth");
    });
  });

  it("R2983000 GIS that cannot draw its button leaves R666's button", async () => {
    vi.stubEnv("VITE_GOOGLE_CLIENT_ID", CLIENT_ID);
    const id = stubGis();
    id.renderButton.mockImplementation(() => {
      throw new Error("GIS cannot draw here");
    });
    serve({});
    render(<LoginRoute />);
    await waitFor(() => {
      expect(screen.queryByTestId(loginTestid.googleButton)).toBeNull();
    });
    expect(id.renderButton).toHaveBeenCalledTimes(1);
    expect(googleOauthButton()).toBeDefined();
  });
});
