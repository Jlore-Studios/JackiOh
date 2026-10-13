// #778: the screens that never call the engine paint without waiting for its WebAssembly module, and
// every other screen waits for it in the route table's `EngineGate` (`main.tsx`).
//
// `wasm/index.ts` is mocked over the module test/setup.ts loaded: every entry point throws until the
// test lets the module arrive, and `loadWasm()` answers a download the test holds, completes or fails.
// `main.tsx` is imported once the mock is in place, so everything it imports reaches it.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { inviteTestid, landingTestid, loginTestid, resetTestid, shellTestid } from "../auth/testids.ts";
import { WORKSHOP } from "../game/deckbuilder/testids.ts";
import { decksResponse } from "../game/deckbuilder/testkit.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";

/** One `loadWasm()` download, settled by the test. */
type Download = { promise: Promise<void>; resolve: () => void; reject: (cause: Error) => void };

function download(): Download {
  let resolve: () => void = () => undefined;
  let reject: (cause: Error) => void = () => undefined;
  const promise = new Promise<void>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

/** Whether the module has arrived: until it has, every entry point throws. */
let arrived = false;
let current = download();
let downloads = 0;

/** The loader and its probes keep their own meaning; everything else waits for `arrived`. */
const LOADER = new Set(["loadWasm", "loadWasmSync", "wasmLoaded", "whenWasmLoaded"]);

vi.doMock("../wasm/index.ts", async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  const mocked: Record<string, unknown> = { ...actual };
  for (const [name, value] of Object.entries(actual)) {
    if (LOADER.has(name) || typeof value !== "function") continue;
    const real = value as (...args: unknown[]) => unknown;
    mocked[name] = (...args: unknown[]): unknown => {
      if (!arrived) throw new Error(`the engine is not loaded: ${name} was called before loadWasm() resolved`);
      return real(...args);
    };
  }
  mocked["wasmLoaded"] = (): boolean => arrived;
  mocked["loadWasm"] = (): Promise<void> => {
    downloads += 1;
    return arrived ? Promise.resolve() : current.promise;
  };
  return mocked;
});

const wasm = await import("../wasm/index.ts");
const { App } = await import("../main.tsx");

/** The held download arrives: the module's entry points answer from now on. */
async function arrive(): Promise<void> {
  await act(async () => {
    arrived = true;
    current.resolve();
    await current.promise;
  });
}

/** The held download fails; the next `loadWasm()` starts another, as the real one does. */
async function fail(): Promise<void> {
  const failing = current;
  current = download();
  await act(async () => {
    failing.reject(new Error("Failed to fetch /assets/jackioh_wasm_bg.wasm"));
    await failing.promise.catch(() => undefined);
  });
}

// ---------------------------------------------------------------------------------------------
// the stubbed server
// ---------------------------------------------------------------------------------------------

function jsonResponse(status: number, body: unknown): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    text: () => Promise.resolve(JSON.stringify(body)),
  } as unknown as Response;
}

/** Stub `/api/auth/me` and the reads `/decks` makes, for one account status. */
function serveAs(status: "pending" | "active"): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/api/auth/me")) {
        return Promise.resolve(
          jsonResponse(200, {
            profile: { id: "profile-1", status },
            needsInviteCode: status === "pending",
            emailVerified: true,
            currentMatchId: null,
            email: "player@example.test",
          }),
        );
      }
      if (url.endsWith("/api/codes/status")) {
        return Promise.resolve(jsonResponse(200, { redemptionEnabled: true, retryAfterMs: 0 }));
      }
      if (url.endsWith("/api/catalog")) {
        return Promise.resolve(jsonResponse(200, { version: "v1", defs: {} }));
      }
      if (url.endsWith("/api/decks")) return Promise.resolve(jsonResponse(200, decksResponse([], [], "v1")));
      if (url.endsWith("/api/collection")) {
        return Promise.resolve(jsonResponse(200, { catalogVersion: "v1", entries: [] }));
      }
      return Promise.resolve(
        jsonResponse(404, { error: { code: "not_found", message: `no stub for ${url}` } }),
      );
    }),
  );
}

