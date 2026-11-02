// `/almanac` (R630): the public Card Almanac, reached signed out, lazy, with its tab title and
// canonical link, linked from the footer, served by vercel.json (net/deploy-routes.test.ts reads
// `paths`) and in the sitemap: every card through the deck builder's browse pane, read-only, asking
// no server anything but the R654 public aggregates. Filter and sort semantics are filters.test.ts's.

import type { CardDef, Tag } from "@jackioh/shared";
import { SHIPPED_SETS, setShips } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, renderHook, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { GLITCH_DEF_ID } from "@jackioh/engine/config";

import { PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { CHINESE } from "../cards/chinese.ts";
import { resetCosmeticsForTests, writeCardPick } from "../cards/cosmetics.ts";
import { INSPECT_CLOSE, INSPECT_DETAIL, closeInspect } from "../cards/index.ts";
import { CARD_FLAVOUR } from "../cards/flavour.ts";
import { INSPECT_FLAVOUR, INSPECT_STATS } from "../cards/inspect/testids.ts";
import { ALMANAC_CHIP_TAGS, DEFAULT_FILTER, DEFAULT_SORT, almanacPool, costBucket } from "../game/deckbuilder/filters.ts";
import { poolCardLabel } from "../game/deckbuilder/PoolGrid.tsx";
import { resetLocaleForTests, setLocale } from "../i18n/i18n.ts";
import {
  CARD_POOL,
  DB_DETAIL_ADD,
  DB_EMPTY,
  DB_FILTER_CLEAR,
  DB_FILTER_OWNED,
  DB_FILTERS,
  DB_RESULT_COUNT,
  DB_SEARCH,
  DB_SORT,
  DB_SORT_DIR,
  filterCostId,
  filterRarityId,
  filterTagId,
  filterTypeId,
  poolCardId,
} from "../game/deckbuilder/testids.ts";
import { navigate, SITE_ORIGIN, paths } from "../net/navigate.ts";
import { NEW_RIBBON_SET } from "../patches/ribbon.ts";
import { collectionOwning } from "../collection/fixtures.ts";
import { at, meBody, signIn, stubServer } from "../test/stubServer.ts";
import AlmanacRoute, { ALMANAC_CATALOG, almanacMeta, almanacTestid } from "./almanac.tsx";
import { useAlmanacOwnCopies } from "./almanacOwnership.ts";
import LandingRoute from "./landing.tsx";
import LoginRoute from "./login.tsx";
import { siteFooterTestid } from "./SiteFooter.tsx";

const { App, canonicalUrlFor, documentTitleFor } = await import("../main.tsx");

/** A lazily-imported route chunk, and a pool of every card, can outrun the 1 s default. */
const SLOW = { timeout: 10_000 } as const;

/** R674, R1420: every card the almanac shows: the catalog's from the sets that ship, Glitch excepted. */
const CARDS: readonly CardDef[] = Object.values(ALMANAC_CATALOG.cards).filter(
  (def) => def.id !== GLITCH_DEF_ID && setShips(def.set),
);
const TOKENS: readonly CardDef[] = CARDS.filter((def) => def.token);

function cardOf(id: string): CardDef {
  const def = ALMANAC_CATALOG.cards[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def;
}

/** The ids the grid shows, in its order. */
function shownIds(): string[] {
  return [...screen.getByTestId(CARD_POOL).querySelectorAll<HTMLElement>(".db-card")].map((card) => card.dataset.card ?? "");
}

beforeEach(() => {
  window.localStorage.clear();
  at("/");
  // An account read that never answers: a screen that asked the server would sit on it.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  closeInspect();
  cleanup();
  vi.unstubAllGlobals();
  resetLocaleForTests();
  document.head.querySelector('link[rel="canonical"]')?.remove();
});

/** The whole grid (710 cards and tokens since #1884), clicked through, takes up to 8 s under jsdom. */
const SLOW_GRID = { timeout: 30_000 } as const;

describe("R630 the /almanac route", SLOW_GRID, () => {
  it("R630 is served at /almanac signed out, with no redirect and no call to the server", async () => {
    at(paths.almanac);
    render(<App />);
    expect(await screen.findByTestId(almanacTestid.screen, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe(paths.almanac);
    expect(document.title).toBe("Almanac · JackiOh");
    expect(vi.mocked(fetch)).not.toHaveBeenCalled();
  });

  it("R630 has a tab title, a canonical link, a heading and Back", () => {
    expect(paths.almanac).toBe("/almanac");
    expect(documentTitleFor(paths.almanac)).toBe("Almanac · JackiOh");
    expect(canonicalUrlFor(paths.almanac)).toBe(`${SITE_ORIGIN}/almanac`);
    render(<AlmanacRoute />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Almanac");
    expect(screen.getByRole("main")).toHaveAccessibleName("Almanac");
    expect(screen.getByTestId("nav-back")).toBeInTheDocument();
  });

  it("R630 is in the sitemap", () => {
    const sitemap = sitemapXml(PUBLIC_PAGES);
    expect(sitemap).toContain(`<loc>${SITE_ORIGIN}/almanac</loc>`);
  });

  it("R630 the site footer links it beside Patch notes on the landing page and the sign-in screen, and opens it in place", async () => {
    render(<LandingRoute />);
    const footer = screen.getByTestId(siteFooterTestid.root);
    const link = within(footer).getByTestId(siteFooterTestid.almanac);
    expect(link).toHaveAttribute("href", paths.almanac);
    expect(link).toHaveTextContent("Card almanac");
    expect(link.previousElementSibling).toBe(within(footer).getByTestId(siteFooterTestid.patchNotes));
    cleanup();

    render(<LoginRoute />);
    expect(within(screen.getByTestId(siteFooterTestid.root)).getByTestId(siteFooterTestid.almanac)).toBeInTheDocument();
    cleanup();

    at(paths.landing);
    render(<App />);
    await userEvent.click(screen.getByTestId(siteFooterTestid.almanac));
    expect(window.location.pathname).toBe(paths.almanac);
    expect(await screen.findByTestId(almanacTestid.screen, undefined, SLOW)).toBeInTheDocument();
  });
});

describe("R630 the almanac's browse pane", SLOW_GRID, () => {
  it("R630 shows every catalog card, tokens included, cost ascending with no filter", () => {
    render(<AlmanacRoute />);
    const ids = shownIds();
    expect(ids).toEqual([...almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, DEFAULT_SORT)]);
    expect([...ids].sort()).toEqual(CARDS.map((def) => def.id).sort());
    expect(TOKENS.length).toBeGreaterThan(0);
    for (const token of TOKENS) expect(ids, token.id).toContain(token.id);
    expect(screen.getByTestId(DB_RESULT_COUNT)).toHaveAttribute("data-count", String(CARDS.length));
    expect(screen.queryByTestId(DB_EMPTY)).toBeNull();
  });

  it("R674 never shows Glitch, though the catalog holds it", () => {
    expect(ALMANAC_CATALOG.cards[GLITCH_DEF_ID]).toBeDefined();
    render(<AlmanacRoute />);
    expect(shownIds()).not.toContain(GLITCH_DEF_ID);
    expect(almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, DEFAULT_SORT)).not.toContain(GLITCH_DEF_ID);
  });

  it("R630 renders the deck builder's own filter bar, pool and look", () => {
    render(<AlmanacRoute />);
    const root = screen.getByTestId(almanacTestid.screen);
    expect(root).toHaveClass("app-shell", "app-shell--wide", "tavern", "deckbuilder", "almanac");
    expect(root.querySelector(".db-header .db-title")).toHaveTextContent("Almanac");
    const browse = root.querySelector("section.db-browse");
    expect(browse).not.toBeNull();
    expect(within(browse as HTMLElement).getByTestId(DB_FILTERS)).toBeInTheDocument();
    expect(within(browse as HTMLElement).getByTestId(CARD_POOL).closest(".db-pool-frame")).not.toBeNull();
    expect(root.querySelector(".db-layout, .db-sidebar")).toBeNull();
  });

  it("R630 is read-only: no owned control, no +, no drag, no deck or ownership marks", () => {
    render(<AlmanacRoute />);
    expect(screen.queryByTestId(DB_FILTER_OWNED)).toBeNull();
    expect(screen.queryByText("Owned only")).toBeNull();
    const pool = screen.getByTestId(CARD_POOL);
    expect(pool.querySelector(".db-add, [data-testid^='db-add-']")).toBeNull();
    const cards = [...pool.querySelectorAll<HTMLElement>(".db-card")];
    expect(cards).toHaveLength(CARDS.length);
    for (const card of cards) {
      const id = card.dataset.card ?? "";
      expect(card.getAttribute("draggable"), id).toBeNull();
      for (const attribute of ["data-legal", "data-in-deck", "data-unavailable", "data-held-by", "data-refused", "data-owned", "aria-disabled"]) {
        expect(card.hasAttribute(attribute), `${id} ${attribute}`).toBe(false);
      }
      const label = card.getAttribute("aria-label") ?? "";
      expect(label, id).toBe(poolCardLabel(cardOf(id), null, null, cardOf(id).set === NEW_RIBBON_SET));
      expect(label, id).toMatch(/\. Show details$/u);
      expect(label, id).not.toMatch(/deck|collection|unavailable/iu);
    }
  });

  // A long-press goes through the same `useInspectTrigger` as the right-click, held by browse.test.tsx.
  it("R630 a click or a right-click opens the card's detail, with its rarity and no add action", () => {
    render(<AlmanacRoute />);
    const unit = CARDS.find((def) => !def.token && def.set === "Core");
    if (unit === undefined) throw new Error("the catalog has no Core card");
    fireEvent.click(screen.getByTestId(poolCardId(unit.id)));
    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", unit.id);
    expect(within(detail).getByText(unit.rarity, { selector: ".db-detail-meta" })).toBeInTheDocument();
    expect(screen.queryByTestId(DB_DETAIL_ADD)).toBeNull();
    expect(within(detail).getAllByRole("button", { name: "Close" })).toHaveLength(1);
    fireEvent.click(within(detail).getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();

    fireEvent.contextMenu(screen.getByTestId(poolCardId(unit.id)));
    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveAttribute("data-card", unit.id);
  });

  it("opens the detail a ?card= link names, a token's too, and the URL follows the detail opened and closed", () => {
    at(paths.almanacCard("core-t-rush"));
    render(<AlmanacRoute />);
    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveAttribute("data-card", "core-t-rush");
    fireEvent.click(within(screen.getByTestId(INSPECT_DETAIL)).getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    expect(`${window.location.pathname}${window.location.search}`).toBe(paths.almanac);

    fireEvent.click(screen.getByTestId(poolCardId("core-001")));
    expect(`${window.location.pathname}${window.location.search}`).toBe(paths.almanacCard("core-001"));
    cleanup();

    at(paths.almanacCard("no-such-card"));
    render(<AlmanacRoute />);
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
  });

  it("R660 a card's detail shows its flavour line from the sidecar, a token's too", () => {
    render(<AlmanacRoute />);
    for (const id of ["core-001", "core-t-sheep"]) {
      fireEvent.click(screen.getByTestId(poolCardId(id)));
      const flavour = within(screen.getByTestId(INSPECT_DETAIL)).getByTestId(INSPECT_FLAVOUR);
      expect(flavour).toHaveTextContent(CARD_FLAVOUR[id]?.flavour ?? "missing");
      closeInspect();
    }
  });

  it("R630 a token's detail says it is a token and never goes in a deck", () => {
    render(<AlmanacRoute />);
    const plain = TOKENS.find((def) => def.printedRarity === undefined);
    const printed = TOKENS.find((def) => def.printedRarity !== undefined);
    if (plain === undefined || printed === undefined) throw new Error("the catalog's tokens changed shape");
    expect(almanacMeta(plain)).toBe("Token · not deckable");
    expect(almanacMeta(printed)).toBe(`${String(printed.printedRarity)} · Token · not deckable`);
    expect(almanacMeta(cardOf("core-001"))).toBe(cardOf("core-001").rarity);

    fireEvent.click(screen.getByTestId(poolCardId(plain.id)));
    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(detail.querySelector(".db-detail-meta")).toHaveTextContent("Token · not deckable");
    expect(screen.queryByTestId(DB_DETAIL_ADD)).toBeNull();
  });

  it("R1177001 the grid names a token that prints a rarity by both, Token and the printed rarity, and every token as Token", () => {
    render(<AlmanacRoute />);
    for (const token of TOKENS) {
      const label = screen.getByTestId(poolCardId(token.id)).getAttribute("aria-label") ?? "";
      const rarity = token.printedRarity === undefined ? "Token" : `Token, printed ${token.printedRarity}`;
      expect(label, token.id).toContain(`, ${rarity}`);
      if (token.printedRarity === undefined) expect(label, token.id).not.toContain("printed");
    }
    const top = cardOf("classicplus-019-1");
    expect(poolCardLabel(top)).toBe(`${top.name}, Unit, (2) Cost, Token, printed Legendary. Show details`);
    expect(poolCardLabel(cardOf("core-t-rush"))).toBe("Rush Token, Unit, (1) Cost, Token. Show details");
  });

  // One filter per test: each render of a full pool (317 cards) costs about as much as a whole test.
  it("R630 the search narrows the pool, and a search nothing matches shows the empty state", () => {
    render(<AlmanacRoute />);
    const named = cardOf("core-001");
    fireEvent.change(screen.getByTestId(DB_SEARCH), { target: { value: named.name } });
    expect(shownIds()).toContain(named.id);
    expect(shownIds().length).toBeLessThan(CARDS.length);
    fireEvent.change(screen.getByTestId(DB_SEARCH), { target: { value: "zzz-no-card-says-this" } });
    expect(shownIds()).toEqual([]);
    expect(screen.getByTestId(DB_EMPTY)).toBeInTheDocument();
  });

  it("R630 a cost chip keeps the cards of that cost", () => {
    render(<AlmanacRoute />);
    fireEvent.click(screen.getByTestId(filterCostId("6+")));
    expect(shownIds().length).toBeGreaterThan(0);
    for (const id of shownIds()) expect(costBucket(cardOf(id).cost), id).toBe("6+");
  });

  it("R630 a type chip keeps the cards of that type", () => {
    render(<AlmanacRoute />);
    fireEvent.click(screen.getByTestId(filterTypeId("Trap")));
    expect(shownIds().length).toBeGreaterThan(0);
    for (const id of shownIds()) expect(cardOf(id).type, id).toBe("Trap");
  });

  it("R630 every almanac tag has a chip, and the Token chip keeps exactly the tokens", () => {
    render(<AlmanacRoute />);
    for (const tag of ALMANAC_CHIP_TAGS) expect(screen.getByTestId(filterTagId(tag)), tag).toBeInTheDocument();
    fireEvent.click(screen.getByTestId(filterTagId("Token" satisfies Tag)));
    expect([...shownIds()].sort()).toEqual(TOKENS.map((def) => def.id).sort());
  });

  it("R1380 R1381 the set chips are the sets that ship and the tag chips the tags their cards carry", () => {
    render(<AlmanacRoute />);
    const chips = (group: string): string[] =>
      within(within(screen.getByTestId(DB_FILTERS)).getByRole("group", { name: group }))
        .getAllByRole("button")
        .map((chip) => chip.textContent ?? "");
    // A set the catalog holds before it ships has no chip; Meditative's arrives with its patch.
    expect(chips("Set")).toEqual([...SHIPPED_SETS]);
    expect(chips("Set").includes("Meditative")).toBe(setShips("Meditative"));
    expect(chips("Tag")).toEqual([...ALMANAC_CHIP_TAGS]);
    // Wincon has a chip only if a card on the shelf carries it.
    expect(chips("Tag").includes("Wincon")).toBe(CARDS.some((def) => def.tags.includes("Wincon")));
  });

  it("R630 a rarity chip keeps the cards of that rarity, and Clear filters shows every card again", () => {
    render(<AlmanacRoute />);
    fireEvent.click(screen.getByTestId(filterRarityId("Mythic")));
    expect(shownIds().length).toBeGreaterThan(0);
    for (const id of shownIds()) expect(cardOf(id).rarity, id).toBe("Mythic");
    fireEvent.click(screen.getByTestId(DB_FILTER_CLEAR));
    expect(shownIds()).toHaveLength(CARDS.length);
  });

  it("R630 the sort key and its direction order the pool", () => {
    render(<AlmanacRoute />);
    fireEvent.change(screen.getByTestId(DB_SORT), { target: { value: "name" } });
    expect(shownIds()).toEqual([...almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, { key: "name", dir: "asc" })]);
    fireEvent.click(screen.getByTestId(DB_SORT_DIR));
    expect(screen.getByTestId(DB_SORT_DIR)).toHaveAttribute("data-dir", "desc");
    expect(shownIds()).toEqual([...almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, { key: "name", dir: "desc" })]);
  });

  it("R711000 under zh-CN the search finds a card by its Chinese name, live across a switch and back", async () => {
    render(<AlmanacRoute />);
    fireEvent.change(screen.getByTestId(DB_SEARCH), { target: { value: CHINESE["core-002"]?.name ?? "" } });
    expect(shownIds()).toEqual([]);
    await act(async () => {
      await setLocale("zh-CN");
    });
    expect(shownIds()).toContain("core-002");
    await act(async () => {
      await setLocale("en");
    });
    expect(shownIds()).toEqual([]);
  });
});

describe("R654 the almanac's card statistics block", SLOW_GRID, () => {
  it("R654 R661 the almanac's detail view renders the compact statistics block, names no source, and links the full stats page, in place on a plain click and left to the browser on a ctrl or meta click", async () => {
    const unit = CARDS.find((def) => !def.token && def.set === "Core");
    if (unit === undefined) throw new Error("the catalog has no Core card");
    vi.mocked(fetch).mockImplementation((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.includes("/api/stats/cards")) {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              patch: "0.2.0",
              previousPatch: null,
              gate: { cleared: false, liveGames: 412, minLiveGames: 1000 },
              source: "provisional",
              sourceLabel: "AI games + live games (provisional)",
              minSample: 20,
              totalGames: 1200,
              cards: [
                {
                  id: unit.id,
                  name: unit.name,
                  cost: unit.cost,
                  rarity: unit.rarity,
                  set: unit.set,
                  games: 50,
                  winRate: 0.6,
                  drawnGames: 40,
                  drawnWinRate: 0.65,
                  playedGames: 30,
                  playedWinRate: 0.7,
                  playRate: 0.5,
                  hasEnoughGames: true,
                },
              ],
              summary: {
                totalGames: 1200,
                liveGames: 412,
                activePatch: "0.2.0",
                source: "provisional",
                bestCard: null,
                worstCard: null,
              },
            }),
            { status: 200 },
          ),
        );
      }
      return Promise.resolve(new Response("{}", { status: 200 }));
    });
    render(<AlmanacRoute />);
    fireEvent.click(screen.getByTestId(poolCardId(unit.id)));
    const stats = await screen.findByTestId(INSPECT_STATS, undefined, SLOW);
    // The skeleton already wears the block's test id: wait for the figures to land.
    await waitFor(() => expect(screen.getByTestId(INSPECT_STATS)).toHaveTextContent("60%"), SLOW);
    expect(stats).toHaveTextContent("60%");
    // R661: the response still names its source, but the block never shows it.
    expect(stats.textContent).not.toContain("AI games + live games (provisional)");
    expect(stats.querySelector(".inspect-stats-badge")).toBeNull();
    const link = within(screen.getByTestId(INSPECT_DETAIL)).getByRole("link", { name: /view full stats/i });
    const fullStats = `/stats?tab=cards&card=${encodeURIComponent(unit.id)}`;
    expect(link).toHaveAttribute("href", fullStats);
    expect(paths.statsCard(unit.id)).toBe(fullStats);
    // #2562: a ctrl or meta click is the browser's; a plain one moves in place, with no page load.
    const here = `${window.location.pathname}${window.location.search}`;
    expect(here).not.toBe(fullStats);
    expect(fireEvent.click(link, { button: 0, ctrlKey: true })).toBe(true);
    expect(fireEvent.click(link, { button: 0, metaKey: true })).toBe(true);
    expect(`${window.location.pathname}${window.location.search}`).toBe(here);
    expect(fireEvent.click(link, { button: 0 })).toBe(false);
    expect(`${window.location.pathname}${window.location.search}`).toBe(fullStats);
  });
});

