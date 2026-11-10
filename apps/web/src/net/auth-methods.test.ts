// `net/auth.ts`'s three further ways in (issue #267): the email sign-in link and code (R664), the
// authenticator app's second step (R665) and the OAuth providers (R666), with Google's ID token from
// its own button (R2983000). The server gates each one's account as a password sign-in's (§9.4), so
// these tests prove only what each call sends and reads.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { challengeFor, storedVerifiers } from "../auth/pkce.ts";
import {
  AUTH_MESSAGES,
  AuthError,
  assuranceLevel,
  enrollTotp,
  exchangeAuthCode,
  googleClientId,
  normalizeOneTimeCode,
  oauthAuthorizeUrl,
  oauthProviderLabel,
  oauthProviders,
  removeFactor,
  requestEmailSignIn,
  secondFactorFor,
  signInWithGoogleIdToken,
  verifyEmailCode,
  verifySecondFactor,
  identityLinkUrl,
  listIdentities,
  ROBLOX_AVATAR_IMAGE_HOST,
  unlinkIdentity,
} from "./auth.ts";

const URL_ = "https://project.supabase.co";
const KEY = "sb_publishable_test";
const EMAIL = "player@example.com";

type Call = { url: URL; method: string; headers: Headers; body: unknown };
type Answer = { status: number; body?: unknown };

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
    // The auth server's own answer (supabase/auth's mfa.go, drawn by svgo): the SVG's text, its XML
    // declaration and a comment first, with no data URI around it.
    const svg =
      '<svg width="290" height="290"\n     xmlns="http://www.w3.org/2000/svg"\n     xmlns:xlink="http://www.w3.org/1999/xlink">\n<rect x="40" y="40" width="10" height="10" style="fill:#000000;stroke:none" />\n</svg>\n';
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
        body: {
          id: "f-new",
          type: "totp",
          totp: { qr_code: `<?xml version="1.0"?>\n<!-- Generated by SVGo -->\n${svg}`, secret: "JBSWY3DP", uri: "otpauth://x" },
        },
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

  it("R665 the QR code in supabase-js's shape, an SVG data URI, is drawn too", async () => {
    const svg = '<svg xmlns="http://www.w3.org/2000/svg"><path fill="#000" d="M0 0h1v1H0z"/></svg>';
    for (const qrCode of [`data:image/svg+xml;utf-8,${svg}`, `data:image/svg+xml;utf-8,${encodeURIComponent(`<?xml version="1.0"?>${svg}`)}`]) {
      serveBy((call) =>
        call.method === "GET"
          ? { status: 200, body: { id: "user-1", factors: [] } }
          : { status: 200, body: { id: "f-new", totp: { qr_code: qrCode, secret: "JBSWY3DP" } } },
      );
      const enrolment = await enrollTotp({ accessToken: AAL1 });
      expect(enrolment.qrCode, qrCode).toBe(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
    }
  });

  it("R665 a QR code that is no SVG is not drawn; the key to type stays", async () => {
    for (const qrCode of [
      "javascript:alert(1)",
      '<?xml version="1.0"?>\n<html><script>alert(1)</script></html>',
      "data:text/html,<svg></svg>",
      "",
    ]) {
      serveBy((call) =>
        call.method === "GET"
          ? { status: 200, body: { id: "user-1", factors: [] } }
          : { status: 200, body: { id: "f-new", totp: { qr_code: qrCode, secret: "JBSWY3DP" } } },
      );
      const enrolment = await enrollTotp({ accessToken: AAL1 });
      expect(enrolment.qrCode, qrCode).toBeNull();
      expect(enrolment.secret).toBe("JBSWY3DP");
    }
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

describe("R3147000 Roblox, the projects' custom OIDC provider", () => {
  it("R3147000 is offered by its dashboard identifier, custom:roblox, and by no other custom name", () => {
    expect(oauthProviders("google, Custom:Roblox")).toEqual(["google", "custom:roblox"]);
    expect(oauthProviders("custom:other,roblox,custom:")).toEqual([]);
    expect(oauthProviderLabel("custom:roblox")).toBe("Roblox");
    expect(oauthProviderLabel("email")).toBeNull();
    expect(oauthProviderLabel(null)).toBeNull();
  });

  it("R3147000 leaves for Roblox as R666 leaves for any provider: its name, the /login return and PKCE", async () => {
    const url = new URL(await oauthAuthorizeUrl("custom:roblox"));
    expect(url.origin + url.pathname).toBe(`${URL_}/auth/v1/authorize`);
    expect(url.searchParams.get("provider")).toBe("custom:roblox");
    expect(url.searchParams.get("redirect_to")).toBe(`${window.location.origin}/login`);
    expect(storedVerifiers().map((entry) => entry.flow)).toContain("oauth");
  });
});

describe("R2983000 Google's ID token, from Google's own button", () => {
  const CLIENT_ID = "1234567890-abcdefghijklmnop.apps.googleusercontent.com";
  const ID_TOKEN = tokenWith({ iss: "https://accounts.google.com", sub: "google-1", nonce: "hashed" });

  it("R2983000 names a client ID only when the build offers Google", () => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google");
    expect(googleClientId(`  ${CLIENT_ID} `)).toBe(CLIENT_ID);
    for (const raw of [undefined, "", "   ", "GOCSPX-abcdefghijklmnop", `${CLIENT_ID} extra`, `${CLIENT_ID}.evil.com`, "sb_secret_abc"]) {
      expect(googleClientId(raw), String(raw)).toBeNull();
    }
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "github");
    expect(googleClientId(CLIENT_ID)).toBeNull();
    // No auth provider in this build: no providers, so no Google button either (R666).
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google");
    vi.stubEnv("VITE_SUPABASE_URL", "");
    expect(googleClientId(CLIENT_ID)).toBeNull();
  });

  it("R2983000 exchanges the ID token at grant_type=id_token with the raw nonce, in the body", async () => {
    const calls = serve(200, TOKEN_BODY);
    const { session, secondFactor } = await signInWithGoogleIdToken(ID_TOKEN, "raw-nonce");
    expect(calls).toHaveLength(1);
    const [call] = calls;
    expect(call?.method).toBe("POST");
    expect(`${call?.url.origin ?? ""}${call?.url.pathname ?? ""}`).toBe(`${URL_}/auth/v1/token`);
    expect([...(call?.url.searchParams.entries() ?? [])]).toEqual([["grant_type", "id_token"]]);
    expect(call?.headers.get("apikey")).toBe(KEY);
    expect(call?.body).toEqual({ provider: "google", id_token: ID_TOKEN, nonce: "raw-nonce" });
    expect(session.accessToken).toBe("access-new");
    expect(session.refreshToken).toBe("refresh-new");
    expect(secondFactor).toBeNull();
  });

  it("R2983000 an account with an authenticator app still needs its code (R665)", async () => {
    serve(200, {
      access_token: tokenWith({ sub: "user-1", aal: "aal1" }),
      expires_in: 3600,
      user: { factors: [{ id: "f-app", factor_type: "totp", status: "verified" }] },
    });
    expect((await signInWithGoogleIdToken(ID_TOKEN, "raw-nonce")).secondFactor).toBe("f-app");
  });

  it("R2983000 a refused ID token is oauthFailed in our words; a rate limit and a fault say so", async () => {
    serve(400, { error_code: "bad_oauth_callback", msg: "PROVIDER TEXT: nonces mismatch" });
    const refused = await refusal(signInWithGoogleIdToken(ID_TOKEN, "raw-nonce"));
    expect(refused.failure).toBe("oauthFailed");
    expect(refused.message).toBe(AUTH_MESSAGES.oauthFailed);
    serve(422, { error_code: "validation_failed" });
    expect((await refusal(signInWithGoogleIdToken(ID_TOKEN, "raw-nonce"))).failure).toBe("oauthFailed");
    serve(429, {});
    expect((await refusal(signInWithGoogleIdToken(ID_TOKEN, "raw-nonce"))).failure).toBe("rateLimited");
    serve(500, {});
    expect((await refusal(signInWithGoogleIdToken(ID_TOKEN, "raw-nonce"))).failure).toBe("service");
    serve(200, { user: {} });
    expect((await refusal(signInWithGoogleIdToken(ID_TOKEN, "raw-nonce"))).failure).toBe("oauthFailed");
  });

  it("R2983000 never sends what is not a JWT, or no nonce", async () => {
    const calls = serve(200, TOKEN_BODY);
    for (const [idToken, nonce] of [
      ["not-a-token", "raw-nonce"],
      ["a.b", "raw-nonce"],
      ["a.b.c&grant_type=password", "raw-nonce"],
      [ID_TOKEN, ""],
    ] as const) {
      expect((await refusal(signInWithGoogleIdToken(idToken, nonce))).failure).toBe("oauthFailed");
    }
    expect(calls).toHaveLength(0);
  });
});

describe("R3025000 linking a provider to the signed-in account", () => {
  const SESSION_TOKEN = tokenWith({ sub: "user-1", aal: "aal1" });

  it("R3025000 lists the account's ways in from its user record", async () => {
    const calls = serve(200, {
      identities: [
        { identity_id: "id-email", provider: "email" },
        { identity_id: "id-google", provider: "google" },
        { provider: "github" },
      ],
    });
    expect(await listIdentities(SESSION_TOKEN)).toEqual([
      { id: "id-email", provider: "email" },
      { id: "id-google", provider: "google" },
    ]);
    expect(calls[0]?.url.pathname).toBe("/auth/v1/user");
    expect(calls[0]?.headers.get("authorization")).toBe(`Bearer ${SESSION_TOKEN}`);

    serve(401, { error_code: "bad_jwt" });
    expect((await refusal(listIdentities(SESSION_TOKEN))).failure).toBe("sessionEnded");
  });

  it("R3025000 asks for the provider's page with the session, a challenge of its own flow and the /login return", async () => {
    const calls = serve(200, { url: "https://accounts.google.com/o/oauth2/v2/auth?x=1" });
    expect(await identityLinkUrl("google", SESSION_TOKEN)).toBe("https://accounts.google.com/o/oauth2/v2/auth?x=1");
    const call = calls[0];
    expect(call?.method).toBe("GET");
    expect(call?.url.pathname).toBe("/auth/v1/user/identities/authorize");
    expect(call?.headers.get("authorization")).toBe(`Bearer ${SESSION_TOKEN}`);
    expect(call?.url.searchParams.get("provider")).toBe("google");
    expect(call?.url.searchParams.get("skip_http_redirect")).toBe("true");
    expect(call?.url.searchParams.get("redirect_to")).toBe(`${window.location.origin}/login`);
    const kept = storedVerifiers().find((entry) => entry.flow === "link");
    expect(await challengeFor(kept?.verifier ?? "")).toBe(call?.url.searchParams.get("code_challenge"));
  });

  it("R3025000 a project without linking, or an account linked elsewhere, reads linkRefused and keeps no verifier", async () => {
    for (const code of ["manual_linking_disabled", "identity_already_exists"]) {
      serve(422, { error_code: code });
      expect((await refusal(identityLinkUrl("google", SESSION_TOKEN))).failure, code).toBe("linkRefused");
      expect(storedVerifiers().map((entry) => entry.flow), code).not.toContain("link");
    }
    serve(200, { url: "javascript:alert(1)" });
    expect((await refusal(identityLinkUrl("google", SESSION_TOKEN))).failure).toBe("service");
    serve(429, {});
    expect((await refusal(identityLinkUrl("google", SESSION_TOKEN))).failure).toBe("rateLimited");
  });

  it("R3025000 is never started without PKCE", async () => {
    vi.stubGlobal("crypto", { getRandomValues: (bytes: Uint8Array) => bytes, subtle: undefined });
    const calls = serve(200, { url: "https://accounts.google.com/" });
    expect((await refusal(identityLinkUrl("google", SESSION_TOKEN))).failure).toBe("oauthUnavailable");
    expect(calls).toHaveLength(0);
  });

  it("R3025000 unlinking deletes the identity with the session's token, and the last way in reads lastIdentity", async () => {
    const calls = serve(200, {});
    await unlinkIdentity(SESSION_TOKEN, "id/google");
    expect(calls[0]?.method).toBe("DELETE");
    expect(calls[0]?.url.pathname).toBe("/auth/v1/user/identities/id%2Fgoogle");
    expect(calls[0]?.headers.get("authorization")).toBe(`Bearer ${SESSION_TOKEN}`);

    serve(422, { error_code: "single_identity_not_deletable" });
    expect((await refusal(unlinkIdentity(SESSION_TOKEN, "id-google"))).failure).toBe("lastIdentity");
    serve(500, {});
    expect((await refusal(unlinkIdentity(SESSION_TOKEN, "id-google"))).failure).toBe("service");
  });
});

describe("R3507000 a linked Roblox identity keeps its username and headshot", () => {
  const SESSION_TOKEN = tokenWith({ sub: "user-1", aal: "aal1" });
  const PICTURE = "https://tr.rbxcdn.com/30DAY-AvatarHeadshot-ABC-Png/150/150/AvatarHeadshot/Png/noFilter";

  /** What `listIdentities` keeps of one Roblox identity whose `identity_data` is `data`. */
  async function robloxOf(data: unknown, provider = "custom:roblox"): Promise<unknown> {
    serve(200, { identities: [{ identity_id: "id-roblox", provider, identity_data: data }] });
    return (await listIdentities(SESSION_TOKEN))[0];
  }

  it("R3507000 keeps the trimmed username, the display name and the headshot on Roblox's image host", async () => {
    expect(ROBLOX_AVATAR_IMAGE_HOST).toBe("tr.rbxcdn.com");
    expect(
      await robloxOf({ preferred_username: "  Builderman ", name: "Builder Man", nickname: "Builder", picture: PICTURE, sub: "156" }),
    ).toEqual({ id: "id-roblox", provider: "custom:roblox", username: "Builderman", displayName: "Builder Man", avatarUrl: PICTURE });
    // `nickname` names the person when `name` does not.
    expect(await robloxOf({ preferred_username: "Builderman", nickname: "Builder" })).toEqual({
      id: "id-roblox",
      provider: "custom:roblox",
      username: "Builderman",
      displayName: "Builder",
    });
    expect(await robloxOf({ preferred_username: "Builderman", name: " ", nickname: "Builder" })).toEqual({
      id: "id-roblox",
      provider: "custom:roblox",
      username: "Builderman",
      displayName: "Builder",
    });
  });

  it("R3507000 drops a username that is not a string, or is empty or only spaces", async () => {
    for (const bad of [42, null, true, ["Builderman"], { name: "Builderman" }, "", "   "]) {
      expect(await robloxOf({ preferred_username: bad, picture: PICTURE }), JSON.stringify(bad)).toEqual({
        id: "id-roblox",
        provider: "custom:roblox",
        avatarUrl: PICTURE,
      });
    }
  });

  it("R3507000 drops a headshot that is not an https: URL on Roblox's image host, whatever else it is", async () => {
    const bad: unknown[] = [
      null,
      7,
      "",
      "   ",
      "not a url",
      "/images/headshot.png",
      "//tr.rbxcdn.com/headshot.png",
      "http://tr.rbxcdn.com/headshot.png",
      "javascript:alert(1)",
      "data:image/png;base64,AAAA",
      "https://example.com/headshot.png",
      "https://t0.rbxcdn.com/headshot.png",
      "https://rbxcdn.com/headshot.png",
      "https://evil.tr.rbxcdn.com/headshot.png",
      "https://tr.rbxcdn.com.evil.test/headshot.png",
      "https://tr.rbxcdn.com@evil.test/headshot.png",
    ];
    for (const picture of bad) {
      expect(await robloxOf({ preferred_username: "Builderman", picture }), JSON.stringify(picture)).toEqual({
        id: "id-roblox",
        provider: "custom:roblox",
        username: "Builderman",
      });
    }
  });

  it("R3507000 keeps today's { id, provider } for a Roblox identity with no usable claim, or no identity_data at all", async () => {
    for (const data of [undefined, null, "Builderman", 3, ["x"], {}, { preferred_username: "", picture: "" }]) {
      expect(await robloxOf(data), JSON.stringify(data)).toEqual({ id: "id-roblox", provider: "custom:roblox" });
    }
  });

  it("R3507000 reads no identity_data of another provider", async () => {
    for (const provider of ["google", "email", "github", "custom:other"]) {
      expect(await robloxOf({ preferred_username: "Builderman", name: "Builder Man", picture: PICTURE }, provider), provider).toEqual({
        id: "id-roblox",
        provider,
      });
    }
  });
});
