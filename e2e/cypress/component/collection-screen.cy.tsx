// #1307: the collection screen fits a phone. jsdom cannot measure layout, so this mounts the route
// in a browser, serves the reads it makes from the fixtures, and checks that nothing on the
// page is wider than the viewport at 360 px and 390 px, with every panel shown and then with a
// set's card grid open.
//
// R1579000: the hub and each section fit one screen at those two sizes and on a desktop's: the
// page never scrolls, either way, and the card list's body is what scrolls. A door is never
// clicked here: `navigate` would move the iframe's own path, so each test sets `?section=` itself.

import { newestShippedSet } from "@jackioh/shared";

import { collectionTestid, type CollectionSection } from "../../../apps/web/src/collection/collection.ts";
import {
  fullCollection,
  questsHeld,
  tracksOf,
} from "../../../apps/web/src/collection/fixtures.ts";
import { packsInfo } from "../../../apps/web/src/packs/fixtures.ts";
import CollectionRoute from "../../../apps/web/src/routes/collection.tsx";
import { cosmeticPacksInfo, ownedCosmeticsOf, walletInfo } from "../../../apps/web/src/shop/fixtures.ts";

const VIEWPORTS = [
  { width: 360, height: 740 },
  { width: 390, height: 844 },
  { width: 1280, height: 800 },
] as const;

type Viewport = (typeof VIEWPORTS)[number];

function byTestid(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** What each read the screen makes answers; any other request goes on to the network. */
function serveFixtures(): void {
  const bodies: Record<string, unknown> = {
    "/api/collection": fullCollection(),
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

/** Nothing on the page is wider than the viewport. */
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

/** R1579000: the page never scrolls: the document is no taller than the window. */
function expectNoPageScroll(viewport: Viewport): void {
  cy.window({ log: false }).should((win) => {
    const where = `${String(viewport.width)}x${String(viewport.height)}`;
    expect(win.document.documentElement.scrollHeight, `the page does not scroll at ${where}`).to.be.at.most(win.innerHeight);
  });
}

describe("#1307 the collection screen fits a phone", () => {
  afterEach(() => {
    openAt(null);
  });

  for (const viewport of VIEWPORTS) {
    const size = `${String(viewport.width)}x${String(viewport.height)}`;
    it(`#1307 has no horizontal scroll at ${size}, a set opened`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt("cards");
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      cy.get(byTestid(collectionTestid.completion("Core"))).should("exist");
      cy.get(byTestid(collectionTestid.track(newestShippedSet()))).should("exist");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);

      cy.get(byTestid(collectionTestid.setToggle("Core"))).click();
      cy.get(byTestid(collectionTestid.card("core-001"))).should("exist");
      cy.get(byTestid(collectionTestid.body)).should("have.css", "overflow-y", "auto");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });

    it(`R1579000 the hub fits one screen at ${size}`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt(null);
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      for (const door of ["decks", "cards", "cosmetics", "builder"] as const) {
        cy.get(byTestid(collectionTestid.door(door))).should("be.visible");
      }
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });

    it(`R1579000 Cosmetics fits one screen at ${size}`, () => {
      cy.viewport(viewport.width, viewport.height);
      openAt("cosmetics");
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      cy.get(byTestid(collectionTestid.equip("foil"))).should("exist");
      cy.get(byTestid(collectionTestid.ownedCosmetic("full-art-gary-the-gambler", "core-004"))).should("exist");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });
  }
});
