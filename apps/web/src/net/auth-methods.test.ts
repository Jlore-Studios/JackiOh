// `net/auth.ts`'s three further ways in (issue #267): the email sign-in link and code (R664), the
// authenticator app's second step (R665) and the OAuth providers (R666). Every one ends in a session
// for an account the server gates exactly as a password sign-in's (§9.4): nothing here can make an
// account active, and these tests only prove what each call sends and how it reads the answers.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { challengeFor, storedVerifiers } from "../auth/pkce.ts";
import {
  AUTH_MESSAGES,
  AuthError,
  assuranceLevel,
  enrollTotp,
  exchangeAuthCode,
  normalizeOneTimeCode,
  oauthAuthorizeUrl,
  oauthProviders,
  removeFactor,
  requestEmailSignIn,
  secondFactorFor,
  verifyEmailCode,
  verifySecondFactor,
} from "./auth.ts";

const URL_ = "https://project.supabase.co";
const KEY = "sb_publishable_test";
const EMAIL = "player@example.com";

type Call = { url: URL; method: string; headers: Headers; body: unknown };
type Answer = { status: number; body?: unknown };

/** Answers each request with `answer(call)`, recording every call. */
function serveBy(answer: (call: Call) => Answer): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const call: Call = {
        url: new URL(String(input)),
        method: (init?.method ?? "GET").toUpperCase(),
        headers: new Headers(init?.headers),
        body: typeof init?.body === "string" ? (JSON.parse(init.body) as unknown) : undefined,
      };
      calls.push(call);
      const { status, body } = answer(call);
      return Promise.resolve(
        new Response(body === undefined ? null : JSON.stringify(body), {
          status,
          headers: { "content-type": "application/json" },
        }),
      );
    }),
  );
  return calls;
}

function serve(status: number, body?: unknown): Call[] {
  return serveBy(() => ({ status, body }));
}

async function refusal(promise: Promise<unknown>): Promise<AuthError> {
  const caught = await promise.then(
    () => null,
    (error: unknown) => error,
  );
  expect(caught).toBeInstanceOf(AuthError);
  return caught as AuthError;
}

/** An unsigned token whose payload claims `claims` (the client never verifies one). */
function tokenWith(claims: Record<string, unknown>): string {
  const encode = (value: unknown): string =>
    btoa(JSON.stringify(value)).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/u, "");
  return `${encode({ alg: "none" })}.${encode(claims)}.signature`;
}

const TOKEN_BODY = { access_token: "access-new", refresh_token: "refresh-new", expires_in: 3600 };

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", URL_);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", KEY);
  window.localStorage.clear();
  window.history.replaceState(null, "", "/login");
});

afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("R664 the email sign-in link and code", () => {
  it("R664 mails a link and code only to an existing account, with a PKCE challenge this browser keeps", async () => {
    const calls = serve(200, {});
    await requestEmailSignIn(EMAIL);
    expect(calls).toHaveLength(1);
    const [call] = calls;
    expect(call?.method).toBe("POST");
    expect(call?.url.pathname).toBe("/auth/v1/otp");
    expect(call?.url.searchParams.get("redirect_to")).toBe(`${window.location.origin}/login`);
    // `create_user: false`: no account is made here, and an unconfirmed one is not signed into.
    expect(call?.body).toEqual({
      email: EMAIL,
      create_user: false,
      code_challenge: expect.stringMatching(/^[A-Za-z0-9_-]{43}$/u),
      code_challenge_method: "s256",
    });
    const kept = storedVerifiers().find((entry) => entry.flow === "magiclink");
    expect(kept).toBeDefined();
    expect(await challengeFor(kept?.verifier ?? "")).toBe((call?.body as { code_challenge: string }).code_challenge);
  });

  it.each([
    ["an unknown or unconfirmed address (otp_disabled)", 422, { error_code: "otp_disabled" }],
    ["the per-address interval", 429, { error_code: "over_email_send_rate_limit" }],
    ["a mail that could not be sent", 500, {}],
    ["the built-in mailer's allow-list", 400, { error_code: "email_address_not_authorized" }],
  ])("R664 answers the same for %s, so the form is no oracle (R192)", async (_name, status, body) => {
    serve(status, body);
    await expect(requestEmailSignIn(EMAIL)).resolves.toBeUndefined();
  });

  it("R664 still says when the typed address is not an address", async () => {
    serve(400, { error_code: "email_address_invalid" });
    expect((await refusal(requestEmailSignIn("nope"))).failure).toBe("invalidEmail");
  });

  it("R664 a code is verified as type email, with spaces and dashes taken out", async () => {
    const calls = serve(200, TOKEN_BODY);
    const { session, secondFactor } = await verifyEmailCode(EMAIL, " 123 456 ");
    expect(secondFactor).toBeNull();
    expect(calls[0]?.url.pathname).toBe("/auth/v1/verify");
    expect(calls[0]?.body).toEqual({ type: "email", email: EMAIL, token: "123456" });
    expect(session.accessToken).toBe("access-new");
    expect(session.refreshToken).toBe("refresh-new");
    expect(normalizeOneTimeCode("12-34 56")).toBe("123456");
  });

  it.each([
    ["a wrong or expired code", 403, { error_code: "otp_expired" }],
    ["an address with no account", 400, { error_code: "user_not_found" }],
    ["an unauthorised answer", 401, {}],
    ["any other refusal", 422, { error_code: "validation_failed" }],
  ])("R664 reads %s as the one sentence (R160)", async (_name, status, body) => {
    serve(status, body);
    const error = await refusal(verifyEmailCode(EMAIL, "123456"));
    expect(error.failure).toBe("codeInvalid");
    expect(error.message).toBe(AUTH_MESSAGES.codeInvalid);
  });

  it("R664 a rate limit is a rate limit (R192), and a code that is not digits is never sent", async () => {
    serve(429, {});
    expect((await refusal(verifyEmailCode(EMAIL, "123456"))).failure).toBe("rateLimited");
    const calls = serve(200, TOKEN_BODY);
    expect((await refusal(verifyEmailCode(EMAIL, "12ab56"))).failure).toBe("codeInvalid");
    expect(calls).toHaveLength(0);
  });

  it("R664 the link's code exchanges with the magic-link verifier, and says which way in it was", async () => {
    serve(200, {});
    await requestEmailSignIn(EMAIL);
    const calls = serve(200, TOKEN_BODY);
    const exchange = await exchangeAuthCode("2b5e0c1a-8f1b-4c4f-9a35-0d8b3c1f2e77");
    expect(exchange.kind).toBe("session");
    if (exchange.kind === "session") expect(exchange.flow).toBe("magiclink");
    expect(calls[0]?.url.searchParams.get("grant_type")).toBe("pkce");
    expect(storedVerifiers().some((entry) => entry.flow === "magiclink")).toBe(false);
  });
});

