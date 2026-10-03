// Where a session is kept (SPEC R632, §9.2): in the tab's `sessionStorage`, never in `localStorage`.
//
// `net/session.ts` is the one module that reads, writes and clears the token, so these assert it
// directly over jsdom's two storages. The gate's side (another tab's sign-out leaves this tab
// signed in) is in routes/shell-gate.test.tsx.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  E2E_SESSION_STORAGE_KEY,
  SESSION_STORAGE_KEY,
  clearSession,
  readSession,
  writeSession,
  type Session,
} from "./session.ts";

const TAB: Session = { accessToken: "tab-token", refreshToken: "tab-refresh", expiresAt: 1_900_000_000 };
const OLDER: Session = { accessToken: "older-token", refreshToken: "older-refresh", expiresAt: 1_900_000_001 };
const FIXTURE = { accessToken: "fixture-token" };

function tabRaw(): string | null {
  return window.sessionStorage.getItem(SESSION_STORAGE_KEY);
}

function sharedRaw(key: string): string | null {
  return window.localStorage.getItem(key);
}

beforeEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("R632 a session belongs to its tab", () => {
  it("R632 writes a sign-in to the tab's sessionStorage and nothing to localStorage", () => {
    writeSession(TAB);

    expect(JSON.parse(tabRaw() ?? "null")).toEqual(TAB);
    expect(sharedRaw(SESSION_STORAGE_KEY)).toBeNull();
    expect(window.localStorage.length).toBe(0);
    expect(readSession()).toEqual(TAB);
  });

  it("R632 a sign-in or a renewal deletes the copy an older build left in localStorage", () => {
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(OLDER));

    writeSession(TAB);

    expect(sharedRaw(SESSION_STORAGE_KEY)).toBeNull();
    expect(readSession()).toEqual(TAB);
  });

  it("R632 moves a session an older build left in localStorage into this tab, and deletes it there", () => {
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(OLDER));

    expect(readSession()).toEqual(OLDER);

    expect(JSON.parse(tabRaw() ?? "null")).toEqual(OLDER);
    expect(sharedRaw(SESSION_STORAGE_KEY)).toBeNull();
    expect(readSession()).toEqual(OLDER);
  });

  it("R632 prefers the tab's own session to one in localStorage and leaves the tab's alone", () => {
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(TAB));
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(OLDER));

    expect(readSession()).toEqual(TAB);
    expect(JSON.parse(tabRaw() ?? "null")).toEqual(TAB);
  });

  it("R632 still reads the end-to-end fixtures' session from localStorage when the tab has none", () => {
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify(FIXTURE));
    expect(readSession()).toEqual(FIXTURE);
    // Reading it moves nothing: the fixtures' key is a contract of the fixed specs.
    expect(sharedRaw(E2E_SESSION_STORAGE_KEY)).not.toBeNull();
    expect(tabRaw()).toBeNull();

    writeSession(TAB);
    expect(readSession()).toEqual(TAB);
  });

  it("R632 a sign-out clears the tab's session, any copy in localStorage and the fixtures' session", () => {
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(TAB));
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(OLDER));
    window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify(FIXTURE));

    clearSession();

    expect(tabRaw()).toBeNull();
    expect(sharedRaw(SESSION_STORAGE_KEY)).toBeNull();
    expect(sharedRaw(E2E_SESSION_STORAGE_KEY)).toBeNull();
    expect(readSession()).toBeNull();
  });

  it("R632 treats anything but a stored access token as no session", () => {
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, "not json");
    expect(readSession()).toBeNull();
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "" }));
    expect(readSession()).toBeNull();
    window.sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ refreshToken: "r" }));
    expect(readSession()).toBeNull();
  });

  it("R632 never throws on blocked storage: there is no session, and writing and clearing do nothing", () => {
    const blocked = (): never => {
      throw new DOMException("blocked", "SecurityError");
    };

    vi.spyOn(Storage.prototype, "getItem").mockImplementation(blocked);
    expect(readSession()).toBeNull();
    vi.restoreAllMocks();

    vi.spyOn(Storage.prototype, "setItem").mockImplementation(blocked);
    expect(() => {
      writeSession(TAB);
    }).not.toThrow();
    vi.restoreAllMocks();

    vi.spyOn(Storage.prototype, "removeItem").mockImplementation(blocked);
    expect(() => {
      clearSession();
    }).not.toThrow();
    vi.restoreAllMocks();

    // A private window or blocked site data: merely touching the storage throws.
    const names = ["sessionStorage", "localStorage"] as const;
    const saved = names.map((name) => [name, Object.getOwnPropertyDescriptor(window, name)] as const);
    for (const name of names) Object.defineProperty(window, name, { configurable: true, get: blocked });
    try {
      expect(readSession()).toBeNull();
      expect(() => {
        writeSession(TAB);
        clearSession();
      }).not.toThrow();
    } finally {
      for (const [name, descriptor] of saved) {
        if (descriptor !== undefined) Object.defineProperty(window, name, descriptor);
      }
    }
  });

  it("R632 keeps reading an older build's session when the tab's storage refuses the move", () => {
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(OLDER));
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("quota", "QuotaExceededError");
    });

    expect(readSession()).toEqual(OLDER);
    // Not deleted from the one place it is kept, so the next read can move it.
    expect(sharedRaw(SESSION_STORAGE_KEY)).not.toBeNull();
  });
});
