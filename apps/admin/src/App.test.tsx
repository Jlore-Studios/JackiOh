// The page from boot to the dashboard: each way to sign in, the two-step code, the server's answer to
// `GET /api/admin/me` (the dashboard, its 403 screen, a 401, a fault), and the e2e fixture's session.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "./App.tsx";
import { AUTH_MESSAGES, EMAIL_CODE_SENT } from "./auth.ts";
import { E2E_SESSION_STORAGE_KEY, parseSession, SESSION_STORAGE_KEY } from "./session.ts";
import { SECOND_FACTOR_NOTICE } from "./SignIn.tsx";
import { dashboardRoutes } from "./test/canned.ts";
import { fakeFetch, jwt, stubSupabaseEnv, SUPABASE, tokens, type FakeFetch } from "./test/fakes.ts";
import { testids } from "./testids.ts";

const AAL1 = jwt({ sub: "dev", aal: "aal1" });
const AAL2 = jwt({ sub: "dev", aal: "aal2" });

const PASSWORD = `POST ${SUPABASE}/auth/v1/token?grant_type=password`;
const USER = `GET ${SUPABASE}/auth/v1/user`;
const LOGOUT = `POST ${SUPABASE}/auth/v1/logout?scope=local`;

const stored = () => parseSession(window.sessionStorage.getItem(SESSION_STORAGE_KEY));

const meCalls = (fake: FakeFetch) => fake.sent("GET", "/api/admin/me");

async function signInWithPassword(user: ReturnType<typeof userEvent.setup>): Promise<void> {
  await user.type(screen.getByTestId(testids.signInEmail), "dev@example.com");
  await user.type(screen.getByTestId(testids.signInPassword), "a long password");
  await user.click(screen.getByTestId(testids.signInSubmit));
}

beforeEach(() => {
  stubSupabaseEnv();
});

