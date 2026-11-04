// The account's own sign-in against its Supabase project: password, mailed code, the two-step code,
// renewal and sign-out, each against a fake `fetch`.

import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  AUTH_MESSAGES,
  AuthError,
  EMAIL_CODE_SENT,
  assuranceLevel,
  pendingSecondFactor,
  refreshSession,
  requestEmailCode,
  revokeSession,
  sessionFromTokens,
  signInWithPassword,
  verifyEmailCode,
  verifySecondFactor,
} from "./auth.ts";
import { fakeFetch, jwt, PUBLISHABLE_KEY, stubSupabaseEnv, SUPABASE, tokens } from "./test/fakes.ts";
import { AUTH_PROVIDER_TIMEOUT_SECONDS, MS_PER_SECOND } from "./wire.ts";

const NOW = Date.UTC(2026, 10, 4, 12);
const EXPIRES_IN = 3600;

const AAL1 = jwt({ sub: "user-1", aal: "aal1" });
const AAL2 = jwt({ sub: "user-1", aal: "aal2" });

const failureOf = async (promise: Promise<unknown>): Promise<string> => {
  try {
    await promise;
  } catch (cause) {
    if (cause instanceof AuthError) return cause.failure;
    throw cause;
  }
  throw new Error("it did not fail");
};

beforeEach(() => {
  stubSupabaseEnv();
  vi.useFakeTimers({ now: NOW, toFake: ["Date", "setTimeout", "clearTimeout"] });
});

describe("assuranceLevel", () => {
  it("reads the token's own aal claim, base64url and unpadded, and nothing from a malformed token", () => {
    expect(assuranceLevel(AAL1)).toBe("aal1");
    expect(assuranceLevel(AAL2)).toBe("aal2");
    // A payload whose base64url needs `-` and `_` and no padding.
    expect(assuranceLevel(jwt({ aal: "aal2", name: "??>>~~" }))).toBe("aal2");
    expect(assuranceLevel("not-a-token")).toBeNull();
    expect(assuranceLevel("a.%%%.b")).toBeNull();
    expect(assuranceLevel(jwt({ sub: "no aal" }))).toBeNull();
  });
});

describe("sessionFromTokens", () => {
  it("reads expires_in against this device's clock, and keeps the old refresh token when none is sent", () => {
    expect(sessionFromTokens({ access_token: "a", refresh_token: "r", expires_in: EXPIRES_IN }, null, NOW)).toEqual({
      accessToken: "a",
      refreshToken: "r",
      expiresAt: NOW + EXPIRES_IN * MS_PER_SECOND,
    });
    expect(sessionFromTokens({ access_token: "a" }, "old", NOW)).toEqual({ accessToken: "a", refreshToken: "old", expiresAt: null });
    expect(sessionFromTokens({ refresh_token: "r" }, null, NOW)).toBeNull();
  });
});

