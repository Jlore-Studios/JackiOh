// #1307: the collection screen fits a phone. jsdom cannot measure layout, so this mounts the route
// in a browser, serves its reads from the fixtures, and checks nothing is wider than the window.
// R1579000, R3734000: each of its four tabs (Cards, Decks, Cosmetics and Progress) fits one screen at a phone's two sizes,
// an upright and a landscape iPad's and a desktop's, only a tab's own body scrolling; the Cards tab's grid is wider than the
// old 960 px frame at 1280 with its filters in a sidebar to its left, and a Filters button in place of it from 1024 px down, and
// mounts only the cards near the view. A tab is never clicked here: `navigate` would move the iframe's own path,
// so each test sets `?section=` itself.

import { newestShippedSet } from "@jackioh/shared";

import { collectionTestid, COLLECTED_DEFS, type CollectionSection } from "../../../apps/web/src/collection/collection.ts";
import {
  fullCollection,
  questsHeld,
  tracksOf,
} from "../../../apps/web/src/collection/fixtures.ts";
import { currentCatalog } from "../../../apps/web/src/game/currentCatalog.ts";
import { poolFrom } from "../../../apps/web/src/game/deckbuilder/loadout.ts";
import { decksResponse, savedDeck } from "../../../apps/web/src/game/deckbuilder/testkit.ts";
import { DB_SEARCH } from "../../../apps/web/src/game/deckbuilder/testids.ts";
import { packsInfo } from "../../../apps/web/src/packs/fixtures.ts";
import CollectionRoute from "../../../apps/web/src/routes/collection.tsx";
import { cosmeticPacksInfo, ownedCosmeticsOf, walletInfo } from "../../../apps/web/src/shop/fixtures.ts";

const VIEWPORTS = [
  { width: 360, height: 740 },
  { width: 390, height: 844 },
  { width: 820, height: 1180 },
  { width: 1180, height: 820 },
  { width: 1280, height: 800 },
] as const;

type Viewport = (typeof VIEWPORTS)[number];

/** `tablet` of breakpoints.ts: the filters are a sidebar over this width, a sheet at it and under. */
const TABLET_MAX_PX = 1024;

/** The reading column the Collection left (R3734000): its old frame. */
const OLD_FRAME_PX = 960;

function byTestid(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** Three decks of real cards, for the Decks tab. */
function decksFixture(): unknown {
  const { cards, version } = currentCatalog();
  const pool = poolFrom({ version, cards }, null);
  return decksResponse(
    [0, 1, 2, 3].map((at) => savedDeck(`d-${String(at)}`, `Deck ${String(at)}`, pool.slice(at * 25, at * 25 + 25 - at * 5), at + 1, version)),
    [],
    version,
  );
}

/** What each read the screen makes answers; any other request goes on to the network. */
function serveFixtures(): void {
  const bodies: Record<string, unknown> = {
    "/api/collection": fullCollection(),
    "/api/decks": decksFixture(),
    "/api/wallet": walletInfo(),
    "/api/packs": packsInfo(),
    "/api/cosmetic-packs": cosmeticPacksInfo(),
    "/api/tracks": tracksOf(),
    "/api/quests": questsHeld(),
    "/api/cosmetics": ownedCosmeticsOf(),
  };
  cy.window({ log: false }).then((win) => {
    const passThrough = win.fetch.bind(win);
    cy.stub(win, "fetch").callsFake((input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof win.Request ? input.url : String(input);
      const body = bodies[new URL(url, win.location.href).pathname];
      if (body === undefined) return passThrough(input, init);
      return Promise.resolve(
        new win.Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } }),
      );
    });
  });
}

