// #778: the screens that never call the engine paint without waiting for its WebAssembly module, and
// every other screen waits for it in the route table's `EngineGate` (`main.tsx`). `main.tsx` is
// imported once the `wasm/index.ts` mock is in place, so everything it imports reaches it.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { inviteTestid, landingTestid, loginTestid, resetTestid, shellTestid } from "../auth/testids.ts";
import { testid } from "../game/contract.ts";
import { WORKSHOP } from "../game/deckbuilder/testids.ts";
import { decksResponse } from "../game/deckbuilder/testkit.ts";
import { PLAYER_STATS_KEY, PLAYER_STATS_VERSION } from "../stats/config.ts";
import { STAGE_INSTANCE_ID } from "../stats/showcaseStage.ts";
import { dropPlayerStatsCache } from "../stats/store.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { at, jsonResponse, signIn } from "../test/stubServer.ts";

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
const { CosmeticLook } = await import("../collection/CosmeticLook.tsx");
const { WhenEngineReady } = await import("../wasm/WhenEngineReady.tsx");

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

// the stubbed server

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

// the screens that never call the engine

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
    ["/ai", "ai-screen"],
  ])("#778 %s paints while the module is still loading", async (path, testId) => {
    serveAs("active");
    at(path);
    render(<App />);

    expect(await screen.findByTestId(testId, undefined, SLOW)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.engineLoading)).toBeNull();
    expect(screen.queryByTestId(shellTestid.loadFailed)).toBeNull();
    expect(arrived).toBe(false);
  });

  it("#778 / after a played game keeps its screen while the module is still loading, and draws the showcased card once it is in", async () => {
    // A game played (the e2e of R765 plays one, then visits `/`): the landing shows "Your table" with a card
    // whose face the module works out, which it cannot do before it has arrived.
    window.localStorage.setItem(
      PLAYER_STATS_KEY,
      JSON.stringify({
        v: PLAYER_STATS_VERSION,
        games: 1,
        wins: 1,
        losses: 0,
        draws: 0,
        cards: { "core-004": { seen: 1, played: 5, playedInWin: 5 } },
      }),
    );
    dropPlayerStatsCache();
    signIn();
    serveAs("active");
    at("/");
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    render(<App />);

    expect(await screen.findByTestId(landingTestid.stats, undefined, SLOW)).toBeTruthy();
    expect(screen.getByTestId(landingTestid.root)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.loadFailed)).toBeNull();
    // The showcase's code arrives long before the module does (a cached chunk against a 2 MB download).
    await act(async () => {
      await import("../stats/StatShowcase.tsx");
    });
    expect(screen.getByTestId(landingTestid.root)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.loadFailed)).toBeNull();
    expect(screen.queryByTestId(testid.card(STAGE_INSTANCE_ID))).toBeNull();

    await arrive();

    const card = await screen.findByTestId(testid.card(STAGE_INSTANCE_ID), undefined, SLOW);
    expect(card.querySelector(".cf")).not.toBeNull();
    expect(screen.queryByTestId(shellTestid.loadFailed)).toBeNull();
  });

  it("#778 a face drawn where no gate waits for the module stands aside until it is in", async () => {
    // The shop's pack reveal and cosmetic samples are such places (#1577); `WhenEngineReady` and `CosmeticLook`
    // work a face out only when the module has arrived, and throw nothing before.
    const worked: string[] = [];
    const { container } = render(
      <>
        <WhenEngineReady fallback={<p data-testid="waiting">waiting</p>}>
          {() => {
            worked.push(wasm.catalogVersion());
            return <p data-testid="drawn">drawn</p>;
          }}
        </WhenEngineReady>
        <CosmeticLook id="foil-holo-sweep" cardId="core-004" />
      </>,
    );

    expect(screen.getByTestId("waiting")).toBeTruthy();
    expect(screen.queryByTestId("drawn")).toBeNull();
    expect(worked).toEqual([]);
    expect(container.querySelector(".cf")).toBeNull();

    await arrive();

    expect(await screen.findByTestId("drawn")).toBeTruthy();
    expect(screen.queryByTestId("waiting")).toBeNull();
    expect(worked).toHaveLength(1);
    await waitFor(() => {
      expect(container.querySelector(".cf")).not.toBeNull();
    });
  });

  it("#778 /invite shows a pending account the code screen while the module is still loading", async () => {
    signIn();
    serveAs("pending");
    at("/invite");
    render(<App />);

    expect(await screen.findByTestId(inviteTestid.input, undefined, SLOW)).toBeTruthy();
    expect(window.location.pathname).toBe("/invite");
    expect(screen.queryByTestId(shellTestid.engineLoading)).toBeNull();
  });
});

// a screen that calls the engine

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
    signIn();
    serveAs("active");
    at("/decks");
    render(<App />);

    expect(await screen.findByTestId(shellTestid.engineLoading, undefined, SLOW)).toBeTruthy();
    expectSkeleton(within(screen.getByTestId(shellTestid.engineLoading)).getByTestId(shellTestid.loading), "panel");
    expect(screen.queryByTestId(WORKSHOP)).toBeNull();

    await arrive();

    expect(await screen.findByTestId(WORKSHOP, undefined, SLOW)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.engineLoading)).toBeNull();
    expect(screen.queryByTestId(shellTestid.loading)).toBeNull();
    expect(window.location.pathname).toBe("/decks");
  });

  it("R2630001 /decks asks for the engine's module while /api/auth/me is still unanswered", async () => {
    signIn();
    vi.stubGlobal(
      "fetch",
      vi.fn((input: unknown) =>
        String(input).endsWith("/api/auth/me")
          ? new Promise<Response>(() => undefined)
          : Promise.resolve(jsonResponse(404, { error: { code: "not_found", message: "no stub" } })),
      ),
    );
    at("/decks");
    render(<App />);

    await waitFor(() => {
      expect(downloads).toBeGreaterThan(0);
    }, SLOW);
    // The gate is still checking the account: the download did not wait for it.
    expect(screen.getByTestId(shellTestid.loading)).toBeTruthy();
    expect(screen.queryByTestId(shellTestid.engineLoading)).toBeNull();
  });

  it("R2630001 with no session nothing starts: the gate sends the player to sign in", async () => {
    serveAs("active");
    at("/decks");
    render(<App />);

    await waitFor(() => {
      expect(window.location.pathname).toBe("/login");
    }, SLOW);
    expect(downloads).toBe(0);
  });

  it("#778 a module that fails to load shows the error panel, and its retry loads it", async () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    signIn();
    serveAs("active");
    at("/decks");
    render(<App />);
    expect(await screen.findByTestId(shellTestid.engineLoading, undefined, SLOW)).toBeTruthy();
    // The panel can paint before the gate's effect asks `loadWasm()`: a download failed before it
    // asks would hand the gate the next, held one, and the panel would wait forever.
    await waitFor(() => {
      expect(downloads).toBeGreaterThan(0);
    });

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