describe("signInWithPassword", () => {
  it("posts the password grant with the publishable key and keeps the session the provider sent", async () => {
    const fake = fakeFetch({ [`POST ${SUPABASE}/auth/v1/token?grant_type=password`]: tokens(AAL1, "refresh-1") });
    const session = await signInWithPassword("dev@example.com", "hunter22hunter22");
    expect(session).toEqual({ accessToken: AAL1, refreshToken: "refresh-1", expiresAt: NOW + EXPIRES_IN * MS_PER_SECOND });
    const [call] = fake.calls;
    expect(call?.body).toEqual({ email: "dev@example.com", password: "hunter22hunter22" });
    expect(call?.headers["apikey"]).toBe(PUBLISHABLE_KEY);
    expect(call?.headers["content-type"]).toBe("application/json");
  });

  it("says the same thing for every refused sign-in, and tells a rate limit, a fault and no network apart", async () => {
    const path = `POST ${SUPABASE}/auth/v1/token?grant_type=password`;
    const fake = fakeFetch({ [path]: { status: 400, body: { error_code: "invalid_credentials", msg: "provider words" } } });
    await expect(signInWithPassword("a@b.c", "x")).rejects.toThrow(AUTH_MESSAGES.credentials);
    fake.routes[path] = { status: 429 };
    expect(await failureOf(signInWithPassword("a@b.c", "x"))).toBe("rateLimited");
    fake.routes[path] = { status: 503 };
    expect(await failureOf(signInWithPassword("a@b.c", "x"))).toBe("service");
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new TypeError("Failed to fetch"))));
    expect(await failureOf(signInWithPassword("a@b.c", "x"))).toBe("network");
  });

  it("gives up after the server's AUTH_PROVIDER_TIMEOUT_SECONDS", async () => {
    fakeFetch({ [`POST ${SUPABASE}/auth/v1/token?grant_type=password`]: "never" });
    const failure = failureOf(signInWithPassword("a@b.c", "x"));
    await vi.advanceTimersByTimeAsync(AUTH_PROVIDER_TIMEOUT_SECONDS * MS_PER_SECOND);
    expect(await failure).toBe("network");
  });

  it("asks nothing of a build without a Supabase project", async () => {
    vi.stubEnv("VITE_SUPABASE_URL", "");
    const fake = fakeFetch();
    expect(await failureOf(signInWithPassword("a@b.c", "x"))).toBe("unconfigured");
    expect(fake.calls).toHaveLength(0);
  });
});

describe("the mailed code", () => {
  it("asks for a code for an existing account alone, and reads every refusal but a limit or a fault as sent", async () => {
    const path = `POST ${SUPABASE}/auth/v1/otp`;
    const fake = fakeFetch({ [path]: { status: 200, body: {} } });
    await requestEmailCode("dev@example.com");
    expect(fake.calls[0]?.body).toEqual({ email: "dev@example.com", create_user: false });
    // An address with no account: the same answer as one with (R192).
    fake.routes[path] = { status: 422, body: { error_code: "otp_disabled" } };
    await expect(requestEmailCode("nobody@example.com")).resolves.toBeUndefined();
    fake.routes[path] = { status: 429 };
    expect(await failureOf(requestEmailCode("dev@example.com"))).toBe("rateLimited");
    fake.routes[path] = { status: 500 };
    expect(await failureOf(requestEmailCode("dev@example.com"))).toBe("service");
    expect(EMAIL_CODE_SENT).toMatch(/^If that address has an account/u);
  });

  it("verifies the code as typed, spaces and dashes taken out", async () => {
    const path = `POST ${SUPABASE}/auth/v1/verify`;
    const fake = fakeFetch({ [path]: tokens(AAL1, "refresh-2") });
    const session = await verifyEmailCode("dev@example.com", "123 45-6");
    expect(fake.calls[0]?.body).toEqual({ type: "email", email: "dev@example.com", token: "123456" });
    expect(session.accessToken).toBe(AAL1);
    fake.routes[path] = { status: 403, body: { error_code: "otp_expired" } };
    expect(await failureOf(verifyEmailCode("dev@example.com", "000000"))).toBe("codeInvalid");
  });
});

