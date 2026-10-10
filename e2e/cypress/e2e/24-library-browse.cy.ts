// Library browsing and tutorial coverage (SPEC §10.8, §10.10, §9.10; R310, R313, R314, R373).
// BUILD M8: seeds come from `seedFor`; waits are retried rather than fixed.

import { CARD_NAMES } from "../../support/cards.ts";
import { seedFor } from "../../support/config.ts";
import {
  BOARD,
  BROWSABLE,
  COACH,
  INSPECT_CLOSE,
  INSPECT_LIST_CARD,
  INSPECT_LIST_COUNT,
  INSPECT_LIST_HOVER,
  INSPECT_LIST_SHEET,
  TUTORIAL_EXIT,
  TUTORIAL_HUD,
  libraryCountId,
  libraryId,
  ts,
} from "../../support/testids.ts";
import {
  TUTORIAL_BOOT_TIMEOUT,
  currentView,
  lessonUrl,
  takeMoment,
  visitTutorial,
  waitForMoment,
  type Moment,
} from "../../support/tutorial.ts";
import type { PlayerId } from "../../support/types.ts";

const SEED = seedFor("24-library");
const DECK_A = "01-aggro-a";
const DECK_B = "01-aggro-b";

const LESSON = "basics";
const MOMENT_BUDGET = 12;

const ORDER_HIDDEN = /order hidden/i;

type Seat = PlayerId;
type LibraryCard = { defId: string };

/** Catalog ids map to SPEC §8 names. */
function nameOf(defId: string): string {
  const match = /^core-(\d{3})$/.exec(defId);
  const name = match === null ? undefined : CARD_NAMES[Number(match[1])];
  expect(name, `SPEC §8 names ${defId}`).to.not.eq(undefined);
  return name ?? defId;
}

function tally(names: readonly string[]): string[] {
  const counts = new Map<string, number>();
  for (const name of names) counts.set(name, (counts.get(name) ?? 0) + 1);
  return [...counts.entries()].map(([name, count]) => `${name} × ${String(count)}`).sort();
}

/** The page must never read the hotseat handle. */
function trueLibrary(seat: Seat): Cypress.Chainable<string[]> {
  return cy.gameState().then((state) => {
    const side = state.players[seat] as { library?: LibraryCard[] };
    expect(side.library, `${seat}'s library in the dev handle`).to.be.an("array");
    return (side.library ?? []).map((card) => nameOf(card.defId));
  });
}

function listedNames(): Cypress.Chainable<string[]> {
  return cy.get(`${ts(INSPECT_LIST_SHEET)} ${ts(INSPECT_LIST_CARD)}`).then(($tiles) =>
    $tiles.toArray().flatMap((tile) => {
      const count = Number(tile.getAttribute("data-count") ?? "1");
      expect(tile.getAttribute("data-unknown"), "every card of a deck its owner built is known (R311)").to.not.eq("true");
      return Array.from({ length: count }, () => tile.getAttribute("data-def-name") ?? "");
    }),
  );
}

const YOUR_SEAT = '[data-side="you"][data-player]';

function shownSeat(): Cypress.Chainable<Seat> {
  return cy
    .get(`${ts(BOARD)} ${YOUR_SEAT}`)
    .invoke("attr", "data-player")
    .then((seat) => {
      expect(seat, "the board names the seat it shows").to.match(/^p[12]$/);
      return seat as Seat;
    });
}

function expectOwnLibraryBrowsable(shot?: string): void {
  shownSeat().then((seat) => {
    cy.get(ts(libraryCountId("you")))
      .invoke("text")
      .then((text) => {
        const count = Number(text);
        expect(count, "a library with cards in it").to.be.greaterThan(0);
        const pile = ts(libraryId("you"));

        cy.get(`${pile}${BROWSABLE}`).should("have.attr", "role", "button");
        cy.get(pile).invoke("attr", "aria-label").should("match", ORDER_HIDDEN);
        cy.get(pile).trigger("pointerover", { pointerType: "mouse" });
        cy.get(ts(INSPECT_LIST_HOVER)).should("be.visible").and("have.css", "pointer-events", "none");
        cy.get(ts(INSPECT_LIST_HOVER)).should("contain.text", "Your deck").invoke("text").should("match", ORDER_HIDDEN);
        cy.get(`${ts(INSPECT_LIST_HOVER)} ${ts(INSPECT_LIST_COUNT)}`).should("have.attr", "data-count", String(count));
        if (shot !== undefined) cy.screenshot(`${shot}-preview`, { capture: "viewport" });
        cy.get(pile).trigger("pointerout", { pointerType: "mouse" });
        cy.get(ts(INSPECT_LIST_HOVER)).should("not.exist");

        cy.get(pile).click();
        cy.get(ts(INSPECT_LIST_SHEET)).should("be.visible").and("have.attr", "role", "dialog");
        cy.get(ts(INSPECT_LIST_SHEET)).invoke("text").should("match", ORDER_HIDDEN);
        cy.get(`${ts(INSPECT_LIST_SHEET)} ${ts(INSPECT_LIST_COUNT)}`).should("have.attr", "data-count", String(count));
        listedNames().then((listed) => {
          expect(listed, "the list has one name per card").to.have.length(count);
          trueLibrary(seat).then((truth) => {
            expect(tally(listed), `${seat}'s library, card for card`).to.deep.eq(tally(truth));
          });
        });
        if (shot !== undefined) cy.screenshot(`${shot}-dialog`, { capture: "viewport" });
        cy.get(`${ts(INSPECT_LIST_SHEET)} ${ts(INSPECT_LIST_CARD)}`).each(($tile) => {
          const tileCount = Number($tile.attr("data-count") ?? "1");
          if (tileCount > 1) expect($tile.text(), "the count on the face").to.contain(`×${String(tileCount)}`);
        });
        cy.get("body").type("{esc}");
        cy.get(ts(INSPECT_LIST_SHEET)).should("not.exist");
        cy.get(pile).should("have.focus");

        cy.get(pile).trigger("keydown", { key: "Enter" });
        cy.get(ts(INSPECT_LIST_SHEET)).should("be.visible");
        cy.get(ts(INSPECT_CLOSE)).click();
        cy.get(ts(INSPECT_LIST_SHEET)).should("not.exist");
      });
  });
}

