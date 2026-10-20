// #1307: the collection screen fits a phone. jsdom cannot measure layout, so this mounts the route
// in a browser, serves the six reads it makes from the fixtures, and checks that nothing on the
// page is wider than the viewport at 360 px and 390 px, with every panel shown and then with a
// set's card grid open.

import { newestShippedSet } from "@jackioh/shared";

import { collectionTestid } from "../../../apps/web/src/collection/collection.ts";
import {
  fullCollection,
  questsHeld,
  tracksOf,
} from "../../../apps/web/src/collection/fixtures.ts";
import { packsInfo } from "../../../apps/web/src/packs/fixtures.ts";
import CollectionRoute from "../../../apps/web/src/routes/collection.tsx";
import { cosmeticPacksInfo, walletInfo } from "../../../apps/web/src/shop/fixtures.ts";

const VIEWPORTS = [
  { width: 360, height: 740 },
  { width: 390, height: 844 },
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

describe("#1307 the collection screen fits a phone", () => {
  for (const viewport of VIEWPORTS) {
    const size = `${String(viewport.width)}x${String(viewport.height)}`;
    it(`#1307 has no horizontal scroll at ${size}, a set opened`, () => {
      cy.viewport(viewport.width, viewport.height);
      serveFixtures();
      cy.mount(<CollectionRoute token="token-1" />);

      cy.get(byTestid(collectionTestid.completion("Core"))).should("exist");
      cy.get(byTestid(collectionTestid.track(newestShippedSet()))).should("exist");
      expectNoHorizontalOverflow(viewport);

      cy.get(byTestid(collectionTestid.setToggle("Core"))).click();
      cy.get(byTestid(collectionTestid.card("core-001"))).should("exist");
      expectNoHorizontalOverflow(viewport);
    });
  }
});
