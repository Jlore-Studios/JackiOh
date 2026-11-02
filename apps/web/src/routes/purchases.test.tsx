// `/purchases` (#1665, part of #1651, SPEC §14.4, R1665000) and the footer's link to it; a failed read
// offers Try again (#1774). Every figure below is the fixture's, none of them the shipped tables'.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PacksResponse } from "../net/api.ts";
import { documentTitleFor, screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { cosmeticPacksInfo } from "../shop/fixtures.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import { at, refusal, signIn, stubServer, type Call, type Reply, type StubServer } from "../test/stubServer.ts";
import PurchasesRoute, { purchasesTestid } from "./purchases.tsx";
import { SiteFooter, siteFooterTestid } from "./SiteFooter.tsx";

const { App, resetUsernamePromptForTests } = await import("../main.tsx");

/** A stubbed round trip, or a lazily-imported chunk, can outrun testing-library's 1 s default under load. */
const SLOW = { timeout: 5_000 } as const;

const TOKEN = "e2e-token";

/** `GET /api/packs` with the answer a duplicate's global roll adds (R1661001), the numbers the fixture's own. */
function packsWithGlobalRoll(): PacksResponse {
  const info = packsInfo();
  return {
    ...info,
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
  it("R1665000 shows the server's odds, the global roll's line and the pity counters", async () => {
    const server = servePurchases();
    signIn(TOKEN);
    render(<PurchasesRoute />);

    const odds = await screen.findByTestId(purchasesTestid.odds, undefined, SLOW);
    const row = (key: string): HTMLElement => within(odds).getByTestId(purchasesTestid.oddsRow(key));
    // A slot's odds and the guaranteed slot's, the fallback (Common) included, as percentages.
    expect(row("common")).toHaveTextContent("Common50%0%");
    expect(row("rare")).toHaveTextContent("Rare32%82%");
    expect(row("epic")).toHaveTextContent("Epic12.3%12.3%");
    expect(row("legendary")).toHaveTextContent("Legendary4.5%4.5%");
    expect(row("mythic")).toHaveTextContent("Mythic1.2%1.2%");

    expect(screen.getByTestId(purchasesTestid.duplicate)).toHaveTextContent(
      "A duplicate becomes a cosmetic: Common 50%, Rare 30%, Epic 15%, Legendary 4%, Mythic 1%.",
    );
    expect(screen.getByTestId(purchasesTestid.globalCosmetic)).toHaveTextContent(
      "a duplicate becomes one 12.5% of the time: Epic 93%, Legendary 6%, Mythic 1%.",
    );

    expect(await screen.findByTestId(purchasesTestid.cosmeticSlot, undefined, SLOW)).toHaveTextContent(
      "Each slot: Common 50%, Rare 30%, Epic 15%, Legendary 4%, Mythic 1%.",
    );
    expect(screen.getByTestId(purchasesTestid.cosmeticFloored)).toHaveTextContent(
      "Guaranteed slot: Common 0%, Rare 0%, Epic 90%, Legendary 8%, Mythic 2%.",
    );

    // The account's counters, once for every card pack (R1298000), with the server's thresholds.
    expect(await screen.findByTestId(purchasesTestid.pityRow("epic"), undefined, SLOW)).toHaveTextContent("7 / 10 packs");
    expect(screen.getByTestId(purchasesTestid.pityRow("legendary"))).toHaveTextContent("13 / 20 packs");
    expect(screen.getByTestId(purchasesTestid.pityRow("mythic"))).toHaveTextContent("21 / 40 packs");
    expect(screen.getByTestId(purchasesTestid.pityLine)).toHaveTextContent(
      "Pity guarantees an Epic within 10 packs, a Legendary within 20 packs and a Mythic within 40 packs.",
    );
    expect(screen.getByTestId(purchasesTestid.disclaimer)).toHaveTextContent("never sold");
    expect(screen.queryByTestId(purchasesTestid.signIn)).toBeNull();

    expect(server.to("GET", "/api/packs")).toHaveLength(1);
    expect(server.to("GET", "/api/cosmetic-packs")).toHaveLength(1);
    for (const call of oddsReads(server)) expect(call.headers.authorization, call.path).toBe(`Bearer ${TOKEN}`);
  });

  it("#2115 shows a skeleton in each section until its read answers, then the odds and the pity", async () => {
    servePurchases();
    signIn(TOKEN);
    render(<PurchasesRoute />);

    // Before any await the stubbed reads have not answered.
    const loading = {
      "card-packs": screen.getByTestId(purchasesTestid.loading("card-packs")),
      "cosmetic-packs": screen.getByTestId(purchasesTestid.loading("cosmetic-packs")),
      pity: screen.getByTestId(purchasesTestid.loading("pity")),
    };
    expect(loading["card-packs"]).toHaveTextContent("Loading your packs…");
    expectSkeleton(loading["card-packs"], "table");
    expect(loading["cosmetic-packs"]).toHaveTextContent("Loading your cosmetic packs…");
    expectSkeleton(loading["cosmetic-packs"], "panel");
    expect(loading.pity).toHaveTextContent("Loading your packs…");
    expectSkeleton(loading.pity, "rows");

    expect(await screen.findByTestId(purchasesTestid.odds, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(purchasesTestid.cosmeticSlot, undefined, SLOW)).toBeInTheDocument();
    expect(await screen.findByTestId(purchasesTestid.pityLine, undefined, SLOW)).toBeInTheDocument();
    for (const place of ["card-packs", "cosmetic-packs", "pity"] as const) {
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
    expect(screen.getByTestId(purchasesTestid.loadError("pity"))).toHaveTextContent("the packs are closed");
    expect(await screen.findByTestId(purchasesTestid.cosmeticSlot, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(purchasesTestid.odds)).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.pityLine)).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.loadError("cosmetic-packs"))).toBeNull();
  });

  it("R1665000 a failed cosmetic packs read leaves the card packs' odds and the pity", async () => {
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
    expect(await screen.findByTestId(purchasesTestid.pityLine, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(purchasesTestid.cosmeticSlot)).toBeNull();
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
    expect(screen.getByTestId(purchasesTestid.loadError("pity"))).toHaveTextContent("the packs are closed");
    for (const place of ["card-packs", "pity"] as const) {
      expect(screen.getByTestId(purchasesTestid.retry(place))).toBeEnabled();
      expect(screen.getByTestId(purchasesTestid.retry(place))).toHaveTextContent("Try again");
    }
    expect(await screen.findByTestId(purchasesTestid.cosmeticSlot, undefined, SLOW)).toBeInTheDocument();
    expect(screen.queryByTestId(purchasesTestid.retry("cosmetic-packs"))).toBeNull();

    fireEvent.click(screen.getByTestId(purchasesTestid.retry("card-packs")));

    const odds = await screen.findByTestId(purchasesTestid.odds, undefined, SLOW);
    expect(within(odds).getByTestId(purchasesTestid.oddsRow("rare"))).toHaveTextContent("Rare32%82%");
    expect(await screen.findByTestId(purchasesTestid.pityRow("epic"), undefined, SLOW)).toHaveTextContent(
      "7 / 10 packs",
    );
    expect(screen.getByTestId(purchasesTestid.pityLine)).toBeInTheDocument();
    expect(screen.queryByTestId(purchasesTestid.loadError("card-packs"))).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.loadError("pity"))).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.retry("card-packs"))).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.retry("pity"))).toBeNull();
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
    expect(screen.getByTestId(purchasesTestid.loadError("pity"))).toHaveTextContent(/You can try again in \d+ s\./);
    for (const place of ["card-packs", "pity"] as const) {
      const retry = screen.getByTestId(purchasesTestid.retry(place));
      expect(retry).toBeDisabled();
      expect(retry).toHaveTextContent(/Try again in \d+ s/);
    }
  });
});

describe("R1665000 /purchases signed out", () => {
  it("R1665000 shows the disclaimer and a sign-in link, and makes no request", async () => {
    const server = servePurchases();
    render(<PurchasesRoute />);

    expect(screen.getByRole("heading", { name: "Purchases" })).toBeInTheDocument();
    expect(screen.getByTestId(purchasesTestid.disclaimer)).toHaveTextContent(
      "Jlore is earned by playing and never sold: nothing here is bought with money.",
    );
    expect(screen.getByTestId(purchasesTestid.disclaimer)).toHaveTextContent("the server's seeded, logged roll");
    const link = within(screen.getByTestId(purchasesTestid.signIn)).getByRole("link", { name: "Sign in" });
    expect(link).toHaveAttribute("href", paths.login);
    expect(screen.queryByTestId(purchasesTestid.odds)).toBeNull();
    expect(screen.queryByTestId(purchasesTestid.pityLine)).toBeNull();

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

    expect(await screen.findByTestId(purchasesTestid.pityLine, undefined, SLOW)).toBeInTheDocument();
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
