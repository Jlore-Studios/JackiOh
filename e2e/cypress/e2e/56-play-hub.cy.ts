// 56 — the Play hub (issue #3729, R3729000, R3729001): /play is four groups of cards, an Online card opens
// one panel from the hub to Find a match, and the search shows in the shell's banner on every menu screen
// until it is cancelled, from Play or from Collection. At 1440x900 and 820x1180 (a desktop and an iPad held
// upright) the spec queues All random from the card, follows the banner to Collection and cancels there,
// then queues again and cancels on Play. Every one of `MENU_SIZES` shoots the hub, and its panel open, for
// the pull request (`3729-play-<size>.png`, `3729-play-panel-<size>.png`; a whole shot wants a window larger
// than the size, `E2E_WINDOW_SIZE` in e2e/README.md). Networked: it needs `E2E=1 cargo run --release -p jackioh-server`.

import { menuShellTestid } from "../../../apps/web/src/shell/testids.ts";
import { MENU_SIZES, accounts, apiUrl, bearerHeaders, routes, timeouts } from "../../support/config.ts";
import { openQueuePanel } from "../../support/play.ts";
import { PLAY_HUB, PLAY_QUEUE, PLAY_QUEUE_PANEL, PLAY_STATUS, playGroupId, ts } from "../../support/testids.ts";

/** Seat two is free of the other lobby specs' queues; R109's request budget is per account. */
const ACCOUNT = accounts.p2;

/** The two sizes the flow runs at; every size shoots. */
const FLOW_SIZES = MENU_SIZES.filter((size) => size.label === "desktop-1440x900" || size.label === "ipad-portrait-820x1180");

/** The hub's one scroll region (play-hub.css). */
const PLAY_BODY = ".play-body";
/** A scroll height this far past the region's own still counts as fitting (a rounded height). */
const FIT_SLACK_PX = 1;

/** An item of the shell's navigation. */
function navItem(testid: string): Cypress.Chainable<JQuery> {
  return cy.get(ts(menuShellTestid.nav), { timeout: timeouts.view }).find(ts(testid), { timeout: timeouts.view });
}

/** Queues All random from its card, the panel's one primary action, and waits for the banner. */
function queueAllRandom(): void {
  openQueuePanel("random");
  cy.get(ts(PLAY_QUEUE), { timeout: timeouts.view }).should("not.be.disabled").click();
  cy.get(ts(PLAY_QUEUE_PANEL)).should("not.exist");
  cy.get(ts(menuShellTestid.queue), { timeout: timeouts.view }).should("be.visible").and("have.attr", "data-mode", "random");
  cy.get(ts(menuShellTestid.queueClock)).should("be.visible");
}

function cancelFromBanner(): void {
  cy.get(ts(menuShellTestid.queueLeave)).click();
  cy.get(ts(menuShellTestid.queue), { timeout: timeouts.view }).should("not.exist");
}

describe("56 — the Play hub (issue #3729)", () => {
  before(() => {
    cy.freeAccount(ACCOUNT());
    // A queue an earlier run left on the server would pair the first Find a match at once.
    cy.request({ method: "DELETE", url: apiUrl("/api/queue"), headers: bearerHeaders(ACCOUNT().token), failOnStatusCode: false });
  });

  for (const size of FLOW_SIZES) {
    describe(size.label, () => {
      beforeEach(() => {
        cy.viewport(size.width, size.height);
      });

      it("R3729000 one panel from the hub to Find a match, and R3729001 the banner follows the player to Collection", () => {
        cy.visitAs(ACCOUNT(), routes.play());
        cy.get(ts(PLAY_HUB), { timeout: timeouts.view }).should("be.visible");
        for (const group of ["online", "daily", "friends", "solo"] as const) {
          cy.get(ts(playGroupId(group))).should("exist");
        }

        // R3729000: an Online card opens its panel, where Find a match is one press.
        queueAllRandom();
        cy.get(ts(PLAY_STATUS)).should("be.visible");
        cy.location("pathname").should("eq", routes.play());

        // R3729001: the banner stays on every menu screen, and Cancel there ends the search.
        navItem(menuShellTestid.collection).click();
        cy.location("pathname", { timeout: timeouts.view }).should("eq", routes.collection());
        cy.get(ts(menuShellTestid.queue), { timeout: timeouts.view }).should("be.visible");
        cy.get(ts(menuShellTestid.queueWait)).should("have.length.lessThan", 2);
        cancelFromBanner();

        // Back on Play the cards are open to a new search, which Cancel on Play ends as well.
        navItem(menuShellTestid.play).click();
        cy.location("pathname", { timeout: timeouts.view }).should("eq", routes.play());
        cy.get(ts(PLAY_STATUS)).should("not.exist");
        queueAllRandom();
        cancelFromBanner();
        cy.get(ts(PLAY_STATUS), { timeout: timeouts.view }).should("contain.text", "Left the queue");
      });
    });
  }

  for (const size of MENU_SIZES) {
    describe(`screenshots at ${size.label}`, () => {
      beforeEach(() => {
        cy.viewport(size.width, size.height);
      });

      it("R3729000 the hub and its panel", () => {
        cy.visitAs(ACCOUNT(), routes.play());
        cy.get(ts(PLAY_HUB), { timeout: timeouts.view }).should("be.visible");
        cy.screenshot(`3729-play-${size.label}`, { capture: "viewport", overwrite: true });
        openQueuePanel("bo1");
        cy.screenshot(`3729-play-panel-${size.label}`, { capture: "viewport", overwrite: true });
      });

      // #3958: a hub whose last cards are cut at the window's foot says so; one that fits needs no cue.
      it("R3729000 the hub shows its Solo cards, or a cue that more is below", () => {
        cy.visitAs(ACCOUNT(), routes.play());
        cy.get(ts(PLAY_HUB), { timeout: timeouts.view }).should("be.visible");
        cy.get(PLAY_BODY).should(($body) => {
          const body = $body[0];
          if (body === undefined) throw new Error("the hub's scroll region is not drawn");
          const win = body.ownerDocument.defaultView;
          if (win === null) throw new Error("the page has no window");
          expect(win.getComputedStyle(body).overflowY, "the one region that scrolls").to.eq("auto");
          const fits = body.scrollHeight <= body.clientHeight + FIT_SLACK_PX;
          const cue = win.getComputedStyle(body, "::after");
          const cued = cue.content !== "none" && parseFloat(cue.height) > 0 && parseFloat(cue.opacity) > 0 && cue.pointerEvents === "none";
          expect(fits || cued, `the region (${String(body.scrollHeight)} px of content in ${String(body.clientHeight)}) fits or shows its cue`).to.eq(true);
        });
      });
    });
  }
});
