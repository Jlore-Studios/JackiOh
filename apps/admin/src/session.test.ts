// Where the session lives (this tab's sessionStorage, never localStorage), the e2e fixture session that
// only a development or e2e build reads, and renewal ahead of expiry.

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AUTH_MESSAGES } from "./auth.ts";
import {
  bootSession,
  dueForRenewal,
  E2E_SESSION_STORAGE_KEY,
  forgetSession,
  parseSession,
  SESSION_STORAGE_KEY,
  storeSession,
  useSession,
} from "./session.ts";
import { fakeFetch, jwt, stubSupabaseEnv, SUPABASE, tokens } from "./test/fakes.ts";
import {
  AUTH_SESSION_REFRESH_MARGIN_SECONDS,
  AUTH_SESSION_RENEWAL_FLOOR_SECONDS,
  MS_PER_SECOND,
} from "./wire.ts";

const MARGIN_MS = AUTH_SESSION_REFRESH_MARGIN_SECONDS * MS_PER_SECOND;
const FLOOR_MS = AUTH_SESSION_RENEWAL_FLOOR_SECONDS * MS_PER_SECOND;
const NOW = Date.UTC(2026, 10, 4, 12);
const HOUR_MS = 3_600_000;

const stored = { accessToken: "tab-token", refreshToken: "tab-refresh", expiresAt: NOW + HOUR_MS };

describe("parseSession", () => {
  it("is a session only with a non-empty token, and never crashes on what storage holds", () => {
    expect(parseSession(JSON.stringify(stored))).toEqual(stored);
    expect(parseSession(JSON.stringify({ accessToken: "t" }))).toEqual({ accessToken: "t", refreshToken: null, expiresAt: null });
    for (const raw of [null, "", "{", "null", "[]", '"t"', '{"accessToken":""}', '{"accessToken":7}']) {
      expect(parseSession(raw), String(raw)).toBeNull();
    }
  });
});

describe("the tab's session", () => {
  it("is kept in sessionStorage and nowhere in localStorage", () => {
    storeSession(stored);
    expect(JSON.parse(window.sessionStorage.getItem(SESSION_STORAGE_KEY) ?? "null")).toEqual(stored);
    expect(window.localStorage.length).toBe(0);
    expect(bootSession()).toEqual(stored);
  });

  it("is not read from localStorage", () => {
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(stored));
    expect(bootSession()).toBeNull();
  });

  it("is forgotten on sign-out, with the e2e fixture's in a development or e2e build", () => {
    storeSession(stored);
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "fixture" }));
    forgetSession();
    expect(window.sessionStorage.getItem(SESSION_STORAGE_KEY)).toBeNull();
    expect(window.localStorage.getItem(E2E_SESSION_STORAGE_KEY)).toBeNull();
  });
});

describe("the e2e fixture session", () => {
  it("a development or e2e build boots on it, ahead of the tab's, as a session that never renews", () => {
    for (const mode of ["development", "test"]) {
      vi.stubEnv("MODE", mode);
      storeSession(stored);
      window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "fixture-token" }));
      expect(bootSession(), mode).toEqual({ accessToken: "fixture-token", refreshToken: null, expiresAt: null });
    }
  });

  it("a development build ignores a malformed one", () => {
    vi.stubEnv("MODE", "development");
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, "{not json");
    expect(bootSession()).toBeNull();
  });

  it("a production build ignores it, and leaves it alone on sign-out", () => {
    vi.stubEnv("MODE", "production");
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "fixture-token" }));
    expect(bootSession()).toBeNull();
    storeSession(stored);
    expect(bootSession()).toEqual(stored);
    forgetSession();
    expect(window.localStorage.getItem(E2E_SESSION_STORAGE_KEY)).not.toBeNull();
  });
});

describe("dueForRenewal", () => {
  it("renews a renewable session within the server's margin of expiry, at most once per floor", () => {
    const session = { accessToken: "a", refreshToken: "r", expiresAt: NOW + MARGIN_MS };
    expect(dueForRenewal(session, NOW, null)).toBe(true);
    expect(dueForRenewal(session, NOW - 1, null)).toBe(false);
    expect(dueForRenewal(session, NOW, NOW - FLOOR_MS + 1)).toBe(false);
    expect(dueForRenewal(session, NOW, NOW - FLOOR_MS)).toBe(true);
    expect(dueForRenewal({ ...session, refreshToken: null }, NOW, null)).toBe(false);
    expect(dueForRenewal({ ...session, expiresAt: null }, NOW, null)).toBe(false);
  });
});

describe("useSession", () => {
  const refresh = `POST ${SUPABASE}/auth/v1/token?grant_type=refresh_token`;
  const renewed = jwt({ aal: "aal2", sub: "renewed" });

  it("renews the session the server's margin before it expires, and keeps the renewed one in the tab", async () => {
    stubSupabaseEnv();
    vi.useFakeTimers({ now: NOW });
    const fake = fakeFetch({ [refresh]: tokens(renewed, "refresh-2") });
    storeSession(stored);
    const { result } = renderHook(() => useSession());
    expect(result.current.session).toEqual(stored);

    await act(() => vi.advanceTimersByTimeAsync(HOUR_MS - MARGIN_MS - 1));
    expect(fake.calls).toHaveLength(0);
    await act(() => vi.advanceTimersByTimeAsync(1));
    expect(fake.calls).toHaveLength(1);
    expect(fake.calls[0]?.body).toEqual({ refresh_token: "tab-refresh" });
    expect(result.current.session?.accessToken).toBe(renewed);
    expect(parseSession(window.sessionStorage.getItem(SESSION_STORAGE_KEY))?.refreshToken).toBe("refresh-2");
  });

  it("renews first when a request asks for the token close to expiry", async () => {
    stubSupabaseEnv();
    vi.useFakeTimers({ now: NOW });
    const fake = fakeFetch({ [refresh]: tokens(renewed, "refresh-2") });
    storeSession({ ...stored, expiresAt: NOW + MARGIN_MS });
    const { result } = renderHook(() => useSession());
    let token = "";
    await act(async () => {
      token = await result.current.token();
    });
    expect(token).toBe(renewed);
    expect(fake.calls).toHaveLength(1);
  });

  it("ends the session, with a notice, when the provider refuses the renewal", async () => {
    stubSupabaseEnv();
    vi.useFakeTimers({ now: NOW });
    fakeFetch({ [refresh]: { status: 400 }, [`POST ${SUPABASE}/auth/v1/logout?scope=local`]: { status: 204 } });
    storeSession({ ...stored, expiresAt: NOW + MARGIN_MS });
    const { result } = renderHook(() => useSession());
    await act(() => vi.advanceTimersByTimeAsync(0));
    expect(result.current.session).toBeNull();
    expect(result.current.notice).toBe(AUTH_MESSAGES.sessionEnded);
    expect(window.sessionStorage.getItem(SESSION_STORAGE_KEY)).toBeNull();
  });

  it("never renews the e2e fixture's session", async () => {
    vi.stubEnv("MODE", "development");
    vi.useFakeTimers({ now: NOW });
    const fake = fakeFetch();
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "fixture-token" }));
    const { result } = renderHook(() => useSession());
    await act(() => vi.advanceTimersByTimeAsync(HOUR_MS * 2));
    expect(await result.current.token()).toBe("fixture-token");
    expect(fake.calls).toHaveLength(0);
  });
});