describe("signing in", () => {
  it("with a password, keeps the session in this tab and opens the dashboard the server allows", async () => {
    const fake = fakeFetch({ ...dashboardRoutes(), [PASSWORD]: tokens(AAL1, "refresh-1"), [USER]: { body: { factors: [] } } });
    const user = userEvent.setup();
    render(<App />);
    expect(screen.getByTestId(testids.signIn)).toBeInTheDocument();
    await signInWithPassword(user);

    expect(await screen.findByTestId(testids.dashboard)).toBeInTheDocument();
    expect(fake.sent("POST", `${SUPABASE}/auth/v1/token?grant_type=password`)[0]?.body).toEqual({
      email: "dev@example.com",
      password: "a long password",
    });
    expect(meCalls(fake)[0]?.headers["authorization"]).toBe(`Bearer ${AAL1}`);
    expect(stored()?.accessToken).toBe(AAL1);
    expect(stored()?.refreshToken).toBe("refresh-1");
    expect(window.localStorage.length).toBe(0);
  });

  it("asks for the authenticator code when the account has one, and keeps only the aal2 session", async () => {
    const fake = fakeFetch({
      ...dashboardRoutes(),
      [PASSWORD]: tokens(AAL1, "refresh-1"),
      [USER]: { body: { factors: [{ id: "f-app", factor_type: "totp", status: "verified" }] } },
      [`POST ${SUPABASE}/auth/v1/factors/f-app/challenge`]: { body: { id: "challenge-1" } },
      [`POST ${SUPABASE}/auth/v1/factors/f-app/verify`]: tokens(AAL2, "refresh-2"),
    });
    const user = userEvent.setup();
    render(<App />);
    await signInWithPassword(user);

    const totp = await screen.findByTestId(testids.signInTotp);
    expect(screen.getByTestId(testids.signInNotice)).toHaveTextContent(SECOND_FACTOR_NOTICE);
    // The aal1 session is held in memory alone, and the server is not asked yet.
    expect(stored()).toBeNull();
    expect(meCalls(fake)).toHaveLength(0);

    await user.type(totp, "123456");
    await user.click(screen.getByTestId(testids.signInTotpSubmit));
    expect(await screen.findByTestId(testids.dashboard)).toBeInTheDocument();
    expect(fake.sent("POST", `${SUPABASE}/auth/v1/factors/f-app/verify`)[0]?.body).toEqual({
      challenge_id: "challenge-1",
      code: "123456",
    });
    expect(meCalls(fake)[0]?.headers["authorization"]).toBe(`Bearer ${AAL2}`);
    expect(stored()?.accessToken).toBe(AAL2);
  });

  it("shows a wrong authenticator code in its own words, and Cancel drops the half-signed-in session", async () => {
    const fake = fakeFetch({
      [PASSWORD]: tokens(AAL1, "refresh-1"),
      [USER]: { body: { factors: [{ id: "f-app", factor_type: "totp", status: "verified" }] } },
      [`POST ${SUPABASE}/auth/v1/factors/f-app/challenge`]: { body: { id: "challenge-1" } },
      [`POST ${SUPABASE}/auth/v1/factors/f-app/verify`]: { status: 422, body: { msg: "provider words" } },
      [LOGOUT]: { status: 204 },
    });
    const user = userEvent.setup();
    render(<App />);
    await signInWithPassword(user);
    await user.type(await screen.findByTestId(testids.signInTotp), "000000");
    await user.click(screen.getByTestId(testids.signInTotpSubmit));
    expect(await screen.findByTestId(testids.signInError)).toHaveTextContent(AUTH_MESSAGES.totpInvalid);
    expect(screen.queryByText(/provider words/u)).toBeNull();

    await user.click(screen.getByTestId(testids.signInCancel));
    expect(screen.getByTestId(testids.signInPassword)).toBeInTheDocument();
    await waitFor(() => {
      expect(fake.sent("POST", `${SUPABASE}/auth/v1/logout?scope=local`)[0]?.headers["authorization"]).toBe(`Bearer ${AAL1}`);
    });
    expect(stored()).toBeNull();
  });

  it("with a mailed code, so an account without a password signs in too", async () => {
    const fake = fakeFetch({
      ...dashboardRoutes(),
      [`POST ${SUPABASE}/auth/v1/otp`]: { body: {} },
      [`POST ${SUPABASE}/auth/v1/verify`]: tokens(AAL1, "refresh-1"),
      [USER]: { body: { factors: [] } },
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByTestId(testids.signInMethodCode));
    expect(screen.queryByTestId(testids.signInPassword)).toBeNull();
    await user.type(screen.getByTestId(testids.signInEmail), "dev@example.com");
    await user.click(screen.getByTestId(testids.signInSendCode));
    expect(await screen.findByTestId(testids.signInNotice)).toHaveTextContent(EMAIL_CODE_SENT);
    expect(fake.sent("POST", `${SUPABASE}/auth/v1/otp`)[0]?.body).toEqual({ email: "dev@example.com", create_user: false });

    await user.type(screen.getByTestId(testids.signInCode), "246 810");
    await user.click(screen.getByTestId(testids.signInVerifyCode));
    expect(await screen.findByTestId(testids.dashboard)).toBeInTheDocument();
    expect(fake.sent("POST", `${SUPABASE}/auth/v1/verify`)[0]?.body).toEqual({
      type: "email",
      email: "dev@example.com",
      token: "246810",
    });
    expect(stored()?.accessToken).toBe(AAL1);
  });

  it("shows a refused sign-in in its own words and keeps nothing", async () => {
    fakeFetch({ [PASSWORD]: { status: 400, body: { error_code: "invalid_credentials", msg: "Invalid login credentials" } } });
    const user = userEvent.setup();
    render(<App />);
    await signInWithPassword(user);
    expect(await screen.findByTestId(testids.signInError)).toHaveTextContent(AUTH_MESSAGES.credentials);
    expect(screen.queryByText(/Invalid login credentials/u)).toBeNull();
    expect(stored()).toBeNull();
  });
});

describe("the server's answer to GET /api/admin/me", () => {
  const signedIn = { accessToken: AAL2, refreshToken: "refresh-1", expiresAt: null };

  it("403: the server's message and Sign out, nothing else", async () => {
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(signedIn));
    const fake = fakeFetch({
      ...dashboardRoutes(),
      "GET /api/admin/me": { status: 403, body: { error: { code: "forbidden", message: "This account is not a dev account." } } },
      [LOGOUT]: { status: 204 },
    });
    const user = userEvent.setup();
    render(<App />);
    const screenOf = await screen.findByTestId(testids.forbidden);
    expect(screen.getByTestId(testids.forbiddenMessage)).toHaveTextContent("This account is not a dev account.");
    expect(within(screenOf).getAllByRole("button")).toEqual([screen.getByTestId(testids.signOut)]);
    expect(screen.queryByTestId(testids.dashboard)).toBeNull();
    // Nothing of the dashboard was read.
    expect(fake.calls.map((call) => call.url)).toEqual(["/api/admin/me"]);

    await user.click(screen.getByTestId(testids.signOut));
    expect(screen.getByTestId(testids.signIn)).toBeInTheDocument();
    expect(window.sessionStorage.getItem(SESSION_STORAGE_KEY)).toBeNull();
    await waitFor(() => {
      expect(fake.sent("POST", `${SUPABASE}/auth/v1/logout?scope=local`)).toHaveLength(1);
    });
  });

  it("401: back to sign-in, with the server's message", async () => {
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(signedIn));
    fakeFetch({
      "GET /api/admin/me": { status: 401, body: { error: { code: "unauthorized", message: "Sign in again." } } },
      [LOGOUT]: { status: 204 },
    });
    render(<App />);
    expect(await screen.findByTestId(testids.signInNotice)).toHaveTextContent("Sign in again.");
    expect(window.sessionStorage.getItem(SESSION_STORAGE_KEY)).toBeNull();
  });

  it("a fault: the server's message, Try again and Sign out", async () => {
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(signedIn));
    const fake = fakeFetch({
      ...dashboardRoutes(),
      "GET /api/admin/me": { status: 500, body: { error: { code: "internal", message: "The database is away." } } },
    });
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByTestId(testids.bootError)).toHaveTextContent("The database is away.");
    expect(screen.getByTestId(testids.signOut)).toBeInTheDocument();
    fake.routes["GET /api/admin/me"] = dashboardRoutes()["GET /api/admin/me"] ?? "never";
    await user.click(screen.getByTestId(testids.retry));
    expect(await screen.findByTestId(testids.dashboard)).toBeInTheDocument();
    expect(meCalls(fake)).toHaveLength(2);
  });

  it("200: Sign out on the dashboard ends the session", async () => {
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(signedIn));
    const fake = fakeFetch({ ...dashboardRoutes(), [LOGOUT]: { status: 204 } });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByTestId(testids.signOut));
    expect(screen.getByTestId(testids.signIn)).toBeInTheDocument();
    expect(screen.queryByTestId(testids.signInNotice)).toBeNull();
    expect(window.sessionStorage.getItem(SESSION_STORAGE_KEY)).toBeNull();
    await waitFor(() => {
      expect(fake.sent("POST", `${SUPABASE}/auth/v1/logout?scope=local`)).toHaveLength(1);
    });
  });
});

describe("the e2e fixture's session", () => {
  it("a development or e2e build signs in with it and asks the server with its token", async () => {
    vi.stubEnv("MODE", "development");
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "fixture-token" }));
    const fake = fakeFetch(dashboardRoutes());
    render(<App />);
    expect(await screen.findByTestId(testids.dashboard)).toBeInTheDocument();
    expect(meCalls(fake)[0]?.headers["authorization"]).toBe("Bearer fixture-token");
    expect(fake.calls.some((call) => call.url.startsWith(SUPABASE))).toBe(false);
  });

  it("a production build ignores it and asks for a sign-in", () => {
    vi.stubEnv("MODE", "production");
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "fixture-token" }));
    const fake = fakeFetch(dashboardRoutes());
    render(<App />);
    expect(screen.getByTestId(testids.signIn)).toBeInTheDocument();
    expect(fake.calls).toHaveLength(0);
  });
});
