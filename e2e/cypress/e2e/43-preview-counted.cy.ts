// Spec 43 (#991): the board rings the permanents a hand card's preview counts (SPEC §10.10, R280, R583, R1447).
// BUILD M8: use a seeded hotseat game, retried assertions, and test ids.

import { seedFor } from "../../support/config.ts";
import { COUNTED, RADIANT, cardId, handCardId, ts } from "../../support/testids.ts";

const SEED = seedFor("43-counted-1");
const DECK_A = "43-counted-a";
const DECK_B = "08-do-nothing-b";

const SIMPLICITY_AUDIT = "classicplus-044";
const MR_VANILLA = "core-008";
const GLOWY_JELLY_BEAN = "core-026";

describe("43 — the board rings the permanents a hand card's preview counts", () => {
  beforeEach(() => {
    cy.seedGame({ seed: SEED, a: DECK_A, b: DECK_B });
    cy.jackioh().then((handle) => {
      if (handle.seat !== "p1") cy.handOver();
    });
  });

  it("#991 a Radiant Simplicity Audit under the pointer rings Mr. Vanilla, the one permanent it would exile, and nothing once the pointer leaves", () => {
    cy.instanceInHand("p1", MR_VANILLA).then((vanillaId) => {
      cy.playCard(vanillaId, { zone: { side: "you", row: "units", lane: 1 } });
      cy.instanceInHand("p1", SIMPLICITY_AUDIT).then((auditId) => {
        cy.instanceInHand("p1", GLOWY_JELLY_BEAN).then((beanId) => {
          cy.playCard(beanId, { answers: [{ kind: "hand", cards: [auditId] }] });
        });
        const audit = ts(handCardId(auditId));
        cy.get(`${audit}${RADIANT}`).should("exist");

        // The pick above may have left the pointer on the Audit, so reset it first.
        cy.get(audit).trigger("pointerout", { pointerType: "mouse", force: true });
        cy.get(COUNTED).should("not.exist");

        cy.get(audit).trigger("pointerover", { pointerType: "mouse", force: true });
        cy.get(COUNTED).should("have.length", 1).and("have.attr", "data-testid", cardId(vanillaId));

        cy.get(audit).trigger("pointerout", { pointerType: "mouse", force: true });
        cy.get(COUNTED).should("not.exist");
      });
    });
  });
});
