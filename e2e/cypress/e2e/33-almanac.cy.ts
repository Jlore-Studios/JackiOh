// Spec 33 — the Card Almanac (issue #54; SPEC §10.10, R630), on the landing page and `/almanac`,
// against `build:e2e` with no server.
//
// BUILD M8's key assertions for this row: "the footer's "Card almanac" link opens `/almanac`;
// filtering by a cost keeps only cards of that cost; a card's detail view opens with no add action
// and closes; no API call is made on the page; Back returns to the landing page". Issue #54 asks
// for the same walk: signed out, the landing page, the footer's link, a cost, a card's detail
// closed again, and Back. Also asserted, off the DOM: the link sits right after Patch notes, and
// the almanac is the deck builder's browse pane, read-only (no "Owned only", no "+", no draggable
// card).
//
// Run it:
//   pnpm build:e2e
//   pnpm --dir apps/web exec vite preview --port 5173 --strictPort
//   pnpm --dir e2e exec cypress run --spec cypress/e2e/33-almanac.cy.ts

import { landingTestid } from "../../../apps/web/src/auth/testids.ts";
import { seedFor } from "../../support/config.ts";
import {
  ALMANAC,
  CARD_POOL,
  DB_DETAIL_ADD,
  DB_FILTER_OWNED,
  DB_RESULT_COUNT,
  INSPECT_CLOSE,
  INSPECT_DETAIL,
  NAV_BACK,
  SITE_FOOTER,
  SITE_FOOTER_ALMANAC,
  SITE_FOOTER_PATCH_NOTES,
  filterCostId,
  ts,
} from "../../support/testids.ts";

/** The cost chip the spec filters by, and the words every card it keeps carries in its name (R432). */
const COST = "1";
const COST_WORDS = `(${COST}) Cost`;

/** Every `/api/…` path the page asked for, in order. */
const apiCalls: string[] = [];

describe("Spec 33 — the Card Almanac (R630)", () => {
  // BUILD M8: every spec sets a seed. No game is started here, so the seed pins the scenario's
  // identity and lets CI re-run the file with `--expose seed=`.
  const seed = seedFor("33-almanac");

  beforeEach(() => {
    apiCalls.length = 0;
    // No server: the landing page's account check gets a plain "signed out", and every call is
    // recorded so the almanac can be shown to make none.
    cy.intercept({ url: /\/api\// }, (request) => {
      apiCalls.push(new URL(request.url).pathname);
      request.reply({ statusCode: 401, body: { code: "unauthenticated", message: "signed out" } });
    });
  });

  it("R630 signed out: the footer's Card almanac link, a cost filter, a card's detail, and Back to the landing page", () => {
    expect(seed, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");

    cy.visit("/");
    cy.get(ts(landingTestid.root)).should("be.visible");

    // The link sits right after Patch notes in the footer.
    cy.get(ts(SITE_FOOTER))
      .find(ts(SITE_FOOTER_ALMANAC))
      .should("have.attr", "href", "/almanac")
      .and("contain.text", "Card almanac")
      .prev()
      .should("have.attr", "data-testid", SITE_FOOTER_PATCH_NOTES);

    let callsBefore = 0;
    cy.then(() => {
      callsBefore = apiCalls.length;
    });
    cy.get(ts(SITE_FOOTER_ALMANAC)).click();
    cy.location("pathname").should("eq", "/almanac");
    cy.title().should("eq", "Almanac · JackiOh");
    cy.get(ts(ALMANAC)).should("be.visible");

    // The deck builder's browse pane, read-only.
    cy.get(ts(CARD_POOL)).find(".db-card").should("have.length.greaterThan", 0);
    cy.get(ts(DB_FILTER_OWNED)).should("not.exist");
    cy.get(ts(CARD_POOL)).find(".db-add").should("not.exist");
    cy.get(ts(CARD_POOL)).find('.db-card[draggable="true"]').should("not.exist");

    // Filter by a cost: fewer cards, each of that cost.
    cy.get(ts(DB_RESULT_COUNT))
      .invoke("attr", "data-count")
      .then((all) => {
        cy.get(ts(filterCostId(COST))).click().should("have.attr", "aria-pressed", "true");
        cy.get(ts(DB_RESULT_COUNT)).invoke("attr", "data-count").then(Number).should("be.lessThan", Number(all)).and("be.greaterThan", 0);
      });
    cy.get(ts(CARD_POOL))
      .find(".db-card")
      .each(($card) => {
        expect($card.attr("aria-label"), $card.attr("data-card")).to.contain(COST_WORDS);
      });

    // A card's detail view: both faces, Close, and no way to add the card anywhere.
    cy.get(ts(CARD_POOL))
      .find(".db-card")
      .first()
      .then(($card) => {
        const id = $card.attr("data-card") ?? "";
        cy.wrap($card).click();
        cy.get(ts(INSPECT_DETAIL)).should("be.visible").and("have.attr", "data-card", id);
      });
    cy.get(ts(DB_DETAIL_ADD)).should("not.exist");
    cy.get(ts(INSPECT_CLOSE)).click();
    cy.get(ts(INSPECT_DETAIL)).should("not.exist");

    // The almanac asked the server nothing.
    cy.then(() => {
      expect(apiCalls.slice(callsBefore), "API calls made on /almanac").to.deep.eq([]);
      for (const path of apiCalls) expect(path).to.not.match(/\/api\/(catalog|collection|decks)/u);
    });

    // Back returns to the landing page.
    cy.get(ts(NAV_BACK)).click();
    cy.location("pathname").should("eq", "/");
    cy.get(ts(landingTestid.root)).should("be.visible");
  });
});
