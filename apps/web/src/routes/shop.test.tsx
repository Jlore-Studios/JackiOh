// `/shop` (#1577, SPEC §14.1, §14.3): the shop on its own screen renders the same panel the
// account screen shows, reading with the caller's token, and the route table serves it behind
// the gate like `/collection` (the anonymous and pending bounces live in gate.test.tsx beside
// the other routes'): an active account sees the shop, and the landing's Shop door lands on it.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { walletInfo } from "../shop/fixtures.ts";
import { shopTestid } from "../shop/shop.ts";
import { refusal, stubServer, type StubServer } from "../test/stubServer.ts";
import ShopRoute from "./shop.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

/** A stubbed round trip can outrun testing-library's 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const TOKEN = "token-1";

function meBody(status: "pending" | "active"): unknown {
  return {
    profile: { id: "profile-1", status },
    needsInviteCode: status === "pending",
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.test",
  };
}

/** The gate's read plus the shop's two reads, answered from the fixtures. */
function serveShop(status: "active" | "pending" = "active"): StubServer {
  return stubServer((call) => {
    if (call.path === "/api/auth/me") return { status: 200, body: meBody(status) };
    if (call.method === "GET" && call.path === "/api/packs") return { status: 200, body: packsInfo() };
    if (call.method === "GET" && call.path === "/api/wallet") return { status: 200, body: walletInfo() };
    return undefined;
  });
}

function signedIn(token = "e2e-token"): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
}

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

beforeEach(() => {
  window.localStorage.clear();
  resetUsernamePromptForTests();
  at("/");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("#1577 the shop screen", () => {
  it("reads the packs and the wallet with the caller's token", async () => {
    const server = serveShop();
    render(<ShopRoute token={TOKEN} />);

    expect(await screen.findByTestId(shopTestid.panel, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(shopTestid.wallet, undefined, SLOW)).toHaveTextContent("37 Jlore");
    for (const path of ["/api/packs", "/api/wallet"] as const) {
      const calls = server.to("GET", path);
      expect(calls, `one read of ${path}`).toHaveLength(1);
      expect(calls[0]?.headers.authorization).toBe(`Bearer ${TOKEN}`);
    }
  });

  it("a refused read says why on the screen, which keeps its heading", async () => {
    stubServer((call) => {
      if (call.path === "/api/auth/me") return { status: 200, body: meBody("active") };
      if (call.method === "GET" && call.path === "/api/packs") {
        return refusal(503, "internal", "the shop is closed");
      }
      if (call.method === "GET" && call.path === "/api/wallet") return { status: 200, body: walletInfo() };
      return undefined;
    });
    render(<ShopRoute token={TOKEN} />);

    expect(await screen.findByTestId(shopTestid.loadError, undefined, SLOW)).toHaveTextContent(
      "the shop is closed",
    );
    expect(screen.getByRole("heading", { name: "Shop" })).toBeInTheDocument();
  });

  it("names the screen Shop", () => {
    expect(paths.shop).toBe("/shop");
    expect(screenNameFor(paths.shop)).toBe("Shop");
    expect(documentTitleFor(paths.shop)).toMatch(/^Shop · /u);
  });
});

describe("#1577 the /shop route through the route table", () => {
  it("an active account at /shop sees the shop, not the 404 panel", async () => {
    const server = serveShop();
    signedIn();
    at(paths.shop);
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Shop" }, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(shopTestid.wallet, undefined, SLOW)).toHaveTextContent("37 Jlore");
    expect(window.location.pathname).toBe(paths.shop);
    expect(server.to("GET", "/api/packs")).toHaveLength(1);
  });

  it("following the Shop door from the landing ends on /shop", async () => {
    serveShop();
    signedIn();
    at(paths.landing);
    render(<App />);

    fireEvent.click(await screen.findByTestId(landingTestid.shop, undefined, SLOW), { button: 0 });
    await waitFor(() => {
      expect(window.location.pathname).toBe(paths.shop);
    }, SLOW);
    expect(await screen.findByRole("heading", { name: "Shop" }, SLOW)).toBeInTheDocument();
  });
});
