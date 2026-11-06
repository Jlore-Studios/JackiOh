// `/shop` (#1577, #1580, SPEC §14.1, §14.3–§14.6, R1580000): four panels, each reading with the caller's
// token and the others again when one changes the account (#1309); odds and pity are `/purchases`'
// (R1665000). Gated like `/collection` (its bounces are in gate.test.tsx); the account screen links it
// only by the top bar's Jlore pill (R1646000, R1651000). R3211000: a spend's answer folds into the panels
// that show it and the page reads the rest once, when the opening closes (the last describe).

import { CATALOG } from "@jackioh/cards";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
import { CardFace, faceModel } from "../cards/index.ts";
import { resetCosmeticsForTests } from "../cards/cosmetics.ts";
import {
  ViewerAccountCosmetics,
  ViewerFullArt,
  readOwnedFullArt,
  resetOwnedFullArtForTests,
} from "../cards/ownedFullArt.ts";
import AccountCosmeticTiles from "../collection/AccountCosmeticTiles.tsx";
import { collectionTestid } from "../collection/collection.ts";
import { jlorePillTestid } from "../economy/JlorePill.tsx";
import type { PacksResponse } from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { openedPack, packsInfo } from "../packs/fixtures.ts";
import { packTestid } from "../packs/packs.ts";
import {
  collectionOwningAllBut,
  cosmeticPacksInfo,
  openedCosmeticPack,
  ownedCosmeticsOf,
  tokenBars,
  walletInfo,
} from "../shop/fixtures.ts";
import { shopTestid } from "../shop/shop.ts";
import { at, meBody, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { expectOnePageTitle } from "../test/pageTitle.ts";
import { accountTestid } from "./account.tsx";
import { purchasesTestid } from "./purchases.tsx";
import ShopRoute, { shopRouteTestid } from "./shop.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

/** A stubbed round trip can outrun testing-library's 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const TOKEN = "token-1";

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
      return { status: 200, body: collectionOwningAllBut("core-002", "core-003") };
    case "/api/cosmetics":
      return { status: 200, body: ownedCosmeticsOf() };
    case "/api/cosmetic-packs":
      return { status: 200, body: cosmeticPacksInfo() };
    default:
      return undefined;
  }
}

/** The gate's read, the account's profile and the Shop's six reads, from the fixtures. */
function serveShop(status: "active" | "pending" = "active"): StubServer {
  return stubServer((call) => {
    if (call.path === "/api/auth/me") {
      return { status: 200, body: meBody({ profile: { id: "profile-1", status }, needsInviteCode: status === "pending" }) };
    }
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

beforeEach(() => {
  window.localStorage.clear();
  resetUsernamePromptForTests();
  resetOwnedFullArtForTests();
  resetCosmeticsForTests();
  at("/");
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
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
    // R3206000: both panels ask at once, and one read answers both.
    expect(server.to("GET", "/api/packs"), "the shop's price and the opening's packs share one read").toHaveLength(1);
  });

  it("a refused read says why on the screen, which keeps its heading", async () => {
    stubServer((call) => {
      if (call.path === "/api/auth/me") return { status: 200, body: meBody() };
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

  it("R3223000 the screen has one visible h1, the PageTitle", async () => {
    serveShop();
    render(<ShopRoute token={TOKEN} />);

    expect(await screen.findByTestId(shopTestid.wallet, undefined, SLOW)).toHaveTextContent("37 Jlore");
    expectOnePageTitle("Shop");
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
    // R3206000: the two panels' first reads were one, and the buy's answer folded into the kept packs,
    // which the opening's read then found fresh: the screen shows what the server holds with one read.
    expect(server.to("GET", "/api/packs")).toHaveLength(1);

    expect(await screen.findByTestId(shopTestid.cardBar, undefined, SLOW)).toHaveTextContent("149 / 150");
    fireEvent.click(await screen.findByTestId(shopTestid.open, undefined, SLOW));
    await waitFor(() => {
      expect(screen.getByTestId(shopTestid.cardBar), "the picker reads the bars the opening filled").toHaveTextContent(
        "3 / 150",
      );
    }, SLOW);
    expect(screen.getByTestId(shopTestid.wallet)).toHaveTextContent("0 Jlore");
  });

  it("an opening on the Shop has the next screen read what the account owns again", async () => {
    stubServer((call) =>
      call.method === "POST" && call.path === "/api/cosmetic-packs/open"
        ? { status: 200, body: openedCosmeticPack() }
        : shopRead(call),
    );
    const fetchOwned = vi.fn(() => Promise.resolve(ownedCosmeticsOf()));
    await readOwnedFullArt(TOKEN, fetchOwned);
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned, "the session's answer is kept").toHaveBeenCalledTimes(1);

    render(<ShopRoute token={TOKEN} />);
    fireEvent.click(await screen.findByTestId(shopTestid.open, undefined, SLOW));
    await screen.findByTestId(shopTestid.reveal, undefined, SLOW);

    // The opening may have granted a per-card cosmetic: the Collection's strip and the board read again.
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned).toHaveBeenCalledTimes(2);
  });

  it("R1955000 an opening has the open screen read what the account owns again, so what its reveal equips is worn at once", async () => {
    const before = ownedCosmeticsOf({ account: ["table-chessboard"] });
    let owned = before;
    stubServer((call) => {
      if (call.method === "POST" && call.path === "/api/cosmetic-packs/open") {
        owned = {
          cards: [...before.cards, { cardId: "core-001", cosmeticId: "border-stone" }],
          account: [...before.account, "table-marble"],
        };
        const answer = openedCosmeticPack();
        return {
          status: 200,
          body: {
            ...answer,
            slots: [
              {
                rarity: "Common",
                grant: { kind: "cosmetic", cosmeticId: "border-stone", cosmeticRarity: "Common", cardId: "core-001", cardGranted: false },
              },
              {
                rarity: "Epic",
                grant: { kind: "cosmetic", cosmeticId: "table-marble", cosmeticRarity: "Epic", cardId: null, cardGranted: false },
              },
            ],
          },
        };
      }
      if (call.method === "GET" && call.path === "/api/cosmetics") return { status: 200, body: owned };
      return shopRead(call);
    });
    render(
      <ViewerFullArt token={TOKEN} gated>
        <ViewerAccountCosmetics token={TOKEN} gated>
          <div data-testid="viewer-card">
            <CardFace face={faceModel({ defId: "core-001", def: CATALOG["core-001"], radiant: false })} worn />
          </div>
          <AccountCosmeticTiles />
          <ShopRoute token={TOKEN} />
        </ViewerAccountCosmetics>
      </ViewerFullArt>,
    );
    await waitFor(() => {
      expect(screen.getByTestId(collectionTestid.tile("table-chessboard"))).toHaveAttribute("data-owned", "true");
    }, SLOW);
    expect(screen.getByTestId(collectionTestid.tile("table-marble"))).toHaveAttribute("data-owned", "false");

    fireEvent.click(await screen.findByTestId(shopTestid.open, undefined, SLOW));
    await screen.findByTestId(shopTestid.reveal, undefined, SLOW);
    fireEvent.click(screen.getByTestId(shopTestid.equip(0)));
    fireEvent.click(screen.getByTestId(shopTestid.equip(1)));

    // Equip only wrote the picks; the re-read of the ledger is what lets the card and the side wear them.
    await waitFor(() => {
      expect(within(screen.getByTestId("viewer-card")).getByText(CATALOG["core-001"]!.name).closest(".cf")).toHaveAttribute(
        "data-frame",
        "border-stone",
      );
      expect(screen.getByTestId(collectionTestid.tile("table-marble"))).toHaveAttribute("data-equipped", "true");
    }, SLOW);
  });

  it("R1665000 renders no pity counters and no odds table, and links /purchases", async () => {
    serveShop();
    render(<ShopRoute token={TOKEN} />);

    // Every panel has read, so anything drawn only on an answer would be here.
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
    signIn();
    at(paths.shop);
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Shop" }, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(shopTestid.wallet, undefined, SLOW)).toHaveTextContent("37 Jlore");
    expect(window.location.pathname).toBe(paths.shop);
    // R3206000: the shop's panel and the opening's share the one read.
    expect(server.to("GET", "/api/packs")).toHaveLength(1);
  });

  it("R1310000 an active account the server does not list as dev opens its card packs on the Shop", async () => {
    const server = serveShop();
    signIn();
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
      if (call.path === "/api/auth/me") return { status: 200, body: meBody() };
      if (call.method === "GET" && call.path === "/api/packs") {
        return { status: 200, body: { ...info, odds: { ...info.odds, globalCosmetic } } };
      }
      return shopRead(call);
    });
    signIn();
    at(paths.shop);
    render(<App />);

    fireEvent.click(await screen.findByTestId(shopRouteTestid.purchases, undefined, SLOW), { button: 0 });
    expect(await screen.findByTestId(purchasesTestid.pity, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe(paths.purchases);
  });

  it("following the Shop door from the landing ends on /shop", async () => {
    serveShop();
    signIn();
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
    signIn();
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
    signIn();
    at(paths.account);
    render(<App />);

    const pill = await screen.findByTestId(jlorePillTestid, undefined, SLOW);
    const account = screen.getByTestId(accountTestid.screen);
    expect(Array.from(account.querySelectorAll(`a[href="${paths.shop}"]`))).toEqual([pill]);
    expect(pill.closest("nav")).not.toBeNull();
    expect(pill).toHaveAccessibleName(`${String(walletInfo().jlore)} Jlore`);
  });
});

describe("R3211000 the Shop folds each spend's answer and reads once, when the opening closes", () => {
  /** What an opening answers for the bars: the card bar paid a token and the cosmetic bar moved on. */
  const OPENING_BARS = {
    card: { before: 149, after: 41, size: 150, pays: 1 },
    cosmetic: { before: 30, after: 77, size: 150, pays: 0 },
    cosmeticPaid: [],
    slotPoints: [0, 0, 0, 0, 0],
  } as const;

  /**
   * A server that keeps the account: `held` Core packs, a wallet and the tokens, each moving with the spend
   * that changes it, so what a read answers is what the spends left. `GET`s after the Shop is up are
   * `reads()`, the `POST`s `spends()`.
   */
  function serveSpends(start: number): { server: StubServer; reads: () => string[]; spends: () => string[]; mark: () => void } {
    let held = start;
    let jlore = packsInfo().price + 50;
    let bars = tokenBars();
    const server = stubServer((call) => {
      if (call.method === "GET") {
        switch (call.path) {
          case "/api/packs":
            return { status: 200, body: { ...packsInfo(), unopened: held === 0 ? [] : [{ pack: "Core", count: held }] } };
          case "/api/wallet":
            return { status: 200, body: { jlore } };
          case "/api/tokens":
            return { status: 200, body: bars };
          default:
            return shopRead(call);
        }
      }
      if (call.path === "/api/packs/buy") {
        held += 1;
        jlore = 12;
        return { status: 200, body: { pack: "Core", jlore, unopened: held } };
      }
      if (call.path === "/api/packs/open") return { status: 200, body: open() };
      // R3212000: each pack as `/api/packs/open` would open it, in one answer.
      if (call.path === "/api/packs/open-all") {
        const { count } = call.body as { count: number };
        return { status: 200, body: { openings: Array.from({ length: count }, open) } };
      }
      return undefined;
    });
    function open() {
      held -= 1;
      bars = {
        ...bars,
        cardBar: { points: OPENING_BARS.card.after, size: OPENING_BARS.card.size },
        cosmeticBar: { points: OPENING_BARS.cosmetic.after, size: OPENING_BARS.cosmetic.size },
        tokens: { ...bars.tokens, card: bars.tokens.card + 1 },
      };
      return { ...openedPack(), unopened: held, bars: OPENING_BARS };
    }
    let from = 0;
    return {
      server,
      mark: () => {
        from = server.calls.length;
      },
      reads: () => server.calls.slice(from).filter((call) => call.method === "GET").map((call) => call.path),
      spends: () => server.calls.slice(from).filter((call) => call.method === "POST").map((call) => call.path),
    };
  }

  /** The Shop with every panel read, so what is counted from here is what the spends cost. */
  async function shopUp(): Promise<void> {
    await screen.findByTestId(shopTestid.wallet, undefined, SLOW);
    await screen.findByTestId(shopTestid.unopened, undefined, SLOW);
    await screen.findByTestId(shopTestid.cardBar, undefined, SLOW);
    await waitFor(() => {
      expect(screen.queryByTestId(packTestid.held) ?? screen.queryByTestId(packTestid.none)).not.toBeNull();
    }, SLOW);
  }

  const SETTLE_READS = ["/api/cosmetic-packs", "/api/tokens"];

  it("R3211000 one buy then one open sends the two spends and at most two reads, after the scene closes", async () => {
    const account = serveSpends(0);
    render(<ShopRoute token={TOKEN} />);
    await shopUp();
    account.mark();

    fireEvent.click(screen.getByTestId(shopTestid.buy("Core")));
    await waitFor(() => {
      expect(screen.getByTestId(shopTestid.wallet)).toHaveTextContent("12 Jlore");
    }, SLOW);
    expect(screen.getByTestId(shopTestid.held("Core"))).toHaveTextContent("Core pack: 1 unopened");
    await waitFor(() => {
      expect(screen.getByTestId(packTestid.heldRow("Core"))).toHaveTextContent("Core pack: 1 unopened");
    }, SLOW);
    expect(account.reads(), "a buy's answer is folded, nothing is read").toEqual([]);

    fireEvent.click(screen.getByTestId(packTestid.open("Core")));
    fireEvent.click(screen.getByTestId(packTestid.skipAll));
    await screen.findByTestId(packTestid.outcome(0), undefined, SLOW);
    expect(account.reads(), "nothing is read while the scene is open").toEqual([]);

    fireEvent.click(screen.getByTestId(packTestid.close));
    await waitFor(() => {
      expect([...account.reads()].sort()).toEqual(SETTLE_READS);
    }, SLOW);
    // The Shop is settled: the reads are as many as they were, and the tokens read brought the held counts.
    await waitFor(() => {
      expect(screen.getByTestId(shopTestid.heldCard)).toHaveTextContent("Card tokens held: 2");
    }, SLOW);
    expect([...account.reads()].sort()).toEqual(SETTLE_READS);
    expect(account.spends()).toEqual(["/api/packs/buy", "/api/packs/open"]);
  });

  it("R3211000 R3212000 Open all of ten sends one spend, no read while the scene is open, and at most two once it closes", async () => {
    const account = serveSpends(10);
    const fetchOwned = vi.fn(() => Promise.resolve(ownedCosmeticsOf()));
    render(<ShopRoute token={TOKEN} />);
    await shopUp();
    account.mark();

    fireEvent.click(screen.getByTestId(packTestid.openAll("Core")));
    for (let pack = 1; pack <= 10; pack += 1) {
      fireEvent.click(await screen.findByTestId(packTestid.skipAll, undefined, SLOW));
      await screen.findByTestId(packTestid.outcome(0), undefined, SLOW);
      if (pack < 10) {
        fireEvent.click(screen.getByTestId(packTestid.next));
        await waitFor(() => {
          expect(screen.queryByTestId(packTestid.outcome(0))).toBeNull();
        }, SLOW);
      }
    }
    expect(account.spends()).toEqual(["/api/packs/open-all"]);
    expect(account.reads(), "no read while the scene is open").toEqual([]);
    await waitFor(() => {
      expect(screen.getByTestId(packTestid.none), "the opening folded its count to none").toBeInTheDocument();
      expect(screen.getByTestId(shopTestid.held("Core"))).toHaveTextContent("Core pack: 0 unopened");
      expect(screen.getByTestId(shopTestid.cardBar)).toHaveTextContent("41 / 150");
    }, SLOW);

    // The ledger of owned cosmetics is read again once for the scene, not once for each opening.
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned, "no opening forgot it yet").toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId(packTestid.close));
    await waitFor(() => {
      expect([...account.reads()].sort()).toEqual(SETTLE_READS);
    }, SLOW);
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned, "the close forgot it once").toHaveBeenCalledTimes(2);
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned).toHaveBeenCalledTimes(2);
    expect(account.spends()).toHaveLength(1);
  });

  it("R3211000 the panels show the numbers of the answers", async () => {
    // Every read answers the account as it was at the start: only an answer can have moved a panel.
    const info = { ...packsInfo(), unopened: [{ pack: "Core", count: 2 }] };
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/packs") return { status: 200, body: info };
      // Rich enough for a Buy to be pressed.
      if (call.method === "GET" && call.path === "/api/wallet") return { status: 200, body: { jlore: info.price + 50 } };
      if (call.method === "POST" && call.path === "/api/packs/buy") {
        return { status: 200, body: { pack: "Core", jlore: 12, unopened: 3 } };
      }
      if (call.method === "POST" && call.path === "/api/packs/open") {
        return { status: 200, body: { ...openedPack(), unopened: 2, bars: OPENING_BARS } };
      }
      return shopRead(call);
    });
    render(<ShopRoute token={TOKEN} />);
    await shopUp();
    const readsBefore = server.calls.filter((call) => call.method === "GET").length;

    fireEvent.click(screen.getByTestId(shopTestid.buy("Core")));
    await waitFor(() => {
      expect(screen.getByTestId(shopTestid.wallet)).toHaveTextContent("12 Jlore");
    }, SLOW);
    expect(screen.getByTestId(shopTestid.held("Core"))).toHaveTextContent("Core pack: 3 unopened");
    await waitFor(() => {
      expect(screen.getByTestId(packTestid.heldRow("Core"))).toHaveTextContent("Core pack: 3 unopened");
    }, SLOW);

    fireEvent.click(screen.getByTestId(packTestid.open("Core")));
    fireEvent.click(screen.getByTestId(packTestid.skipAll));
    await screen.findByTestId(packTestid.outcome(0), undefined, SLOW);
    // The open's answer holds 2 unopened, and the bars it settled: the shelf, the opening and the picker take them.
    await waitFor(() => {
      expect(screen.getByTestId(shopTestid.held("Core"))).toHaveTextContent("Core pack: 2 unopened");
      expect(screen.getByTestId(packTestid.heldRow("Core"))).toHaveTextContent("Core pack: 2 unopened");
      expect(screen.getByTestId(shopTestid.cardBar)).toHaveTextContent("41 / 150");
      expect(screen.getByTestId(shopTestid.cosmeticBar)).toHaveTextContent("77 / 150");
    }, SLOW);
    expect(screen.getByTestId(shopTestid.wallet), "an opening spends no Jlore").toHaveTextContent("12 Jlore");
    expect(server.calls.filter((call) => call.method === "GET")).toHaveLength(readsBefore);
  });

  it("R3211000 a cosmetic pack's opening folds its bars and reads the tokens once, and neither the packs nor the wallet", async () => {
    let bars = tokenBars();
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/tokens") return { status: 200, body: bars };
      if (call.method === "POST" && call.path === "/api/cosmetic-packs/open") {
        bars = { ...bars, cardBar: { points: 7, size: 12 }, cosmeticBar: { points: 9, size: 30 } };
        return { status: 200, body: { ...openedCosmeticPack(), shardBefore: 5, shardBar: { points: 6, size: 20 } } };
      }
      return shopRead(call);
    });
    render(<ShopRoute token={TOKEN} />);
    await shopUp();
    const mark = server.calls.length;

    fireEvent.click(await screen.findByTestId(shopTestid.open, undefined, SLOW));
    await screen.findByTestId(shopTestid.reveal, undefined, SLOW);
    await waitFor(() => {
      expect(screen.getByTestId(shopTestid.cardBar)).toHaveTextContent("7 / 12");
    }, SLOW);
    expect(screen.getByTestId(shopTestid.unopened)).toHaveTextContent("Cosmetic pack: 1 unopened");
    await act(async () => {
      await new Promise((resolve) => {
        setTimeout(resolve, 0);
      });
    });
    const reads = server.calls.slice(mark).filter((call) => call.method === "GET");
    expect(reads.map((call) => call.path), "the tokens' held counts, once").toEqual(["/api/tokens"]);
    expect(server.calls.slice(mark).filter((call) => call.method === "POST").map((call) => call.path)).toEqual([
      "/api/cosmetic-packs/open",
    ]);
  });

  it("R3211000 leaving the Shop with an opening unsettled has the owned cosmetics read again", async () => {
    const account = serveSpends(1);
    const fetchOwned = vi.fn(() => Promise.resolve(ownedCosmeticsOf()));
    const shop = render(<ShopRoute token={TOKEN} />);
    await shopUp();
    await readOwnedFullArt(TOKEN, fetchOwned);

    fireEvent.click(screen.getByTestId(packTestid.open("Core")));
    fireEvent.click(screen.getByTestId(packTestid.skipAll));
    await screen.findByTestId(packTestid.outcome(0), undefined, SLOW);
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned, "the scene is open: nothing forgot the ledger").toHaveBeenCalledTimes(1);

    shop.unmount();
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned).toHaveBeenCalledTimes(2);
    expect(account.spends()).toEqual(["/api/packs/open"]);
  });

  it("R3211000 leaving the Shop after the scene closed forgets the owned cosmetics no second time", async () => {
    serveSpends(1);
    const fetchOwned = vi.fn(() => Promise.resolve(ownedCosmeticsOf()));
    const shop = render(<ShopRoute token={TOKEN} />);
    await shopUp();

    fireEvent.click(screen.getByTestId(packTestid.open("Core")));
    fireEvent.click(screen.getByTestId(packTestid.skipAll));
    await screen.findByTestId(packTestid.outcome(0), undefined, SLOW);
    fireEvent.click(screen.getByTestId(packTestid.close));
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned).toHaveBeenCalledTimes(1);

    shop.unmount();
    await readOwnedFullArt(TOKEN, fetchOwned);
    expect(fetchOwned, "the close had settled: leaving has nothing more to forget").toHaveBeenCalledTimes(1);
  });
});
