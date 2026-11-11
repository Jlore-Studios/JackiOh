// R3207003: a screen reads the account once. `/decks` and `/play` are driven through the real route table
// with a stubbed server that answers only `GET /api/auth/me`, and the reads of that path are counted.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DECKBUILDER_RETRY } from "../game/deckbuilder/testids.ts";
import { paths } from "../net/navigate.ts";
import { settle } from "../test/settle.ts";
import { at, meBody, signIn, stubServer, type Reply, type StubServer } from "../test/stubServer.ts";
import { playTestid } from "./play.tsx";

const { App } = await import("../main.tsx");

/** A lazily-imported route chunk plus a stubbed round trip can outrun testing-library's 1 s default. */
const SLOW = { timeout: 5_000 } as const;

const ME_PATH = "/api/auth/me";

/** The account answers; every other read is refused as not found, which these screens show or ignore. */
function serve(): StubServer {
  return stubServer((call): Reply | undefined =>
    call.method === "GET" && call.path === ME_PATH ? { status: 200, body: meBody() } : undefined,
  );
}

beforeEach(() => {
  window.localStorage.clear();
  signIn();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("R3207003 one account read per screen", () => {
  it("R3207003 mounting /play sends one /api/auth/me", async () => {
    const server = serve();
    at(paths.play);
    render(<App />);
    await screen.findByTestId(playTestid.screen, undefined, SLOW);
    await settle({ macrotasks: 5 });

    // The gate's read; the lobby takes its answer instead of reading the account again at mount.
    expect(server.to("GET", ME_PATH)).toHaveLength(1);
  });

  it("R3207003 mounting /decks sends one /api/auth/me", async () => {
    const server = serve();
    at(paths.decks);
    render(<App />);
    // The decks read is refused, so the screen ends on its error with Try again.
    await screen.findByTestId(DECKBUILDER_RETRY, undefined, SLOW);
    await settle({ macrotasks: 5 });

    // R3206000's fresh-answer window already made this one before the screen took the gate's account.
    expect(server.to("GET", ME_PATH)).toHaveLength(1);
  });
});
