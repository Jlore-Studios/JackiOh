// `/purchases` (#1665, part of #1651, SPEC §14.4, R1665000) and the footer's link to it; a failed read
// offers Try again (#1774). R3015003 (#3015): both packs' odds say how often a pull grants a level past a variant's next
// one (`levelSkip`). Every figure below is the fixture's, none of them the shipped tables'; the floors and the
// cosmetic pack's slot count are the generated config's, as the page reads them.

import { COSMETIC_PACK_FLOOR_RARITY, COSMETIC_PACK_SLOTS, PACK_FLOOR_RARITY } from "@jackioh/server-config";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PacksResponse } from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { cosmeticPacksInfo } from "../shop/fixtures.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { at, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import { expectOnePageTitle } from "../test/pageTitle.ts";
import PurchasesRoute, { purchasesTestid } from "./purchases.tsx";
import { SiteFooter, siteFooterTestid } from "./SiteFooter.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

/** A stubbed round trip, or a lazily-imported chunk, can outrun testing-library's 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const TOKEN = "e2e-token";

/**
 * `GET /api/packs` with the answer a duplicate's global roll adds (R1661001), the numbers the fixture's own,
 * and a price and a slot count the summary line must read from it, not from a constant.
 */
function packsWithGlobalRoll(): PacksResponse {
  const info = packsInfo();
  return {
    ...info,
    price: 120,
    slots: 6,
    odds: {
      ...info.odds,
      globalCosmetic: {
        chance: 0.125,
        tiers: { common: 0, rare: 0, epic: 0.93, legendary: 0.06, mythic: 0.01 },
      },
    },
  };
}

function answers(call: Call): Reply | undefined {
  if (call.method !== "GET") return undefined;
  if (call.path === "/api/packs") return { status: 200, body: packsWithGlobalRoll() };
  if (call.path === "/api/cosmetic-packs") return { status: 200, body: cosmeticPacksInfo() };
  return undefined;
}

function servePurchases(): StubServer {
  return stubServer(answers);
}

/** What the page's two odds reads cost: no request at all. */
function oddsReads(server: StubServer): Call[] {
  return [...server.to("GET", "/api/packs"), ...server.to("GET", "/api/cosmetic-packs")];
}

beforeEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  resetUsernamePromptForTests();
  at("/");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
  window.sessionStorage.clear();
});