describe("R1956000 the viewer's own cards in the Almanac", SLOW_GRID, () => {
  const OWNED = "core-002";
  const UNOWNED = "core-004";
  /** The frame a pool card's face wears, null for the default. */
  const frameOf = (id: string): string | null =>
    screen.getByTestId(poolCardId(id)).querySelector(".cf")?.getAttribute("data-frame") ?? null;

  beforeEach(() => {
    resetCosmeticsForTests();
    for (const id of [OWNED, UNOWNED]) {
      writeCardPick(id, "border", "border-bone");
      writeCardPick(id, "finish", "finish-linen");
    }
  });

  afterEach(() => {
    resetCosmeticsForTests();
  });

  it("R1956000 signed out, the Almanac reads nothing and draws every card with the defaults", () => {
    render(<AlmanacRoute />);
    expect(vi.mocked(fetch)).not.toHaveBeenCalled();
    const pool = screen.getByTestId(CARD_POOL);
    expect(pool.querySelectorAll(".cf").length).toBe(CARDS.length);
    expect(pool.querySelectorAll(".cf[data-frame], .cf[data-finish], .cf[data-cosmetic-foil]")).toHaveLength(0);
  });

  it("R1956000 signed in, a card the collection holds wears the viewer's picks and every other card the defaults", async () => {
    signIn();
    const server = stubServer((call) => {
      if (call.path === "/api/auth/me") return { status: 200, body: meBody() };
      if (call.path === "/api/collection") return { status: 200, body: collectionOwning([OWNED]) };
      return undefined;
    });
    render(<AlmanacRoute />);
    await waitFor(() => expect(frameOf(OWNED)).toBe("border-bone"), SLOW);
    expect(screen.getByTestId(poolCardId(OWNED)).querySelector(".cf")?.getAttribute("data-finish")).toBe("finish-linen");
    expect(frameOf(UNOWNED)).toBeNull();
    expect(screen.getByTestId(CARD_POOL).querySelectorAll(".cf[data-finish]")).toHaveLength(1);
    // The account and its collection, with the token; a build where ownership does not decide reads no ledger.
    expect(server.to("GET", "/api/collection")).toHaveLength(1);
    expect(server.to("GET", "/api/collection")[0]?.headers["authorization"]).toBe("Bearer e2e-token");
    expect(server.to("GET", "/api/cosmetics")).toHaveLength(0);
  });

  it("R1956000 useAlmanacOwnCopies reads the collection and, while gated, the ledger for a token, and nothing for none", async () => {
    const ledger = new Map([[OWNED, new Set(["foil-glitter"])]]);
    const readCollection = vi.fn(() => Promise.resolve(collectionOwning([OWNED, UNOWNED])));
    const readOwned = vi.fn(() => Promise.resolve(ledger));
    const reads = { gated: true, readCollection, readOwned };
    const signedOut: { token: string | null } = { token: null };
    const { result, rerender } = renderHook(({ token }: { token: string | null }) => useAlmanacOwnCopies(token, reads), {
      initialProps: signedOut,
    });
    expect(result.current).toBeNull();
    expect(readCollection).not.toHaveBeenCalled();
    expect(readOwned).not.toHaveBeenCalled();

    rerender({ token: "token-a" });
    await waitFor(() => expect(result.current).not.toBeNull());
    expect(result.current?.cards).toEqual(new Set([OWNED, UNOWNED]));
    expect(result.current?.owned).toBe(ledger);
    expect(readCollection).toHaveBeenCalledWith("token-a");
    expect(readOwned).toHaveBeenCalledWith("token-a");

    // Signed out again: no card is the viewer's.
    rerender({ token: null });
    expect(result.current).toBeNull();

    // Before ownership decides, every cosmetic counts as owned and no ledger is read.
    const ungatedOwned = vi.fn(() => Promise.resolve(ledger));
    const ungated = { gated: false, readCollection, readOwned: ungatedOwned };
    const open = renderHook(() => useAlmanacOwnCopies("token-b", ungated));
    await waitFor(() => expect(open.result.current).not.toBeNull());
    expect(open.result.current?.owned).toBe("every");
    expect(ungatedOwned).not.toHaveBeenCalled();

    // A read that fails leaves every card drawn as before.
    const failing = { gated: true, readCollection: () => Promise.reject(new Error("down")), readOwned };
    const failed = renderHook(() => useAlmanacOwnCopies("token-c", failing));
    await act(async () => {
      await Promise.resolve();
    });
    expect(failed.result.current).toBeNull();
  });
});

describe("R2118000 the command menu's card detail", SLOW_GRID, () => {
  it("R2118000 a ?card= move made while the page is open opens that card's detail", () => {
    at(paths.almanac);
    render(<AlmanacRoute />);
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
    act(() => {
      navigate(paths.almanacCard("core-001"));
    });
    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveAttribute("data-card", "core-001");
  });
});
