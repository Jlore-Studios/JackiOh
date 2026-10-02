// `/almanac` (R630): the public Card Almanac. A route anyone reaches signed out, lazy like the
// others, with its tab title and canonical link, linked from the site footer beside Patch notes,
// served by vercel.json (net/deploy-routes.test.ts reads `paths`) and listed in the sitemap. It
// shows every catalog card, tokens included, through the deck builder's own browse pane, read-only,
// and asks no server anything. The pool's filter and sort semantics are filters.test.ts's.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { CardDef, Tag } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { INSPECT_CLOSE, INSPECT_DETAIL, closeInspect } from "../cards/index.ts";
import { ALMANAC_TAGS, DEFAULT_FILTER, DEFAULT_SORT, almanacPool, costBucket } from "../game/deckbuilder/filters.ts";
import { poolCardLabel } from "../game/deckbuilder/PoolGrid.tsx";
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
import { SITE_ORIGIN, paths } from "../net/navigate.ts";
import AlmanacRoute, { ALMANAC_CATALOG, almanacMeta, almanacTestid } from "./almanac.tsx";
import LandingRoute from "./landing.tsx";
import LoginRoute from "./login.tsx";
import { siteFooterTestid } from "./SiteFooter.tsx";

const { App, canonicalUrlFor, documentTitleFor } = await import("../main.tsx");

const HERE = dirname(fileURLToPath(import.meta.url));

/** A lazily-imported route chunk, and a pool of every card, can outrun the 1 s default. */
const SLOW = { timeout: 10_000 } as const;

const CARDS: readonly CardDef[] = Object.values(ALMANAC_CATALOG.cards);
const TOKENS: readonly CardDef[] = CARDS.filter((def) => def.token);

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

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
  document.head.querySelector('link[rel="canonical"]')?.remove();
});

describe("R630 the /almanac route", () => {
  it("R630 is served at /almanac signed out, with no redirect and no call to the server", async () => {
    at(paths.almanac);
    render(<App />);
    expect(await screen.findByTestId(almanacTestid.screen, undefined, SLOW)).toBeInTheDocument();
    expect(window.location.pathname).toBe(paths.almanac);
    expect(document.title).toBe("Almanac · JackiOh");
    // Not behind the gate, and no catalog, collection, deck or account read.
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
    const sitemap = readFileSync(join(HERE, "../../public/sitemap.xml"), "utf8");
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

describe("R630 the almanac's browse pane", () => {
  it("R630 shows every catalog card, tokens included, cost ascending with no filter", () => {
    render(<AlmanacRoute />);
    const ids = shownIds();
    expect(ids).toEqual([...almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, DEFAULT_SORT)]);
    expect([...ids].sort()).toEqual(Object.keys(ALMANAC_CATALOG.cards).sort());
    expect(TOKENS.length).toBeGreaterThan(0);
    for (const token of TOKENS) expect(ids, token.id).toContain(token.id);
    expect(screen.getByTestId(DB_RESULT_COUNT)).toHaveAttribute("data-count", String(CARDS.length));
    expect(screen.queryByTestId(DB_EMPTY)).toBeNull();
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
    // No deck sidebar beside it.
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
      expect(label, id).toBe(poolCardLabel(cardOf(id)));
      expect(label, id).toMatch(/\. Show details$/u);
      expect(label, id).not.toMatch(/deck|collection|unavailable/iu);
    }
  });

  it("R630 a click, a right-click or a long-press opens the card's detail, with its rarity and no add action", () => {
    render(<AlmanacRoute />);
    const unit = CARDS.find((def) => !def.token && def.set === "Core");
    if (unit === undefined) throw new Error("the catalog has no Core card");
    fireEvent.click(screen.getByTestId(poolCardId(unit.id)));
    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", unit.id);
    expect(within(detail).getByText(unit.rarity, { selector: ".db-detail-meta" })).toBeInTheDocument();
    expect(screen.queryByTestId(DB_DETAIL_ADD)).toBeNull();
    // Close is the only action.
    expect(within(detail).getAllByRole("button", { name: "Close" })).toHaveLength(1);
    fireEvent.click(within(detail).getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();

    fireEvent.contextMenu(screen.getByTestId(poolCardId(unit.id)));
    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveAttribute("data-card", unit.id);
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

  // One filter per test: a full pool is 317 cards, and every render of all of them costs about as
  // much as the rest of a test does.
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
    for (const tag of ALMANAC_TAGS) expect(screen.getByTestId(filterTagId(tag)), tag).toBeInTheDocument();
    fireEvent.click(screen.getByTestId(filterTagId("Token" satisfies Tag)));
    expect([...shownIds()].sort()).toEqual(TOKENS.map((def) => def.id).sort());
  });

  it("R630 a rarity chip keeps the cards of that rarity, and Clear filters shows every card again", () => {
    render(<AlmanacRoute />);
    fireEvent.click(screen.getByTestId(filterRarityId("Mythic")));
    expect(shownIds().length).toBeGreaterThan(0);
    for (const id of shownIds()) expect(cardOf(id).rarity, id).toBe("Mythic");
    fireEvent.click(screen.getByTestId(DB_FILTER_CLEAR));
    expect(shownIds()).toHaveLength(CARDS.length);
  });

  it("R630 the sort orders the pool", () => {
    render(<AlmanacRoute />);
    fireEvent.change(screen.getByTestId(DB_SORT), { target: { value: "name" } });
    expect(shownIds()).toEqual([...almanacPool(ALMANAC_CATALOG, DEFAULT_FILTER, { key: "name", dir: "asc" })]);
    expect(screen.getByTestId(DB_SORT_DIR)).toHaveAttribute("data-dir", "asc");
  });
});
