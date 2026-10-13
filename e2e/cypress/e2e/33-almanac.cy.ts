// Spec 33 (SPEC §10.10, R630): signed-out landing-page access to read-only `/almanac`.
// BUILD M8 permits only its public statistics request (R654) in this serverless scenario.

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

const apiCalls: string[] = [];

describe("Spec 33 — the Card Almanac (R630)", () => {
  // BUILD M8: the seed identifies this serverless scenario for CI re-runs.
  const seed = seedFor("33-almanac");

  beforeEach(() => {
    apiCalls.length = 0;
    // No server: record the signed-out landing page's calls.
    cy.intercept({ url: /\/api\// }, (request) => {
      apiCalls.push(new URL(request.url).pathname);
      request.reply({ statusCode: 401, body: { code: "unauthenticated", message: "signed out" } });
    });
  });

  it("R630 signed out: the footer's Card almanac link, a cost filter, a card's detail, and Back to the landing page", () => {
    expect(seed, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");

    cy.visit("/");
    cy.get(ts(landingTestid.root)).should("be.visible");

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

    cy.get(ts(CARD_POOL)).find(".db-card").should("have.length.greaterThan", 0);
    cy.get(ts(DB_FILTER_OWNED)).should("not.exist");
    cy.get(ts(CARD_POOL)).find(".db-add").should("not.exist");
    cy.get(ts(CARD_POOL)).find('.db-card[draggable="true"]').should("not.exist");

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

    // R654: `/api/stats/*` is public and cacheable; all other almanac calls are forbidden.
    cy.then(() => {
      const calls = apiCalls.slice(callsBefore);
      const nonStats = calls.filter((path) => !path.startsWith("/api/stats/"));
      expect(nonStats, "non-stats API calls made on /almanac").to.deep.eq([]);
      for (const path of apiCalls) expect(path).to.not.match(/\/api\/(catalog|collection|decks)/u);
    });

    cy.get(ts(NAV_BACK)).click();
    cy.location("pathname").should("eq", "/");
    cy.get(ts(landingTestid.root)).should("be.visible");
  });
});