describe("R1665000 /purchases signed in", () => {
  it("R1665000 R3015003 shows the server's odds in tables, the duplicates' table, the chance of a level past the next and the pity table", async () => {
    const server = servePurchases();
    signIn(TOKEN);
    render(<PurchasesRoute />);

    const odds = await screen.findByTestId(purchasesTestid.odds, undefined, SLOW);
    const row = (table: HTMLElement, testid: (key: string) => string, key: string): HTMLElement =>
      within(table).getByTestId(testid(key));
    // The summary line's price and slot count are the server's, its floor the generated config's.
    expect(screen.getByTestId(purchasesTestid.cardPacksSummary)).toHaveTextContent(
      `120 Jlore · 6 cards · one card is always ${PACK_FLOOR_RARITY} or better`,
    );
    // A slot's odds and the guaranteed slot's, the fallback (Common) a dash when it is 0.
    expect(within(odds).getAllByRole("columnheader").map((head) => head.textContent)).toEqual([
      "Rarity",
      "Each card",
      "Guaranteed card",
    ]);
    expect(row(odds, purchasesTestid.oddsRow, "common")).toHaveTextContent("Common50%—");
    expect(row(odds, purchasesTestid.oddsRow, "rare")).toHaveTextContent("Rare32%82%");
    expect(row(odds, purchasesTestid.oddsRow, "epic")).toHaveTextContent("Epic12.3%12.3%");
    expect(row(odds, purchasesTestid.oddsRow, "legendary")).toHaveTextContent("Legendary4.5%4.5%");
    expect(row(odds, purchasesTestid.oddsRow, "mythic")).toHaveTextContent("Mythic1.2%1.2%");

    // What a duplicate becomes: its cosmetic beside the global roll's, the chance in the column's head.
    expect(screen.getByRole("heading", { name: "Duplicates" })).toBeInTheDocument();
    expect(screen.getByText("A card you already own becomes a cosmetic.")).toBeInTheDocument();
    const duplicate = screen.getByTestId(purchasesTestid.duplicate);
    expect(within(duplicate).getAllByRole("columnheader").map((head) => head.textContent)).toEqual([
      "Rarity",
      "Cosmetic",
      "Account cosmetic (12.5% of duplicates)",
    ]);
    expect(row(duplicate, purchasesTestid.duplicateRow, "common")).toHaveTextContent("Common50%—");
    expect(row(duplicate, purchasesTestid.duplicateRow, "rare")).toHaveTextContent("Rare30%—");
    expect(row(duplicate, purchasesTestid.duplicateRow, "epic")).toHaveTextContent("Epic15%93%");
    expect(row(duplicate, purchasesTestid.duplicateRow, "legendary")).toHaveTextContent("Legendary4%6%");
    expect(row(duplicate, purchasesTestid.duplicateRow, "mythic")).toHaveTextContent("Mythic1%1%");
    expect(screen.getByTestId(purchasesTestid.duplicateNote)).toHaveTextContent(
      "If you already own every cosmetic of that rarity, the duplicate fills both token bars instead.",
    );
    // R3015003: the server's chance that a duplicate's cosmetic grants a level past the next, and every level below it.
    expect(screen.getByTestId(purchasesTestid.levelSkip)).toHaveTextContent(
      "A variant drops its next level. 5% of the time, when a higher level is left, it drops that level and every level below it instead.",
    );

    const cosmetic = await screen.findByTestId(purchasesTestid.cosmetic, undefined, SLOW);
    expect(screen.getByTestId(purchasesTestid.cosmeticPacksSummary)).toHaveTextContent(
      `${String(COSMETIC_PACK_SLOTS)} cosmetics · one is always ${COSMETIC_PACK_FLOOR_RARITY} or better`,
    );
    expect(within(cosmetic).getAllByRole("columnheader").map((head) => head.textContent)).toEqual([
      "Rarity",
      "Each slot",
      "Guaranteed slot",
    ]);
    expect(row(cosmetic, purchasesTestid.cosmeticRow, "common")).toHaveTextContent("Common50%—");
    expect(row(cosmetic, purchasesTestid.cosmeticRow, "rare")).toHaveTextContent("Rare30%—");
    expect(row(cosmetic, purchasesTestid.cosmeticRow, "epic")).toHaveTextContent("Epic15%90%");
    expect(row(cosmetic, purchasesTestid.cosmeticRow, "legendary")).toHaveTextContent("Legendary4%8%");
    expect(row(cosmetic, purchasesTestid.cosmeticRow, "mythic")).toHaveTextContent("Mythic1%2%");
    // R3015003: and a cosmetic pack slot's, its own read's.
    expect(screen.getByTestId(purchasesTestid.cosmeticLevelSkip)).toHaveTextContent(
      "A variant drops its next level. 3% of the time, when a higher level is left, it drops that level and every level below it instead.",
    );

    // The account's counters, once for every card pack (R1298000), with the server's thresholds.
    const pity = await screen.findByTestId(purchasesTestid.pity, undefined, SLOW);
    expect(screen.getByText("All card packs count toward the same pity.")).toBeInTheDocument();
    expect(within(pity).getAllByRole("columnheader").map((head) => head.textContent)).toEqual([
      "Rarity",
      "Guaranteed within",
      "Your progress",
    ]);
    expect(row(pity, purchasesTestid.pityRow, "epic")).toHaveTextContent("Epic10 packs7 / 10");
    expect(row(pity, purchasesTestid.pityRow, "legendary")).toHaveTextContent("Legendary20 packs13 / 20");
    expect(row(pity, purchasesTestid.pityRow, "mythic")).toHaveTextContent("Mythic40 packs21 / 40");
    expect(screen.queryByText(/Pity guarantees/u)).toBeNull();

    expect(screen.getByTestId(purchasesTestid.disclaimer)).toHaveTextContent("Jlore is never sold for real money.");
    expect(screen.queryByTestId(purchasesTestid.signIn)).toBeNull();

    expect(server.to("GET", "/api/packs")).toHaveLength(1);
    expect(server.to("GET", "/api/cosmetic-packs")).toHaveLength(1);
    for (const call of oddsReads(server)) expect(call.headers.authorization, call.path).toBe(`Bearer ${TOKEN}`);
  });

  it("R3223000 the screen has one visible h1, the PageTitle", async () => {
    servePurchases();
    signIn(TOKEN);
    render(<PurchasesRoute />);

    await screen.findByTestId(purchasesTestid.odds, undefined, SLOW);
    expectOnePageTitle("Purchases");
    expect(screen.getAllByRole("heading", { level: 2 }).length).toBeGreaterThan(0);
  });

  it("R1665000 a rarity with no chance shows a dash, and every figure is the server's", async () => {
    stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/packs") {
        const info = packsWithGlobalRoll();
        return {
          status: 200,
          body: {
            ...info,
            odds: {
              ...info.odds,
              duplicateCosmetic: { ...info.odds.duplicateCosmetic, common: 0 },
              globalCosmetic: {
                ...info.odds.globalCosmetic,
                tiers: { ...info.odds.globalCosmetic.tiers, mythic: 0.003 },
              },
            },
          } satisfies PacksResponse,
        };
      }
      return answers(call);
    });
    signIn(TOKEN);
    render(<PurchasesRoute />);

    const duplicate = await screen.findByTestId(purchasesTestid.duplicate, undefined, SLOW);
    expect(within(duplicate).getByTestId(purchasesTestid.duplicateRow("common"))).toHaveTextContent("Common——");
    expect(within(duplicate).getByTestId(purchasesTestid.duplicateRow("mythic"))).toHaveTextContent(
      /Mythic1%0\.3%$/u,
    );
  });

  it("#2115 shows a skeleton in each section until its read answers, then the odds and the pity", async () => {
    servePurchases();
    signIn(TOKEN);
    render(<PurchasesRoute />);

    // Before any await the stubbed reads have not answered.
    const loading = {
      "card-packs": screen.getByTestId(purchasesTestid.loading("card-packs")),
      duplicates: screen.getByTestId(purchasesTestid.loading("duplicates")),
      "cosmetic-packs": screen.getByTestId(purchasesTestid.loading("cosmetic-packs")),
      pity: screen.getByTestId(purchasesTestid.loading("pity")),
    };
    expect(loading["card-packs"]).toHaveTextContent("Loading your packs…");
    expectSkeleton(loading["card-packs"], "table");
    expect(loading.duplicates).toHaveTextContent("Loading your packs…");
    expectSkeleton(loading.duplicates, "table");
    expect(loading["cosmetic-packs"]).toHaveTextContent("Loading your cosmetic packs…");
    expectSkeleton(loading["cosmetic-packs"], "panel");
    expect(loading.pity).toHaveTextContent("Loading your packs…");
    expectSkeleton(loading.pity, "rows");

    expect(await screen.findByTestId(purchasesTestid.odds, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(purchasesTestid.duplicate, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(purchasesTestid.cosmetic, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(purchasesTestid.pity, undefined, SLOW)).toBeInTheDocument();
    for (const place of ["card-packs", "duplicates", "cosmetic-packs", "pity"] as const) {
      expect(screen.queryByTestId(purchasesTestid.loading(place)), place).toBeNull();
    }
    expect(screen.queryByText("Loading your packs…")).toBeNull();
    expect(screen.queryByText("Loading your cosmetic packs…")).toBeNull();
  });

  it("R1665000 a failed read says why in its place and leaves the other read's sections", async () => {
    stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/packs") return refusal(503, "internal", "the packs are closed");
      return answers(call);
    });
    signIn(TOKEN);
    render(<PurchasesRoute />);

    expect(await screen.findByTestId(purchasesTestid.loadError("card-packs"), undefined, SLOW)).toHaveTextContent(
      "the packs are closed",
    );
    expect(screen.getByTestId(purchasesTestid.loadError("duplicates"))).toHaveTextContent("the packs are closed");
    expect(screen.getByTestId(purchasesTestid.loadError("pity"))).toHaveTextContent("the packs are closed");
    expect(await screen.findByTestId(purchasesTestid.cosmetic, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(purchasesTestid.odds)).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.duplicate)).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.pity)).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.loadError("cosmetic-packs"))).toBeNull();
  });

  it("R1665000 a failed cosmetic packs read leaves the card packs' odds, the duplicates' and the pity", async () => {
    stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/cosmetic-packs") {
        return refusal(503, "internal", "the cosmetic packs are closed");
      }
      return answers(call);
    });
    signIn(TOKEN);
    render(<PurchasesRoute />);

    expect(await screen.findByTestId(purchasesTestid.loadError("cosmetic-packs"), undefined, SLOW)).toHaveTextContent(
      "the cosmetic packs are closed",
    );
    expect(await screen.findByTestId(purchasesTestid.odds, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(purchasesTestid.duplicate, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(purchasesTestid.pity, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(purchasesTestid.cosmetic)).toBeNull();
  });
});