describe("R665 two-step sign-in with an authenticator app", () => {
  const AAL1 = tokenWith({ sub: "user-1", aal: "aal1" });
  const AAL2 = tokenWith({ sub: "user-1", aal: "aal2" });

  it("R665 reads the token's own assurance level", () => {
    expect(assuranceLevel(AAL1)).toBe("aal1");
    expect(assuranceLevel(AAL2)).toBe("aal2");
    expect(assuranceLevel("not-a-token")).toBeNull();
  });

  it("R665 a token answer says which factor is still needed, without asking anyone", async () => {
    serve(200, {
      access_token: AAL1,
      expires_in: 3600,
      user: { factors: [{ id: "f-app", factor_type: "totp", status: "verified" }] },
    });
    expect((await verifyEmailCode(EMAIL, "123456")).secondFactor).toBe("f-app");
    // Raised already (or no verified factor): nothing is needed.
    serve(200, { access_token: AAL2, user: { factors: [{ id: "f-app", factor_type: "totp", status: "verified" }] } });
    expect((await verifyEmailCode(EMAIL, "123456")).secondFactor).toBeNull();
    serve(200, { access_token: AAL1, user: { factors: [{ id: "f-new", factor_type: "totp", status: "unverified" }] } });
    expect((await verifyEmailCode(EMAIL, "123456")).secondFactor).toBeNull();
  });

  it("R665 an aal2 session needs nothing more, and asks nobody", async () => {
    const calls = serve(500);
    expect(await secondFactorFor({ accessToken: AAL2 })).toBeNull();
    expect(calls).toHaveLength(0);
  });

  it("R665 an aal1 session needs the account's first VERIFIED totp factor", async () => {
    const calls = serve(200, {
      id: "user-1",
      factors: [
        { id: "f-unfinished", factor_type: "totp", status: "unverified" },
        { id: "f-phone", factor_type: "phone", status: "verified" },
        { id: "f-app", factor_type: "totp", status: "verified" },
      ],
    });
    expect(await secondFactorFor({ accessToken: AAL1 })).toBe("f-app");
    expect(calls[0]?.method).toBe("GET");
    expect(calls[0]?.url.pathname).toBe("/auth/v1/user");
    expect(calls[0]?.headers.get("authorization")).toBe(`Bearer ${AAL1}`);

    serve(200, { id: "user-1", factors: [{ id: "f-unfinished", factor_type: "totp", status: "unverified" }] });
    expect(await secondFactorFor({ accessToken: AAL1 })).toBeNull();
  });

  it("R665 a code is a challenge and then its answer, and the session that comes back replaces the old", async () => {
    const calls = serveBy((call) =>
      call.url.pathname.endsWith("/challenge") ? { status: 200, body: { id: "challenge-1" } } : { status: 200, body: { access_token: "aal2-token", expires_in: 3600 } },
    );
    const next = await verifySecondFactor({ accessToken: AAL1, refreshToken: "refresh-old" }, "f-app", "123 456");
    expect(calls.map((call) => `${call.method} ${call.url.pathname}`)).toEqual([
      "POST /auth/v1/factors/f-app/challenge",
      "POST /auth/v1/factors/f-app/verify",
    ]);
    expect(calls[1]?.body).toEqual({ challenge_id: "challenge-1", code: "123456" });
    expect(calls.every((call) => call.headers.get("authorization") === `Bearer ${AAL1}`)).toBe(true);
    expect(next.accessToken).toBe("aal2-token");
    // No refresh token in the answer: the session's own is kept.
    expect(next.refreshToken).toBe("refresh-old");
  });

  it("R665 a wrong code is its own sentence; a code that is not six digits is never sent", async () => {
    serveBy((call) =>
      call.url.pathname.endsWith("/challenge")
        ? { status: 200, body: { id: "challenge-1" } }
        : { status: 422, body: { error_code: "mfa_verification_failed" } },
    );
    const error = await refusal(verifySecondFactor({ accessToken: AAL1 }, "f-app", "123456"));
    expect(error.failure).toBe("mfaCodeInvalid");
    expect(error.message).toBe(AUTH_MESSAGES.mfaCodeInvalid);

    const calls = serve(200, {});
    expect((await refusal(verifySecondFactor({ accessToken: AAL1 }, "f-app", "12345"))).failure).toBe("mfaCodeInvalid");
    expect((await refusal(verifySecondFactor({ accessToken: AAL1 }, "../user", "123456"))).failure).toBe("mfaUnavailable");
    expect(calls).toHaveLength(0);
  });

  it("R665 enrolling clears an unfinished factor first, names the issuer and draws the QR code safely", async () => {
    const svg = '<svg xmlns="http://www.w3.org/2000/svg"><path fill="#000" d="M0 0h1v1H0z"/></svg>';
    const calls = serveBy((call) => {
      if (call.method === "GET") {
        return {
          status: 200,
          body: { id: "user-1", factors: [{ id: "f-old", factor_type: "totp", status: "unverified" }] },
        };
      }
      if (call.method === "DELETE") return { status: 200, body: { id: "f-old" } };
      return {
        status: 200,
        body: { id: "f-new", type: "totp", totp: { qr_code: `data:image/svg+xml;utf-8,${svg}`, secret: "JBSWY3DP", uri: "otpauth://x" } },
      };
    });
    const enrolment = await enrollTotp({ accessToken: AAL1 });
    expect(calls.map((call) => `${call.method} ${call.url.pathname}`)).toEqual([
      "GET /auth/v1/user",
      "DELETE /auth/v1/factors/f-old",
      "POST /auth/v1/factors",
    ]);
    expect(calls[2]?.body).toEqual({ factor_type: "totp", issuer: "JackiOh" });
    expect(enrolment.factorId).toBe("f-new");
    expect(enrolment.secret).toBe("JBSWY3DP");
    // Re-encoded, so the `#` in the SVG cannot end the data URI.
    expect(enrolment.qrCode).toBe(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
  });

  it("R665 a QR code that is not an SVG data URI is not drawn; the key to type stays", async () => {
    serveBy((call) =>
      call.method === "GET"
        ? { status: 200, body: { id: "user-1", factors: [] } }
        : { status: 200, body: { id: "f-new", totp: { qr_code: "javascript:alert(1)", secret: "JBSWY3DP" } } },
    );
    const enrolment = await enrollTotp({ accessToken: AAL1 });
    expect(enrolment.qrCode).toBeNull();
    expect(enrolment.secret).toBe("JBSWY3DP");
  });

  it("R665 a project without TOTP says so in our words", async () => {
    serveBy((call) =>
      call.method === "GET"
        ? { status: 200, body: { id: "user-1", factors: [] } }
        : { status: 422, body: { error_code: "mfa_totp_enroll_not_enabled", msg: "PROVIDER TEXT" } },
    );
    const error = await refusal(enrollTotp({ accessToken: AAL1 }));
    expect(error.failure).toBe("mfaUnavailable");
    expect(error.message).not.toContain("PROVIDER");
  });

  it("R665 removing a factor deletes it with the session's token", async () => {
    const calls = serve(200, { id: "f-app" });
    await removeFactor({ accessToken: AAL2 }, "f-app");
    expect(calls[0]?.method).toBe("DELETE");
    expect(calls[0]?.url.pathname).toBe("/auth/v1/factors/f-app");
    expect(calls[0]?.headers.get("authorization")).toBe(`Bearer ${AAL2}`);

    serve(403, { error_code: "insufficient_aal" });
    expect((await refusal(removeFactor({ accessToken: AAL1 }, "f-app"))).failure).toBe("sessionEnded");
  });
});

describe("R666 OAuth providers, behind configuration", () => {
  it("R666 offers only the providers the public env names, known ones only, once each", () => {
    expect(oauthProviders(undefined)).toEqual([]);
    expect(oauthProviders("")).toEqual([]);
    expect(oauthProviders(" Google, github,google, myspace ,")).toEqual(["google", "github"]);
    // `constructor` and the like are not providers either.
    expect(oauthProviders("constructor,__proto__,toString")).toEqual([]);
  });

  it("R666 offers none when this build has no auth provider", () => {
    vi.stubEnv("VITE_SUPABASE_URL", "");
    expect(oauthProviders("google")).toEqual([]);
  });

  it("R666 the authorize URL carries the provider, the fixed /login return and a PKCE challenge kept here", async () => {
    const url = new URL(await oauthAuthorizeUrl("github"));
    expect(url.origin + url.pathname).toBe(`${URL_}/auth/v1/authorize`);
    expect(url.searchParams.get("provider")).toBe("github");
    expect(url.searchParams.get("redirect_to")).toBe(`${window.location.origin}/login`);
    expect(url.searchParams.get("code_challenge_method")).toBe("s256");
    const kept = storedVerifiers().find((entry) => entry.flow === "oauth");
    expect(await challengeFor(kept?.verifier ?? "")).toBe(url.searchParams.get("code_challenge"));
  });

  it("R666 is never started without PKCE: no challenge, no OAuth (its tokens would come back in a URL)", async () => {
    vi.stubGlobal("crypto", { getRandomValues: (bytes: Uint8Array) => bytes, subtle: undefined });
    expect((await refusal(oauthAuthorizeUrl("google"))).failure).toBe("oauthUnavailable");
  });

  it("R666 the provider's code exchanges with the oauth verifier, and says which way in it was", async () => {
    await oauthAuthorizeUrl("google");
    serve(200, TOKEN_BODY);
    const exchange = await exchangeAuthCode("2b5e0c1a-8f1b-4c4f-9a35-0d8b3c1f2e77");
    expect(exchange.kind === "session" ? exchange.flow : exchange.kind).toBe("oauth");
  });
});
