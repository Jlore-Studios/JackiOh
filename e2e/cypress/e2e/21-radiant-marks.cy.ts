// Spec 21: computed values, card references, and Radiant marks (SPEC §10.10; R277, R279, R280).
// R429: Math Equation's first displayed value is its current formula result.
// BUILD M8: use a seeded hotseat game, retried assertions, and test ids.

import { seedFor, timeouts } from "../../support/config.ts";
import {
  CARD_REF,
  CARD_REF_TOOLTIP,
  CARD_VALUE,
  INSPECT_CLOSE,
  INSPECT_HOVER,
  INSPECT_REFS,
  INSPECT_REFS_FACE,
  INSPECT_SHEET,
  RADIANT,
  RADIANT_MARK,
  handCardId,
  ts,
} from "../../support/testids.ts";

/** This seed deals #31, #26, and #65 and gives player 1 three mana on turn three. */
const SEED = seedFor("21-marks-163");
const DECK_A = "21-marks-a";
const DECK_B = "08-do-nothing-b";

const MATH_EQUATION = "core-031";
const GLOWY_JELLY_BEAN = "core-026";
const MASOCHISM_MASK = "core-065";
const SPIKEY_PILLOW = "core-065-1";
/** Player 1's third turn: 3 mana, what #26 costs (§2.3). */
const THIRD_TURN = 5;

const inHand = (defId: string): string => `[data-testid^="hand-card-"][data-def-id="${defId}"]`;

function restOn(selector: string): void {
  cy.get(selector).trigger("pointerover", { pointerType: "mouse" });
}

function leave(selector: string): void {
  cy.get(selector).trigger("pointerout", { pointerType: "mouse" });
  cy.get(ts(INSPECT_HOVER)).should("not.exist");
}

describe("21 — a card's marks in play: a computed value, a reference and a Radiant face's gold", () => {
  beforeEach(() => {
    cy.seedGame({ seed: SEED, a: DECK_A, b: DECK_B });
    cy.jackioh().then((handle) => {
      if (handle.seat !== "p1") cy.handOver();
    });
  });

  it("R280 KY's Math Equation in hand prints what its formula comes to now", () => {
    restOn(inHand(MATH_EQUATION));
    cy.get(`${ts(INSPECT_HOVER)} ${CARD_VALUE}`, { timeout: timeouts.view })
      .should("be.visible")
      .and("have.text", "{1}")
      .and("have.attr", "data-label", "Fib(times played + 1)");
    cy.get(`${ts(INSPECT_HOVER)} .card-text`).should("contain.text", "Deal Fib(times played + 1) {1} damage");
    leave(inHand(MATH_EQUATION));
  });

  it("R279 Masochism Mask names Spikey Pillow: its face is beside the preview, and the sheet's reference opens it", () => {
    restOn(inHand(MASOCHISM_MASK));
    cy.get(`${ts(INSPECT_HOVER)} ${ts(INSPECT_REFS)}`, { timeout: timeouts.view })
      .should("be.visible")
      .find(`${INSPECT_REFS_FACE}[data-ref="${SPIKEY_PILLOW}"]`)
      .should("have.length", 1)
      .find(".card-name")
      .should("have.text", "Spikey Pillow");
    leave(inHand(MASOCHISM_MASK));

    cy.get(inHand(MASOCHISM_MASK)).trigger("pointerdown", { pointerType: "touch", pointerId: 7, button: 0 });
    cy.get(ts(INSPECT_SHEET), { timeout: timeouts.view }).should("be.visible");
    cy.get(inHand(MASOCHISM_MASK)).trigger("pointerup", { pointerType: "touch", pointerId: 7, button: 0, force: true });
    cy.get(`${ts(INSPECT_SHEET)} ${CARD_REF}[data-ref="${SPIKEY_PILLOW}"]`)
      .first()
      .should("have.attr", "tabindex", "0")
      .click();
    cy.get(ts(CARD_REF_TOOLTIP))
      .should("be.visible")
      .and("have.attr", "role", "tooltip")
      .and("have.attr", "data-ref", SPIKEY_PILLOW)
      .find(".card-name")
      .should("have.text", "Spikey Pillow");
    cy.get(`${ts(INSPECT_SHEET)} ${CARD_REF}[data-ref="${SPIKEY_PILLOW}"]`)
      .first()
      .invoke("attr", "aria-describedby")
      .then((described) => {
        cy.get(ts(CARD_REF_TOOLTIP)).should("have.attr", "id", described ?? "");
      });
    cy.get(ts(INSPECT_CLOSE)).click();
    cy.get(ts(INSPECT_SHEET)).should("not.exist");
  });

  it("R277 a Math Equation made Radiant prints Fib(times played + 3) with the 3 in gold, and its new value", () => {
    cy.advanceToTurn(THIRD_TURN);
    cy.jackioh().then((handle) => {
      if (handle.seat !== "p1") cy.handOver();
    });
    cy.instanceInHand("p1", MATH_EQUATION).then((mathId) => {
      cy.get(`${ts(handCardId(mathId))}${RADIANT}`).should("not.exist");
      cy.instanceInHand("p1", GLOWY_JELLY_BEAN).then((beanId) => {
        cy.playCard(beanId, { answers: [{ kind: "hand", cards: [mathId] }] });
      });
      cy.get(`${ts(handCardId(mathId))}${RADIANT}`).should("exist");

      restOn(ts(handCardId(mathId)));
      cy.get(`${ts(INSPECT_HOVER)} .card-text`, { timeout: timeouts.view })
        .should("contain.text", "Deal Fib(times played + 3) {3} damage")
        .find(RADIANT_MARK)
        .should("have.length", 1)
        .and("have.text", "3")
        .and(($mark) => {
          const style = getComputedStyle($mark[0] as Element);
          expect(Number(style.fontWeight), "the mark is bold").to.be.at.least(700);
          expect(style.textDecorationLine, "the mark is underlined").to.contain("underline");
        });
      cy.get(`${ts(INSPECT_HOVER)} ${CARD_VALUE}`).should("have.text", "{3}").and("have.attr", "data-label", "Fib(times played + 3)");
      leave(ts(handCardId(mathId)));
    });
  });
});