describe("#1774 /purchases offers Try again", () => {
  it("#1774 a failed packs read offers Try again, which reads it again and shows the odds and the pity", async () => {
    let refused = false;
    const server = stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/packs" && !refused) {
        refused = true;
        return refusal(503, "internal", "the packs are closed");
      }
      return answers(call);
    });
    signIn(TOKEN);
    render(<PurchasesRoute />);

    expect(await screen.findByTestId(purchasesTestid.loadError("card-packs"), undefined, SLOW)).toHaveTextContent(
      "the packs are closed",
    );
    for (const place of ["card-packs", "duplicates", "pity"] as const) {
      expect(screen.getByTestId(purchasesTestid.loadError(place)), place).toHaveTextContent("the packs are closed");
      expect(screen.getByTestId(purchasesTestid.retry(place)), place).toBeEnabled();
      expect(screen.getByTestId(purchasesTestid.retry(place)), place).toHaveTextContent("Try again");
    }
    expect(await screen.findByTestId(purchasesTestid.cosmetic, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(purchasesTestid.retry("cosmetic-packs"))).toBeNull();

    fireEvent.click(screen.getByTestId(purchasesTestid.retry("card-packs")));

    const odds = await screen.findByTestId(purchasesTestid.odds, undefined, SLOW);
    expect(within(odds).getByTestId(purchasesTestid.oddsRow("rare"))).toHaveTextContent("Rare32%82%");
    expect(await screen.findByTestId(purchasesTestid.duplicateRow("epic"), undefined, SLOW)).toHaveTextContent(
      "Epic15%93%",
    );
    expect(await screen.findByTestId(purchasesTestid.pityRow("epic"), undefined, SLOW)).toHaveTextContent(
      "Epic10 packs7 / 10",
    );
    for (const place of ["card-packs", "duplicates", "pity"] as const) {
      expect(screen.queryByTestId(purchasesTestid.loadError(place)), place).toBeNull();
      expect(screen.queryByTestId(purchasesTestid.retry(place)), place).toBeNull();
    }
    expect(server.to("GET", "/api/packs")).toHaveLength(2);
    expect(server.to("GET", "/api/cosmetic-packs")).toHaveLength(1);
  });

  it("#1774 a rate-limited read says when to try again and holds Try again", async () => {
    const rateLimited: Reply = {
      status: 429,
      body: { error: { code: "rate_limited", message: "slow down", details: { retryAfterMs: 60_000 } } },
    };
    stubServer((call) => (call.method === "GET" && call.path === "/api/packs" ? rateLimited : answers(call)));
    signIn(TOKEN);
    render(<PurchasesRoute />);

    const error = await screen.findByTestId(purchasesTestid.loadError("card-packs"), undefined, SLOW);
    expect(error).toHaveTextContent("slow down");
    expect(error).toHaveTextContent(/You can try again in \d+ s\./);
    for (const place of ["duplicates", "pity"] as const) {
      expect(screen.getByTestId(purchasesTestid.loadError(place)), place).toHaveTextContent(
        /You can try again in \d+ s\./,
      );
    }
    for (const place of ["card-packs", "duplicates", "pity"] as const) {
      const retry = screen.getByTestId(purchasesTestid.retry(place));
      expect(retry, place).toBeDisabled();
      expect(retry, place).toHaveTextContent(/Try again in \d+ s/);
    }
  });
});

