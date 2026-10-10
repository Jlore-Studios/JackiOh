/** R160: one refusal per endpoint prevents account enumeration (SPEC §9.2, §9.8). */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  AUTH_UNCONFIGURED_MESSAGE,
  AuthError,
  SIGN_IN_FAILED_MESSAGE,
  SIGN_UP_FAILED_MESSAGE,
  authConfig,
  signIn,
  signUp,
} from "./auth.ts";

const URL_ = "https://project.supabase.co";
const KEY = "sb_publishable_test";

function mockFetch(status: number, body: unknown): ReturnType<typeof vi.fn> {
  const fetchMock = vi.fn(async () =>
    Promise.resolve({
      status,
      json: async () => Promise.resolve(body),
    } as Response),
  );
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", URL_);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", KEY);
});

afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("authConfig", () => {
  it("reads the two public VITE_ values and trims a trailing slash", () => {
    vi.stubEnv("VITE_SUPABASE_URL", `${URL_}/`);
    expect(authConfig()).toEqual({ url: URL_, publishableKey: KEY });
  });

  it("is null when the deployment configured no provider", () => {
    vi.stubEnv("VITE_SUPABASE_URL", "");
    expect(authConfig()).toBeNull();
  });
});

describe("signIn", () => {
  it("posts the password grant with the publishable key and returns the session", async () => {
    const fetchMock = mockFetch(200, {
      access_token: "jwt-abc",
      refresh_token: "refresh-abc",
      expires_at: 1_700_000_000,
      user: { email_confirmed_at: "2026-09-20T00:00:00Z" },
    });

    const result = await signIn("player@example.com", "hunter22222");

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(`${URL_}/auth/v1/token?grant_type=password`);
    expect((init.headers as Record<string, string>)["apikey"]).toBe(KEY);

    expect(result.session.accessToken).toBe("jwt-abc");
    expect(result.session.refreshToken).toBe("refresh-abc");
    expect(result.session.expiresAt).toBe(1_700_000_000_000);
    expect(result.emailVerified).toBe(true);
  });

  it("answers identically for a wrong password and an address with no account", async () => {
    mockFetch(400, { error: "invalid_grant", error_description: "Invalid login credentials" });
    await expect(signIn("a@example.com", "wrong-password")).rejects.toThrow(
      SIGN_IN_FAILED_MESSAGE,
    );

    mockFetch(400, { error: "invalid_grant", error_description: "Invalid login credentials" });
    await expect(signIn("nobody@example.com", "any-password")).rejects.toThrow(
      SIGN_IN_FAILED_MESSAGE,
    );
  });

  it("does not relay email_not_confirmed, which would confirm the address is registered", async () => {
    mockFetch(400, { error_code: "email_not_confirmed", msg: "Email not confirmed" });
    const caught = await signIn("real@example.com", "correct-password").catch(
      (error: unknown) => error,
    );
    expect(caught).toBeInstanceOf(AuthError);
    expect((caught as Error).message).toBe(SIGN_IN_FAILED_MESSAGE);
    expect((caught as Error).message).not.toMatch(/confirm/i);
  });

  it("refuses a 200 that carries no access token", async () => {
    mockFetch(200, { access_token: "" });
    await expect(signIn("a@example.com", "p")).rejects.toThrow(SIGN_IN_FAILED_MESSAGE);
  });

  it("says so when the deployment configured no provider", async () => {
    vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "");
    await expect(signIn("a@example.com", "p")).rejects.toThrow(AUTH_UNCONFIGURED_MESSAGE);
  });
});

describe("signUp", () => {
  it("asks for a confirmation and reports that the inbox is the next step", async () => {
    const fetchMock = mockFetch(200, { id: "user-1", email: "new@example.com" });
    const result = await signUp("new@example.com", "hunter22222");

    const url = (fetchMock.mock.calls[0] as [string, RequestInit])[0];
    expect(url).to.contain(`${URL_}/auth/v1/signup`);
    expect(result.needsEmailConfirmation).toBe(true);

    // The confirmation link must return to this deployment, not the provider's default localhost.
    const redirect = new URL(url).searchParams.get("redirect_to");
    expect(redirect, "signup asks for a redirect back to this origin").to.contain(
      window.location.origin,
    );
  });

  /** R160: confirmed and new addresses are indistinguishable. */
  it("reports the same thing for an address that is already registered", async () => {
    mockFetch(200, { id: "00000000-0000-0000-0000-000000000000", identities: [] });
    await expect(signUp("taken@example.com", "hunter22222")).resolves.toEqual({
      needsEmailConfirmation: true,
    });
  });

  it("uses its own failure message, not sign-in's", async () => {
    mockFetch(422, { msg: "Password should be at least 6 characters" });
    await expect(signUp("a@example.com", "x")).rejects.toThrow(SIGN_UP_FAILED_MESSAGE);
  });
});