describe("the two-step code", () => {
  const user = `GET ${SUPABASE}/auth/v1/user`;
  const held = { accessToken: AAL1, refreshToken: "refresh-1", expiresAt: NOW };

  it("asks nothing of an aal2 session", async () => {
    const fake = fakeFetch();
    expect(await pendingSecondFactor({ ...held, accessToken: AAL2 })).toBeNull();
    expect(fake.calls).toHaveLength(0);
  });

  it("finds an aal1 session's verified authenticator in the user object, and no other factor", async () => {
    const fake = fakeFetch({
      [user]: {
        body: {
          id: "user-1",
          factors: [
            { id: "f-unverified", factor_type: "totp", status: "unverified" },
            { id: "f-phone", factor_type: "phone", status: "verified" },
            { id: "f-app", factor_type: "totp", status: "verified" },
          ],
        },
      },
    });
    expect(await pendingSecondFactor(held)).toBe("f-app");
    expect(fake.calls[0]?.headers["authorization"]).toBe(`Bearer ${AAL1}`);
    fake.routes[user] = { body: { id: "user-1", factors: [{ id: "f-unverified", factor_type: "totp", status: "unverified" }] } };
    expect(await pendingSecondFactor(held)).toBeNull();
    fake.routes[user] = { body: { id: "user-1" } };
    expect(await pendingSecondFactor(held)).toBeNull();
    fake.routes[user] = { status: 401 };
    expect(await failureOf(pendingSecondFactor(held))).toBe("sessionEnded");
  });

  it("challenges the factor, verifies the app's code with the session's own token, and keeps the aal2 session", async () => {
    const challenge = `POST ${SUPABASE}/auth/v1/factors/f-app/challenge`;
    const verify = `POST ${SUPABASE}/auth/v1/factors/f-app/verify`;
    const fake = fakeFetch({ [challenge]: { body: { id: "challenge-1", expires_at: 0 } }, [verify]: tokens(AAL2, "refresh-3") });
    const session = await verifySecondFactor(held, "f-app", "654 321");
    expect(session).toEqual({ accessToken: AAL2, refreshToken: "refresh-3", expiresAt: NOW + EXPIRES_IN * MS_PER_SECOND });
    expect(fake.calls.map((call) => `${call.method} ${call.url}`)).toEqual([challenge, verify]);
    expect(fake.calls[1]?.body).toEqual({ challenge_id: "challenge-1", code: "654321" });
    for (const call of fake.calls) expect(call.headers["authorization"]).toBe(`Bearer ${AAL1}`);
  });

  it("says a wrong code is wrong, and an ended session ended", async () => {
    const challenge = `POST ${SUPABASE}/auth/v1/factors/f-app/challenge`;
    const verify = `POST ${SUPABASE}/auth/v1/factors/f-app/verify`;
    const fake = fakeFetch({
      [challenge]: { body: { id: "challenge-1" } },
      [verify]: { status: 422, body: { error_code: "mfa_verification_failed" } },
    });
    expect(await failureOf(verifySecondFactor(held, "f-app", "000000"))).toBe("totpInvalid");
    fake.routes[challenge] = { status: 401 };
    expect(await failureOf(verifySecondFactor(held, "f-app", "000000"))).toBe("sessionEnded");
  });
});

describe("renewal and sign-out", () => {
  const refresh = `POST ${SUPABASE}/auth/v1/token?grant_type=refresh_token`;

  it("renews with the refresh token once however many ask at the same time", async () => {
    const fake = fakeFetch({ [refresh]: tokens(AAL2, "refresh-next") });
    const [first, second] = await Promise.all([refreshSession("refresh-1"), refreshSession("refresh-1")]);
    expect(first).toEqual(second);
    expect(first.refreshToken).toBe("refresh-next");
    expect(fake.calls).toHaveLength(1);
    expect(fake.calls[0]?.body).toEqual({ refresh_token: "refresh-1" });
  });

  it("calls a refused refresh token an ended session", async () => {
    fakeFetch({ [refresh]: { status: 400, body: { error_code: "refresh_token_not_found" } } });
    expect(await failureOf(refreshSession("refresh-gone"))).toBe("sessionEnded");
  });

  it("revokes this session alone, and never throws", async () => {
    const logout = `POST ${SUPABASE}/auth/v1/logout?scope=local`;
    const fake = fakeFetch({ [logout]: { status: 204 } });
    await revokeSession({ accessToken: AAL2, refreshToken: "r", expiresAt: null });
    expect(fake.sent("POST", `${SUPABASE}/auth/v1/logout?scope=local`)[0]?.headers["authorization"]).toBe(`Bearer ${AAL2}`);
    vi.stubGlobal("fetch", vi.fn(() => Promise.reject(new TypeError("offline"))));
    await expect(revokeSession({ accessToken: AAL2, refreshToken: "r", expiresAt: null })).resolves.toBeUndefined();
  });
});
