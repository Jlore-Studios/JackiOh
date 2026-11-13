// R3206000: moving between screens stops re-reading what was just read. The landing and the collection
// are driven through the real route table with a stubbed server, once as R2125000 had it (every kept
// answer old enough to need a read behind it) and once inside `READ_CACHE_FRESH_MS`, and the requests
// each sends for the kept reads are counted.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
import { collectionTestid } from "../collection/collection.ts";
import { fullCollection, questsHeld, tracksOf } from "../collection/fixtures.ts";
import { __resetWalletForTests } from "../economy/wallet.ts";
import { navigate, paths } from "../net/navigate.ts";
import { READ_CACHE_FRESH_MS, clearReadCache } from "../net/readCache.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { cosmeticPacksInfo, ownedCosmeticsOf, tokenBars, walletInfo } from "../shop/fixtures.ts";
import { settle } from "../test/settle.ts";
import { at, meBody, signIn, stubServer, type Reply, type StubServer } from "../test/stubServer.ts";

// The Collection opens on its Cards grid (R3734000), and jsdom has no layout to window it by, so the whole catalog's
// 480 faces would draw at once; what is counted here is requests, not cards.
vi.mock("../collection/collection.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../collection/collection.ts")>();
  return { ...actual, COLLECTED_DEFS: actual.COLLECTED_DEFS.slice(0, 48) };
});

const { App } = await import("../main.tsx");

/** A lazily-imported route chunk plus a stubbed round trip can outrun testing-library's 1 s default. */
const SLOW = { timeout: 5_000 } as const;

const T0 = 1_800_000_000_000;

/** What the stubbed server answers, by the path of each read the two screens make. */
const ANSWERS = {
  "/api/auth/me": meBody(),
  "/api/wallet": walletInfo(),
  "/api/quests": questsHeld(),
  "/api/news": { news: [] },
  "/api/collection": fullCollection(),
  "/api/packs": packsInfo(),
  "/api/cosmetic-packs": cosmeticPacksInfo(),
  "/api/tracks": tracksOf(),
  "/api/cosmetics": ownedCosmeticsOf(),
  "/api/tokens": tokenBars(),
  "/api/catalog": { version: "v1", defs: {} },
} as const;

type Counted = keyof typeof ANSWERS;

const COUNTED = Object.keys(ANSWERS) as Counted[];

function serve(): StubServer {
  return stubServer((call): Reply | undefined => {
    const path = COUNTED.find((counted) => counted === call.path);
    return call.method === "GET" && path !== undefined ? { status: 200, body: ANSWERS[path] } : undefined;
  });
}

/** The page moves on `hopMs` after each screen shows. */
function hop(hopMs: number): void {
  vi.setSystemTime(Date.now() + hopMs);
}

/** Landing → /collection → landing, as a player moves, and the requests each counted read sent. */
async function visit(hopMs: number): Promise<Record<Counted, number>> {
  clearReadCache();
  __resetWalletForTests();
  const server = serve();
  at(paths.landing);
  render(<App />);
  await screen.findByTestId(landingTestid.root, undefined, SLOW);
  await settle({ macrotasks: 5 });
  hop(hopMs);

  act(() => {
    navigate(paths.collection);
  });
  await screen.findByTestId(collectionTestid.screen, undefined, SLOW);
  await settle({ macrotasks: 5 });
  hop(hopMs);

  act(() => {
    navigate(paths.landing);
  });
  await screen.findByTestId(landingTestid.root, undefined, SLOW);
  await settle({ macrotasks: 5 });
  cleanup();

  return Object.fromEntries(COUNTED.map((path) => [path, server.to("GET", path).length])) as Record<Counted, number>;
}

const total = (reads: Record<Counted, number>): number => Object.values(reads).reduce((sum, count) => sum + count, 0);

beforeEach(() => {
  window.localStorage.clear();
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(T0);
  signIn();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("R3206000 moving between screens", () => {
  it("R3206000 landing → /collection → landing sends fewer requests inside the window", async () => {
    // As R2125000 had it: each hop leaves the kept answers older than the window, so each screen reads again.
    const before = await visit(READ_CACHE_FRESH_MS + 1);
    // Inside the window: a second between one screen showing and the next move.
    const after = await visit(1_000);

    expect(total(after)).toBeLessThan(total(before));
    expect(after["/api/auth/me"], "the gate reads the account once, on the first screen").toBe(1);
    expect(after["/api/wallet"], "the pill reads the wallet once").toBe(1);
    expect(before["/api/auth/me"], "past the window each screen's gate reads").toBe(3);
    expect(before["/api/wallet"]).toBeGreaterThan(after["/api/wallet"]);
  });
});