function expectNoHorizontalOverflow(viewport: Viewport): void {
  cy.window({ log: false }).should((win) => {
    const where = `${String(viewport.width)}x${String(viewport.height)}`;
    expect(win.innerWidth, "the viewport is the one asked for").to.eq(viewport.width);
    expect(win.document.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(win.innerWidth);
    expect(win.document.body.scrollWidth, `the body fits ${where}`).to.be.at.most(win.innerWidth);
  });
}

/**
 * Sets (or removes) only `?section=` of the iframe's own address, before the route mounts; the
 * parameters the runner put there (`specPath`) stay.
 */
function openAt(section: CollectionSection | null): void {
  cy.window({ log: false }).then((win) => {
    const url = new URL(win.location.href);
    if (section === null) url.searchParams.delete("section");
    else url.searchParams.set("section", section);
    win.history.replaceState(null, "", url);
  });
}

function expectNoPageScroll(viewport: Viewport): void {
  cy.window({ log: false }).should((win) => {
    const where = `${String(viewport.width)}x${String(viewport.height)}`;
    expect(win.document.documentElement.scrollHeight, `the page does not scroll at ${where}`).to.be.at.most(win.innerHeight);
  });
}

describe("R3734000 the collection screen's tabs fit every size", () => {
  afterEach(() => {
    openAt(null);
  });

  for (const viewport of VIEWPORTS) {
    const size = `${String(viewport.width)}x${String(viewport.height)}`;

    it(`R3734000 Cards fits one screen at ${size}: the grid scrolls inside it and mounts only the cards near the view`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt(null);
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      cy.get(byTestid(collectionTestid.grid)).should("be.visible");
      cy.get(byTestid(collectionTestid.completion("Core"))).should("exist");
      cy.get(byTestid(DB_SEARCH)).should("be.visible");
      // The four tabs are on screen, each a finger's size, whatever the grid asks of the column.
      for (const tab of ["cards", "decks", "cosmetics", "progress"] as const) {
        cy.get(byTestid(collectionTestid.tab(tab))).should("be.visible").and(($tab) => {
          expect($tab[0]?.getBoundingClientRect().height, `the ${tab} tab's height`).to.be.at.least(44);
        });
      }
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);

      // The grid is virtual: a few hundred cards, a screenful of them mounted.
      cy.get(`${byTestid(collectionTestid.grid)} li[data-card]`).should(($cells) => {
        expect($cells.length, "cards mounted").to.be.greaterThan(0);
        expect($cells.length, "cards mounted, of the whole catalog").to.be.lessThan(COLLECTED_DEFS.length / 2);
      });
      cy.get(byTestid(collectionTestid.grid)).then(($grid) => {
        const scroller = $grid.parent()[0];
        expect(scroller?.scrollHeight, "the grid is taller than its scroller").to.be.greaterThan(scroller?.clientHeight ?? 0);
        expect(getComputedStyle(scroller as Element).overflowY, "the grid's scroller").to.eq("auto");
      });
      // A scroll to the end mounts the last card.
      cy.get(byTestid(collectionTestid.grid)).then(($grid) => {
        $grid.parent()[0]?.scrollTo({ top: 1_000_000 });
      });
      cy.get(byTestid(collectionTestid.grid)).find("li[data-card]").last().should("be.visible");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });

    it(`R3734000 Cards has the filters ${viewport.width > TABLET_MAX_PX ? "in a sidebar to the left of the grid" : "behind a Filters button"} at ${size}`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt("cards");
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);
      cy.get(byTestid(collectionTestid.grid)).should("be.visible");

      if (viewport.width > TABLET_MAX_PX) {
        cy.get(byTestid(collectionTestid.filters)).should("be.visible");
        cy.get(byTestid(collectionTestid.filtersOpen)).should("not.exist");
        cy.get(byTestid(collectionTestid.filters)).then(($filters) => {
          cy.get(byTestid(collectionTestid.grid)).should(($grid) => {
            const filters = $filters[0]?.getBoundingClientRect();
            const grid = $grid[0]?.getBoundingClientRect();
            expect(filters?.right, "the sidebar ends where the grid begins").to.be.at.most((grid?.left ?? 0) + 1);
            expect(filters?.top, "the sidebar begins at the grid's height or above").to.be.at.most((grid?.top ?? 0) + 1);
          });
        });
        if (viewport.width >= 1280) {
          cy.get(byTestid(collectionTestid.grid)).should(($grid) => {
            expect($grid[0]?.getBoundingClientRect().width, "the grid leaves the 960 px frame").to.be.greaterThan(OLD_FRAME_PX);
          });
        }
      } else {
        cy.get(byTestid(collectionTestid.filtersOpen)).should("be.visible");
        cy.get(byTestid(collectionTestid.filters)).should("not.exist");
        cy.get(byTestid(collectionTestid.filtersOpen)).click();
        cy.get(byTestid(collectionTestid.filtersSheet)).should("be.visible");
        cy.get(byTestid(collectionTestid.filters)).should("be.visible");
      }
    });

    it(`R3734000 Decks fits one screen at ${size}`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt("decks");
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      cy.get(byTestid(collectionTestid.deckTile("d-0"))).should("be.visible");
      cy.get(byTestid(collectionTestid.deckNew)).should("exist");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });

    it(`R1579000 R3734000 Cosmetics fits one screen at ${size}`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt("cosmetics");
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      cy.get(byTestid(collectionTestid.tiles("table"))).should("exist");
      cy.get(byTestid(collectionTestid.tiles("cardBack"))).should("exist");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });

    it(`#1307 R3734000 Progress has no horizontal scroll at ${size}, and its body scrolls inside the screen`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt("progress");
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      cy.get(byTestid(collectionTestid.track(newestShippedSet()))).should("exist");
      cy.get(byTestid(collectionTestid.body)).should("have.css", "overflow-y", "auto");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });
  }
});
