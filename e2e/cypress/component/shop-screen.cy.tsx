// R1580000: the Shop screen fits one screen. jsdom cannot measure layout, so this mounts the route in
// a browser, serves the six reads its four panels make from the fixtures, and checks that at 360 px,
// 390 px and a desktop's 1280 px nothing is wider than the viewport and the page never scrolls: the
// body is what scrolls. The pack opening still plays full screen over it: its scene, drawn in a
// portal on the body, covers the window.

import { collectionOf, cosmeticPacksInfo, ownedCosmeticsOf, tokenBars, walletInfo } from "../../../apps/web/src/shop/fixtures.ts";
import { packsInfo } from "../../../apps/web/src/packs/fixtures.ts";
import { packTestid } from "../../../apps/web/src/packs/packs.ts";
import ShopRoute from "../../../apps/web/src/routes/shop.tsx";
import { shopTestid } from "../../../apps/web/src/shop/shop.ts";

const VIEWPORTS = [
  { width: 360, height: 740 },
  { width: 390, height: 844 },
  { width: 1280, height: 800 },
] as const;

type Viewport = (typeof VIEWPORTS)[number];

/** A browser rounds a fixed box's edges to a device pixel. */
const EDGE_TOLERANCE_PX = 1;

function byTestid(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** What each read the screen makes answers; any other request goes on to the network. */
function serveFixtures(): void {
  const bodies: Record<string, unknown> = {
    "/api/packs": packsInfo(),
    "/api/wallet": walletInfo(),
    "/api/tokens": tokenBars(),
    "/api/collection": collectionOf(),
    "/api/cosmetics": ownedCosmeticsOf(),
    "/api/cosmetic-packs": cosmeticPacksInfo(),
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

/** R1580000: the page never scrolls: the document is no taller than the window. */
function expectNoPageScroll(viewport: Viewport): void {
  cy.window({ log: false }).should((win) => {
    const where = `${String(viewport.width)}x${String(viewport.height)}`;
    expect(win.document.documentElement.scrollHeight, `the page does not scroll at ${where}`).to.be.at.most(win.innerHeight);
  });
}

/** Every panel has read and drawn what the fixtures answer. */
function waitForPanels(): void {
  cy.get(byTestid(shopTestid.wallet)).should("contain.text", "Jlore");
  cy.get(byTestid(packTestid.pity("epic"))).should("exist");
  cy.get(byTestid(packTestid.open("Core"))).should("exist");
  cy.get(byTestid(shopTestid.packsPanel)).should("exist");
  cy.get(byTestid(shopTestid.cardBar)).should("exist");
}

describe("R1580000 the Shop fits one screen", () => {
  for (const viewport of VIEWPORTS) {
    const size = `${String(viewport.width)}x${String(viewport.height)}`;

    it(`R1580000 the Shop fits one screen at ${size}`, () => {
      cy.viewport(viewport.width, viewport.height);
      serveFixtures();
      cy.mount(<ShopRoute token="token-1" />);

      waitForPanels();
      cy.get(byTestid(shopTestid.body)).should("have.css", "overflow-y", "auto");
      expectNoHorizontalOverflow(viewport);
      expectNoPageScroll(viewport);
    });

    it(`R1580000 the pack opening covers the screen at ${size}`, () => {
      cy.viewport(viewport.width, viewport.height);
      serveFixtures();
      cy.mount(<ShopRoute token="token-1" />);

      cy.get(byTestid(packTestid.open("Core"))).click();
      cy.window({ log: false }).then((win) => {
        cy.get(byTestid(packTestid.scene)).should(($scene) => {
          const rect = $scene[0]?.getBoundingClientRect();
          expect(rect, "the scene is drawn").to.not.eq(undefined);
          if (rect === undefined) return;
          expect(rect.left, "its left edge").to.be.closeTo(0, EDGE_TOLERANCE_PX);
          expect(rect.top, "its top edge").to.be.closeTo(0, EDGE_TOLERANCE_PX);
          expect(rect.width, `its width over ${size}`).to.be.closeTo(win.innerWidth, EDGE_TOLERANCE_PX);
          expect(rect.height, `its height over ${size}`).to.be.closeTo(win.innerHeight, EDGE_TOLERANCE_PX);
        });
      });
    });
  }
});