describe("R1665000 /purchases signed out", () => {
  it("R1665000 shows the disclaimer and a sign-in link, and makes no request", async () => {
    const server = servePurchases();
    render(<PurchasesRoute />);

    expect(screen.getByRole("heading", { name: "Purchases" })).toBeInTheDocument();
    expect(screen.getByTestId(purchasesTestid.disclaimer)).toHaveTextContent(
      "You buy packs with Jlore, which you earn by playing.",
    );
    expect(screen.getByTestId(purchasesTestid.disclaimer)).toHaveTextContent("Jlore is never sold for real money.");
    const link = within(screen.getByTestId(purchasesTestid.signIn)).getByRole("link", { name: "Sign in" });
    expect(link).toHaveAttribute("href", paths.login);
    expect(screen.queryByTestId(purchasesTestid.odds)).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.pity)).toBeNull();

    // Let any read the page might wrongly start go out before counting.
    await waitFor(() => {
      expect(screen.getByTestId(purchasesTestid.signIn)).toBeInTheDocument();
    });
    expect(server.calls).toHaveLength(0);
  });
});

describe("R1665000 /purchases is a page of its own", () => {
  it("R1665000 the footer links it", () => {
    render(<SiteFooter />);
    const link = screen.getByTestId(siteFooterTestid.purchases);
    expect(link).toHaveAttribute("href", paths.purchases);
    expect(link).toHaveTextContent("Purchases");
  });

  it("R1665000 the route table serves it without an account and the footer's link opens it", async () => {
    const server = servePurchases();
    at(paths.landing);
    render(<App />);

    fireEvent.click(await screen.findByTestId(siteFooterTestid.purchases, undefined, SLOW), { button: 0 });
    expect(await screen.findByTestId(purchasesTestid.screen, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe(paths.purchases);
    expect(await screen.findByTestId(purchasesTestid.signIn, undefined, SLOW)).toBeInTheDocument();
    expect(oddsReads(server)).toHaveLength(0);
  });

  it("R1665000 an active session at /purchases reads the odds with its token", async () => {
    const server = servePurchases();
    signIn(TOKEN);
    at(paths.purchases);
    render(<App />);

    expect(await screen.findByTestId(purchasesTestid.pity, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe(paths.purchases);
    expect(oddsReads(server)).toHaveLength(2);
    for (const call of oddsReads(server)) expect(call.headers.authorization, call.path).toBe(`Bearer ${TOKEN}`);
  });

  it("R1665000 names the page Purchases", () => {
    expect(paths.purchases).toBe("/purchases");
    expect(screenNameFor(paths.purchases)).toBe("Purchases");
    expect(documentTitleFor(paths.purchases)).toMatch(/^Purchases · /u);
  });
});
