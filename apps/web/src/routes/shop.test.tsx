// `/shop` (#1577, #1580, SPEC §14.1, §14.3–§14.6, R1580000): the Shop screen holds the Jlore shop, the
// card packs with their opening, the cosmetic packs and the token picker, each reading with the
// caller's token and the others again when one changes the account (#1309, moved from the account
// screen). Their odds and the pity are `/purchases`', which it links (R1665000). The route table serves
// it behind the gate like `/collection` (the anonymous and pending bounces live in gate.test.tsx beside
// the other routes'): an active account sees the shop, the landing's Shop door lands on it, and the
// account screen links it no more but by the top bar's Jlore pill (R1646000, R1651000).

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
import { jlorePillTestid } from "../economy/JlorePill.tsx";
import type { PacksResponse } from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { packTestid } from "../packs/packs.ts";
import {
  collectionOf,
  cosmeticPacksInfo,
  openedCosmeticPack,
  ownedCosmeticsOf,
  tokenBars,
  walletInfo,
} from "../shop/fixtures.ts";
import { shopTestid } from "../shop/shop.ts";
import { refusal, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { accountTestid } from "./account.tsx";
import { purchasesTestid } from "./purchases.tsx";
import ShopRoute, { shopRouteTestid } from "./shop.tsx";

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

/** The six reads the Shop screen's four panels make, answered from the fixtures. */
function shopRead(call: Call): Reply | undefined {
  if (call.method !== "GET") return undefined;
  switch (call.path) {
    case "/api/packs":
      return { status: 200, body: packsInfo() };
    case "/api/wallet":
      return { status: 200, body: walletInfo() };
    case "/api/tokens":
      return { status: 200, body: tokenBars() };
    case "/api/collection":
      return { status: 200, body: collectionOf() };
    case "/api/cosmetics":
      return { status: 200, body: ownedCosmeticsOf() };
    case "/api/cosmetic-packs":
      return { status: 200, body: cosmeticPacksInfo() };
    default:
      return undefined;
  }
}

/** The gate's read, the account screen's profile and the Shop's six reads, answered from the fixtures. */
function serveShop(status: "active" | "pending" = "active"): StubServer {
  return stubServer((call) => {
    if (call.path === "/api/auth/me") return { status: 200, body: meBody(status) };
    if (call.method === "GET" && call.path === "/api/profile") {
      return {
        status: 200,
        body: {
          id: "profile-1",
          email: "player@example.test",
          status: "active",
          record: { wins: 0, losses: 0, draws: 0 },
          winRate: null,
        },
      };
    }
    return shopRead(call);
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
  it("R1580000 every panel reads with the caller's token", async () => {
    const server = serveShop();
    render(<ShopRoute token={TOKEN} />);

    expect(await screen.findByTestId(shopTestid.wallet, undefined, SLOW)).toHaveTextContent("37 Jlore");
    expect(await screen.findByTestId(packTestid.open("Core"), undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(shopTestid.cardBar, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(shopTestid.packsPanel, undefined, SLOW)).toBeInTheDocument();
    for (const path of [
      "/api/packs",
      "/api/wallet",
      "/api/tokens",
      "/api/collection",
      "/api/cosmetics",
      "/api/cosmetic-packs",
    ] as const) {
      const calls = server.to("GET", path);
      expect(calls.length, `${path} is read`).toBeGreaterThan(0);
      for (const call of calls) expect(call.headers.authorization, path).toBe(`Bearer ${TOKEN}`);
    }
    expect(server.to("GET", "/api/packs"), "the shop's price and the opening's packs, once each").toHaveLength(2);
  });

  it("a refused read says why on the screen, which keeps its heading", async () => {
    stubServer((call) => {
      if (call.path === "/api/auth/me") return { status: 200, body: meBody("active") };
      if (call.method === "GET" && call.path === "/api/packs") {
        return refusal(503, "internal", "the shop is closed");
      }
      return shopRead(call);
    });
    render(<ShopRoute token={TOKEN} />);

    expect(await screen.findByTestId(shopTestid.loadError, undefined, SLOW)).toHaveTextContent(
      "the shop is closed",
    );
    expect(await screen.findByTestId(packTestid.loadError, undefined, SLOW)).toHaveTextContent(
      "the shop is closed",
    );
    expect(screen.getByRole("heading", { name: "Shop" })).toBeInTheDocument();
  });

  it("R1580000 buying and opening are on the Shop, and the opening plays over the screen", async () => {
    serveShop();
    render(<ShopRoute token={TOKEN} />);

    const body = await screen.findByTestId(shopTestid.body, undefined, SLOW);
    for (const testid of [shopTestid.panel, packTestid.panel, shopTestid.packsPanel, shopTestid.tokenPanel]) {
      expect(within(body).getByTestId(testid), testid).toBeInTheDocument();
    }
    expect(await screen.findByTestId(shopTestid.buy("Core"), undefined, SLOW)).toBeInTheDocument();

    fireEvent.click(await screen.findByTestId(packTestid.open("Core"), undefined, SLOW));
    const scene = await screen.findByTestId(packTestid.scene, undefined, SLOW);
    expect(scene.parentElement, "the scene is drawn over the page, not inside the body").toBe(document.body);
    expect(body).not.toContainElement(scene);
  });

  it("#1309 a change in one panel reads the others again: a bought pack opens at once, an opening moves the bars", async () => {
    // The server's side: what each read answers moves with what was bought or opened.
    let held: PacksResponse["unopened"] = [];
    let jlore = packsInfo().price;
    let bars = tokenBars();
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/packs") {
        return { status: 200, body: { ...packsInfo(), unopened: held } };
      }
      if (call.method === "GET" && call.path === "/api/wallet") return { status: 200, body: { jlore } };
      if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: bars };
      if (call.method === "POST" && call.path === "/api/packs/buy") {
        const { pack } = call.body as { pack: string };
        held = [{ pack, count: 1 }];
        jlore = 0;
        return { status: 200, body: { pack, jlore, unopened: 1 } };
      }
      if (call.method === "POST" && call.path === "/api/cosmetic-packs/open") {
        bars = { ...bars, cardBar: { ...bars.cardBar, points: 3 } };
        return { status: 200, body: openedCosmeticPack() };
      }
      return shopRead(call);
    });
    render(<ShopRoute token={TOKEN} />);
    expect(await screen.findByTestId(packTestid.none, undefined, SLOW)).toHaveTextContent("No unopened packs.");
    await screen.findByTestId(shopTestid.wallet, undefined, SLOW);

    fireEvent.click(await screen.findByTestId(shopTestid.buy("Core"), undefined, SLOW));
    expect(
      await screen.findByTestId(packTestid.open("Core"), undefined, SLOW),
      "the opening holds the bought pack",
    ).toBeInTheDocument();
    expect(screen.queryByTestId(packTestid.none)).toBeNull();
    expect(server.to("GET", "/api/packs")).toHaveLength(3);

    expect(await screen.findByTestId(shopTestid.cardBar, undefined, SLOW)).toHaveTextContent("149 / 150");
    fireEvent.click(await screen.findByTestId(shopTestid.open, undefined, SLOW));
    await waitFor(() => {
      expect(screen.getByTestId(shopTestid.cardBar), "the picker reads the bars the opening filled").toHaveTextContent(
        "3 / 150",
      );
    }, SLOW);
    expect(screen.getByTestId(shopTestid.wallet)).toHaveTextContent("0 Jlore");
  });

  it("R1665000 renders no pity counters and no odds table, and links /purchases", async () => {
    serveShop();
    render(<ShopRoute token={TOKEN} />);

    // Wait for every panel to read, so an element a panel draws only once it has its answer would be here.
    await screen.findByTestId(packTestid.open("Core"), undefined, SLOW);
    await screen.findByTestId(shopTestid.unopened, undefined, SLOW);
    await screen.findByTestId(shopTestid.cardBar, undefined, SLOW);
    await screen.findByTestId(shopTestid.wallet, undefined, SLOW);

    const link = screen.getByTestId(shopRouteTestid.purchases);
    expect(link).toHaveAttribute("href", paths.purchases);
    expect(link).toHaveTextContent("Odds & pity");
    expect(screen.getByTestId(shopTestid.body), "the link is at the foot, outside the scrolling body").not.toContainElement(link);

    for (const key of ["epic", "legendary", "mythic"]) {
      expect(screen.queryByTestId(packTestid.pity(key)), `pity ${key}`).toBeNull();
    }
    for (const testid of [
      packTestid.pityPanel,
      packTestid.pityLine,
      packTestid.odds,
      packTestid.oddsRow("common"),
      packTestid.duplicate,
      shopTestid.odds,
      shopTestid.flooredOdds,
    ]) {
      expect(screen.queryByTestId(testid), testid).toBeNull();
    }
    expect(screen.queryByRole("table")).toBeNull();
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
    expect(server.to("GET", "/api/packs")).toHaveLength(2);
  });

  it("R1310000 an active account the server does not list as dev opens its card packs on the Shop", async () => {
    const server = serveShop();
    signedIn();
    at(paths.shop);
    render(<App />);

    expect(await screen.findByTestId(packTestid.open("Core"), undefined, SLOW)).toBeInTheDocument();
    for (const call of server.to("GET", "/api/packs")) {
      expect(call.headers.authorization).toBe("Bearer e2e-token");
    }
  });

  it("R1665000 the Odds & pity link at the foot of /shop opens /purchases", async () => {
    // `packsInfo` carries no answer for the global roll (R1661001), which the Purchases page shows.
    const info = packsInfo();
    const globalCosmetic = { chance: 0.1, tiers: { common: 0, rare: 0, epic: 0.9, legendary: 0.08, mythic: 0.02 } };
    stubServer((call) => {
      if (call.path === "/api/auth/me") return { status: 200, body: meBody("active") };
      if (call.method === "GET" && call.path === "/api/packs") {
        return { status: 200, body: { ...info, odds: { ...info.odds, globalCosmetic } } };
      }
      return shopRead(call);
    });
    signedIn();
    at(paths.shop);
    render(<App />);

    fireEvent.click(await screen.findByTestId(shopRouteTestid.purchases, undefined, SLOW), { button: 0 });
    expect(await screen.findByTestId(purchasesTestid.pityLine, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe(paths.purchases);
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

describe("R1646000 the account screen links no Shop", () => {
  it("R1646000 the account screen, through the route table, offers no link to /shop", async () => {
    serveShop();
    signedIn();
    at(paths.account);
    render(<App />);

    const record = await screen.findByTestId(accountTestid.record, undefined, SLOW);
    const account = screen.getByTestId(accountTestid.screen);
    expect(account).toContainElement(record);
    // R1651000: the top bar's Jlore pill is every signed-in screen's; the screen's own links are these.
    const own = Array.from(account.querySelectorAll(`a[href="${paths.shop}"]`)).filter((link) => link.closest("nav") === null);
    expect(own).toEqual([]);
    expect(within(account).queryByRole("link", { name: "Shop" })).toBeNull();
    expect(window.location.pathname).toBe(paths.account);
  });

  it("R1651000 the account screen's one way to /shop is the Jlore pill in its top bar", async () => {
    serveShop();
    signedIn();
    at(paths.account);
    render(<App />);

    const pill = await screen.findByTestId(jlorePillTestid, undefined, SLOW);
    const account = screen.getByTestId(accountTestid.screen);
    expect(Array.from(account.querySelectorAll(`a[href="${paths.shop}"]`))).toEqual([pill]);
    expect(pill.closest("nav")).not.toBeNull();
    expect(pill).toHaveAccessibleName(`${String(walletInfo().jlore)} Jlore`);
  });
});
