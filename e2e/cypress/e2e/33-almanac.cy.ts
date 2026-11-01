// Spec 33 (SPEC §10.10, R630): signed-out landing-page access to read-only `/almanac`.

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
  poolCardId,
  ts,
} from "../../support/testids.ts";

/** The cost chip the spec filters by, and the words every card it keeps carries in its name (R432). */
const COST = "1";
const COST_WORDS = `(${COST}) Cost`;

/** How many cards Tab walks in a row, past the rows the grid mounts at once. */
const POOL_WALK = 120;

const apiCalls: string[] = [];

/**
 * #2334: the grid mounts only the rows near the view (PoolGrid's `virtual`), so a check of every card
 * scrolls the pool from top to foot a screen at a time. `check` sees each card once; the ids come
 * back in the grid's order, as many as the result count says.
 */
function eachPoolCard(check: ($card: JQuery<HTMLElement>) => void): Cypress.Chainable<string[]> {
  const ids: string[] = [];
  /** The rows in view are mounted: the first card starts, and the last ends, at the view's edges, less the grid's padding. */
  const coversView = (pool: HTMLElement): boolean => {
    const view = pool.getBoundingClientRect();
    const padding = Number.parseFloat(getComputedStyle(pool).paddingTop) + pool.clientTop;
    const cards = [...pool.querySelectorAll<HTMLElement>(".db-card")].map((card) => card.getBoundingClientRect());
    return (
      cards.length > 0 &&
      Math.min(...cards.map((card) => card.top)) <= view.top + padding + 1 &&
      Math.max(...cards.map((card) => card.bottom)) >= view.bottom - padding - 1
    );
  };
  const visit = (top: number): void => {
    cy.get(ts(CARD_POOL)).then(($pool) => {
      $pool.scrollTop(top);
    });
    cy.get(ts(CARD_POOL)).should(($pool) => {
      expect(coversView($pool[0] as HTMLElement), `the rows in view at ${String(top)} px are mounted`).to.eq(true);
    });
    cy.get(ts(CARD_POOL)).then(($pool) => {
      $pool.find(".db-card").each((_index, card) => {
        const id = card.dataset.card ?? "";
        if (ids.includes(id)) return;
        ids.push(id);
        check(Cypress.$(card));
      });
      const height = $pool.innerHeight() ?? 0;
      if (top + height < ($pool[0] as HTMLElement).scrollHeight) visit(top + height);
    });
  };
  visit(0);
  cy.get(ts(DB_RESULT_COUNT))
    .invoke("attr", "data-count")
    .then((count) => {
      expect(ids.length, "cards met while scrolling the pool through").to.eq(Number(count));
    });
  return cy.then(() => ids);
}

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
    eachPoolCard(($card) => {
      expect($card.attr("aria-label"), $card.attr("data-card")).to.contain(COST_WORDS);
    }).then((ids) => {
      const id = ids[0] ?? "";
      cy.get(ts(CARD_POOL)).then(($pool) => {
        $pool.scrollTop(0);
      });
      cy.get(ts(poolCardId(id))).click();
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

  it("#2334 mounts only the rows near the view, names every card, and Tab walks the cards in order past what was mounted", () => {
    cy.visit("/almanac");
    cy.get(ts(ALMANAC)).should("be.visible");
    cy.get(ts(CARD_POOL)).find(".db-card").should("have.length.greaterThan", 0);

    cy.get(ts(DB_RESULT_COUNT))
      .invoke("attr", "data-count")
      .then(Number)
      .then((count) => {
        cy.get(ts(CARD_POOL)).find(".db-card").its("length").should("be.lessThan", count / 2);
      });

    eachPoolCard(($card) => {
      expect($card.prop("tagName"), $card.attr("data-card")).to.eq("BUTTON");
      expect($card.attr("aria-label"), $card.attr("data-card")).to.match(/\. Show details$/u);
    }).then((ids) => {
      expect(ids.length, "cards in the pool").to.be.greaterThan(POOL_WALK);
      cy.get(ts(CARD_POOL)).then(($pool) => {
        $pool.scrollTop(0);
      });
      cy.get(ts(poolCardId(ids[0] ?? ""))).focus();
      cy.focused().should("have.attr", "data-card", ids[0]);
      for (let at = 1; at < POOL_WALK; at += 1) {
        cy.press(Cypress.Keyboard.Keys.TAB);
        cy.focused().should("have.attr", "data-card", ids[at]);
      }
    });
  });
});
