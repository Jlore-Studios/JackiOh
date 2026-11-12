// 51 — the menus' shell (issue #3728, R3728000, R3728001): at each of issue #3728's sizes (`MENU_SIZES`,
// support/config.ts) Home, Play, Collection and Shop are one press away in the shell's navigation (a
// labelled rail over 1024 px, an icon rail down to 601 px, a tab bar at 600 px and under), each landing on
// its own path, and Friends opens the friends drawer over the screen it was pressed on, the path unchanged.
// The drawer closes on Escape, on a press outside it (its ✕ at 600 px and under, where it takes the whole
// width) and on Back. Chrome has `CloseWatcher`, and on a computer Back is no close request there (Escape
// is, and Android's back; R2127000), so Back is pressed on a page loaded without it, as a browser without
// one has it. Each size also shoots Home, Play and Collection for the pull request
// (`3728-<screen>-<size>.png`; a whole shot wants a window larger than the size, `E2E_WINDOW_SIZE` in
// e2e/README.md). Per pull request: SMOKE in scripts/shard-specs.mjs holds 51. Networked: it needs
// `E2E=1 cargo run --release -p jackioh-server`.

import { landingTestid } from "../../../apps/web/src/auth/testids.ts";
import { BREAKPOINT_PX } from "../../../apps/web/src/breakpoints.ts";
import { menuShellTestid } from "../../../apps/web/src/shell/testids.ts";
import { MENU_SIZES, accounts, routes, timeouts } from "../../support/config.ts";
import { expectShellNav } from "../../support/shell.ts";
import { COLLECTION_SCREEN, FRIENDS_BUTTON, FRIENDS_CLOSE, FRIENDS_PANEL, PLAY_QUEUE, SHOP_PANEL, ts } from "../../support/testids.ts";

/**
 * R109 refuses an account past 300 requests a minute, and each size here opens about ten screens. 46 signs
 * in as p1 too: smoke's split (WEIGHTS) runs it on the other shard's server, and 47, which runs just before
 * 51, signs in as p2. A 429 here means 51 shared a minute with another p1 spec: give it p2.
 */
const SIGNED_IN = accounts.p1;

/** The drawer's scrim (game/Dialog.tsx's `drawer`), which covers the window. */
const DRAWER_SCRIM = ".dialog-scrim[data-drawer]";

/** A press outside the drawer: this far in from the window's left edge, on the scrim over the rail. */
const OUTSIDE_X_PX = 8;

/** Marks the page Back is pressed on, so a Back that left it and loaded another is caught. */
const SAME_PAGE = "__jackioh51SamePage";

/** Each of the shell's links, from the landing in this order: where it goes and what that screen draws. */
const DESTINATIONS = [
  { item: menuShellTestid.play, path: routes.play, root: PLAY_QUEUE },
  { item: menuShellTestid.collection, path: routes.collection, root: COLLECTION_SCREEN },
  { item: menuShellTestid.shop, path: routes.shop, root: SHOP_PANEL },
  { item: menuShellTestid.home, path: routes.landing, root: landingTestid.root },
] as const;

/** The screens shot at every size. */
const SHOTS = [
  { screen: "home", path: routes.landing, root: landingTestid.root },
  { screen: "play", path: routes.play, root: PLAY_QUEUE },
  { screen: "collection", path: routes.collection, root: COLLECTION_SCREEN },
] as const;

/** An item of the shell's navigation, never a control of the same testid elsewhere on the screen. */
function navItem(testid: string): Cypress.Chainable<JQuery> {
  return cy.get(ts(menuShellTestid.nav), { timeout: timeouts.view }).find(ts(testid), { timeout: timeouts.view });
}

/** The landing's fan is dealt in (R704): Home is shot once every card is drawn. */
function fanDealt(): void {
  cy.get(`${ts(landingTestid.fan)} .landing-fan-card--face`, { timeout: timeouts.view }).should(($cards) => {
    expect($cards.length, "the fan holds its cards").to.be.greaterThan(0);
    for (const card of $cards.toArray()) expect(getComputedStyle(card).opacity, "a dealt card").to.eq("1");
  });
}

function openDrawer(): void {
  navItem(FRIENDS_BUTTON).click();
  cy.get(ts(FRIENDS_PANEL), { timeout: timeouts.view }).should("be.visible");
}

function expectDrawerClosed(path: string): void {
  cy.get(ts(FRIENDS_PANEL)).should("not.exist");
  cy.location("pathname").should("eq", path);
}

/** R2127000: the page as a browser without `CloseWatcher` has it, where an open dialog's history entry takes Back. */
function withoutCloseWatcher(win: Cypress.AUTWindow): void {
  Reflect.deleteProperty(win, "CloseWatcher");
}

describe("51 — the menus' shell (issue #3728)", () => {
  before(() => {
    cy.freeAccount(SIGNED_IN());
  });

  for (const size of MENU_SIZES) {
    describe(size.label, () => {
      beforeEach(() => {
        cy.viewport(size.width, size.height);
      });

      it("R3728000 Home, Play, Collection, Shop are one tap away and Friends opens in place", () => {
        cy.visitAs(SIGNED_IN(), routes.landing());
        cy.get(ts(landingTestid.root), { timeout: timeouts.view }).should("be.visible");
        expectShellNav(true);
        for (const { item, path, root } of DESTINATIONS) {
          navItem(item).click();
          cy.location("pathname").should("eq", path());
          cy.get(ts(root), { timeout: timeouts.view }).should("be.visible");
          navItem(item).should("have.attr", "aria-current", "page");
        }
        openDrawer();
        cy.location("pathname").should("eq", routes.landing());
      });

      it("R3728001 the drawer closes on Escape, Back and a tap outside", () => {
        cy.visitAs(SIGNED_IN(), routes.collection());
        cy.get(ts(COLLECTION_SCREEN), { timeout: timeouts.view }).should("be.visible");

        openDrawer();
        cy.get("body").type("{esc}");
        expectDrawerClosed(routes.collection());

        openDrawer();
        if (size.width > BREAKPOINT_PX.phone) {
          cy.get(DRAWER_SCRIM).click(OUTSIDE_X_PX, Math.round(size.height / 2));
        } else {
          // At 600 px and under the drawer takes the whole width: its ✕ is the press that closes it.
          cy.get(ts(FRIENDS_CLOSE)).click();
        }
        expectDrawerClosed(routes.collection());

        cy.visitAs(SIGNED_IN(), routes.collection(), { onBeforeLoad: withoutCloseWatcher });
        cy.get(ts(COLLECTION_SCREEN), { timeout: timeouts.view }).should("be.visible");
        openDrawer();
        cy.window().then((win) => {
          Reflect.set(win, SAME_PAGE, true);
        });
        cy.go("back");
        expectDrawerClosed(routes.collection());
        cy.window().should((win) => {
          expect(Reflect.get(win, SAME_PAGE), "Back closed the drawer and stayed on the page").to.eq(true);
        });
      });

      it("R3728000 screenshots", () => {
        for (const { screen, path, root } of SHOTS) {
          cy.visitAs(SIGNED_IN(), path());
          cy.get(ts(root), { timeout: timeouts.view }).should("be.visible");
          if (screen === "home") fanDealt();
          expectShellNav(true);
          cy.screenshot(`3728-${screen}-${size.label}`, { capture: "viewport", overwrite: true });
        }
      });
    });
  }
});