function expectOpponentLibraryClosed(): void {
  const pile = ts(libraryId("opponent"));
  // One attribute per `should`: a negated `have.attr` leaves no subject for a second one.
  cy.get(pile).should("not.have.attr", "data-browsable");
  cy.get(pile).should("not.have.attr", "role");
  cy.get(ts(libraryCountId("opponent"))).invoke("text").then((text) => expect(Number(text)).to.be.greaterThan(0));
  cy.get(pile).trigger("pointerover", { pointerType: "mouse" });
  cy.get(pile).click();
  cy.get(ts(INSPECT_LIST_HOVER)).should("not.exist");
  cy.get(ts(INSPECT_LIST_SHEET)).should("not.exist");
  cy.get(pile).trigger("pointerout", { pointerType: "mouse" });
}

/** R314: no Skip controls in the HUD or coach bubble. */
function expectNoSkip(doc: Document): void {
  for (const region of [TUTORIAL_HUD, COACH]) {
    const root = doc.querySelector(ts(region));
    const skips = Array.from(root?.querySelectorAll("button, [data-testid], [aria-label]") ?? []).filter((element) =>
      /skip/i.test(
        [element.textContent ?? "", element.getAttribute("aria-label") ?? "", element.getAttribute("data-testid") ?? ""].join(" "),
      ),
    );
    expect(
      skips.map((element) => element.outerHTML),
      `no Skip step in ${region}`,
    ).to.deep.eq([]);
  }
}

function untilMyTurn(budget: number): void {
  expect(budget, "the human's turn comes within the budget").to.be.greaterThan(0);
  waitForMoment().then((moment: Moment) => {
    cy.document().then(expectNoSkip);
    if (moment.kind === "turn") return;
    expect(moment.kind, "the lesson is not over before the human's first turn").to.not.eq("over");
    if (moment.kind === "over") return;
    takeMoment(moment).then(() => {
      untilMyTurn(budget - 1);
    });
  });
}

describe("24 — your library, without its order (R310–R313)", () => {
  it("R310 R313 hotseat: your library opens its cards with counts and 'order hidden'; the opponent's is a count; the other seat's opens after the hand-over", () => {
    cy.seedGame({ seed: SEED, a: DECK_A, b: DECK_B });
    cy.handOver();
    expectOwnLibraryBrowsable("24-library");
    expectOpponentLibraryClosed();

    shownSeat().then((first) => {
      cy.endTurn();
      cy.get(`${ts(BOARD)} ${YOUR_SEAT}`).invoke("attr", "data-player").should("not.eq", first);
      expectOwnLibraryBrowsable();
      expectOpponentLibraryClosed();
    });
  });
});

describe("24 — the tutorial (R313, R314)", () => {
  it("R314 R313 lesson 1: no Skip step in the HUD or the bubble, Exit tutorial is there, and your library opens its list", () => {
    visitTutorial(lessonUrl(LESSON), { reducedMotion: true });
    cy.get(ts(TUTORIAL_HUD), { timeout: TUTORIAL_BOOT_TIMEOUT }).should("have.attr", "data-lesson", LESSON);
    cy.get(ts(TUTORIAL_EXIT)).should("be.visible");
    cy.document().then(expectNoSkip);
    untilMyTurn(MOMENT_BUDGET);

    cy.window().then((win) => {
      const view = currentView(win);
      expect(view, "the page holds the human's view").to.not.eq(null);
    });
    cy.get(ts(libraryCountId("you")))
      .invoke("text")
      .then((text) => {
        const count = Number(text);
        expect(count, "the lesson's library has cards").to.be.greaterThan(0);
        // The coach may cover the pile, so force the keyboard action.
        cy.get(`${ts(libraryId("you"))}${BROWSABLE}`).trigger("keydown", { key: "Enter", force: true });
        cy.get(ts(INSPECT_LIST_SHEET)).should("be.visible").and("contain.text", "Your deck");
        cy.get(ts(INSPECT_LIST_SHEET)).invoke("text").should("match", ORDER_HIDDEN);
        cy.get(`${ts(INSPECT_LIST_SHEET)} ${ts(INSPECT_LIST_COUNT)}`).should("have.attr", "data-count", String(count));
        cy.get(ts(INSPECT_CLOSE)).click();
        cy.get(ts(INSPECT_LIST_SHEET)).should("not.exist");
      });
    cy.get(ts(libraryId("opponent"))).should("not.have.attr", "data-browsable");
    cy.document().then(expectNoSkip);
  });
});
