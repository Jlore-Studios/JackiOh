// #4180: a Chinese-tables chunk failure never bricks a gated or engine-free screen: readiness is
// the engine's module alone, and a flagged card falls back to English until the tables arrive.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { shellTestid } from "../auth/testids.ts";
import { WORKSHOP } from "../game/deckbuilder/testids.ts";
import { decksResponse } from "../game/deckbuilder/testkit.ts";
import { at, jsonResponse, signIn } from "../test/stubServer.ts";

/** One `loadWasm()` download, settled by the test. */
function download(): { promise: Promise<void>; resolve: () => void } {
  let resolve: () => void = () => undefined;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

let arrived = false;
let current = download();

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
  mocked["loadWasm"] = (): Promise<void> => (arrived ? Promise.resolve() : current.promise);
  return mocked;
});

// The tables' chunk fails to download; every other export keeps its meaning.
vi.doMock("../cards/chinese.ts", async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  return {
    ...actual,
    loadCardTables: () => Promise.reject(new Error("the Chinese card tables did not load")),
  };
});

const { App } = await import("../main.tsx");
const { WhenEngineReady } = await import("../wasm/WhenEngineReady.tsx");

function serveAs(): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/api/auth/me")) {
        return Promise.resolve(
          jsonResponse(200, {
            profile: { id: "profile-1", status: "active" },
            needsInviteCode: false,
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
      return Promise.resolve(jsonResponse(404, { error: { code: "not_found", message: `no stub for ${url}` } }));
    }),
  );
}

const SLOW = { timeout: 5_000 } as const;

beforeEach(() => {
  arrived = false;
  current = download();
  window.localStorage.clear();
  at("/");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("#4180 a failed Chinese-tables chunk never bricks the client", () => {
  it("a gated screen renders once the module is in, with no error panel", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    signIn();
    serveAs();
    at("/decks");
    render(<App />);
    expect(await screen.findByTestId(shellTestid.engineLoading, undefined, SLOW)).toBeTruthy();

    await act(async () => {
      arrived = true;
      current.resolve();
      await current.promise;
    });

    expect(await screen.findByTestId(WORKSHOP, undefined, SLOW)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.engineFailed)).toBeNull();
  });

  it("an engine-free face draws once the module is in, tables or no tables", async () => {
    render(
      <WhenEngineReady fallback={<p data-testid="waiting">waiting</p>}>
        {() => <p data-testid="drawn">drawn</p>}
      </WhenEngineReady>,
    );
    expect(screen.getByTestId("waiting")).toBeTruthy();

    await act(async () => {
      arrived = true;
      current.resolve();
      await current.promise;
    });

    expect(await screen.findByTestId("drawn")).toBeTruthy();
    expect(screen.queryByTestId("waiting")).toBeNull();
  });
});