/** The key the M8 specs seed in `cy.visit`'s `onBeforeLoad` (`net/session.ts`). */
function signedIn(token = "e2e-token"): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

/** A lazily-imported route chunk plus a stubbed round trip can outrun testing-library's 1 s default. */
const SLOW = { timeout: 5_000 } as const;

beforeEach(() => {
  arrived = false;
  current = download();
  downloads = 0;
  window.localStorage.clear();
  at("/");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

// ---------------------------------------------------------------------------------------------
// the screens that never call the engine
// ---------------------------------------------------------------------------------------------

describe("#778 the screens that never call the engine", () => {
  it("#778 the mock holds: before the module arrives its entry points throw", () => {
    expect(wasm.wasmLoaded()).toBe(false);
    expect(() => wasm.catalogVersion()).toThrow(/not loaded/u);
  });

  it.each([
    ["/", landingTestid.root],
    ["/login", loginTestid.form],
    ["/reset-password", resetTestid.screen],
    ["/privacy", "privacy-screen"],
    ["/terms", "terms-screen"],
    ["/accessibility", "accessibility-screen"],
  ])("#778 %s paints while the module is still loading", async (path, testId) => {
    serveAs("active");
    at(path);
    render(<App />);

    expect(await screen.findByTestId(testId, undefined, SLOW)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.engineLoading)).toBeNull();
    expect(screen.queryByTestId(shellTestid.loadFailed)).toBeNull();
    expect(arrived).toBe(false);
  });

  it("#778 /invite shows a pending account the code screen while the module is still loading", async () => {
    signedIn();
    serveAs("pending");
    at("/invite");
    render(<App />);

    expect(await screen.findByTestId(inviteTestid.input, undefined, SLOW)).toBeTruthy();
    expect(window.location.pathname).toBe("/invite");
    expect(screen.queryByTestId(shellTestid.engineLoading)).toBeNull();
  });
});

// ---------------------------------------------------------------------------------------------
// a screen that calls the engine
// ---------------------------------------------------------------------------------------------

describe("#778 a screen that calls the engine", () => {
  it("#778 /practice waits in the loading panel until the module is in", async () => {
    serveAs("active");
    at("/practice");
    render(<App />);

    expect(await screen.findByTestId(shellTestid.engineLoading)).toBeTruthy();
    expect(screen.getByTestId(shellTestid.loading).textContent).toBe("Loading…");
    expect(screen.queryByTestId(shellTestid.loadFailed)).toBeNull();
    expect(downloads).toBeGreaterThan(0);
  });

  it("#778 /decks waits in the loading panel, then renders once the module is in", async () => {
    signedIn();
    serveAs("active");
    at("/decks");
    render(<App />);

    expect(await screen.findByTestId(shellTestid.engineLoading, undefined, SLOW)).toBeTruthy();
    expect(screen.queryByTestId(WORKSHOP)).toBeNull();

    await arrive();

    expect(await screen.findByTestId(WORKSHOP, undefined, SLOW)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.engineLoading)).toBeNull();
    expect(window.location.pathname).toBe("/decks");
  });

  it("#778 a module that fails to load shows the error panel, and its retry loads it", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    signedIn();
    serveAs("active");
    at("/decks");
    render(<App />);
    expect(await screen.findByTestId(shellTestid.engineLoading, undefined, SLOW)).toBeTruthy();

    await fail();

    expect(await screen.findByTestId(shellTestid.engineFailed)).toBeTruthy();
    expect(screen.getByTestId(shellTestid.home).getAttribute("href")).toBe("/");
    expect(screen.getByTestId(shellTestid.signOut)).toBeTruthy();

    const before = downloads;
    fireEvent.click(screen.getByTestId(shellTestid.retry));

    expect(await screen.findByTestId(shellTestid.engineLoading)).toBeTruthy();
    expect(downloads).toBeGreaterThan(before);

    await arrive();

    expect(await screen.findByTestId(WORKSHOP, undefined, SLOW)).toBeTruthy();
    await waitFor(() => {
      expect(screen.queryByTestId(shellTestid.engineFailed)).toBeNull();
    });
  });
});
