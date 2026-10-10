// BUILD M9-T12 `29-animated-trap.cy.ts`: P2's Tesla animates during P1's turn (R383).
// Read each seat's DOM only (CLAUDE.md rule 7): P1 cannot identify a set Tesla (R33), and Lifesteal
// can heal beyond 30 (R19). An animated Tesla remains in place on later triggers (R383).

import { seedFor } from "../../support/config.ts";
import {
  cardId,
  graveyardCountId,
  healthIs,
  heroId,
  modifiersId,
  positionIs,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { PlayerId } from "../../support/types.ts";

const SEED = seedFor("29-tesla-45");
const FULL_ROW_SEED = seedFor("29-full-65");

/** C #5 damage and SPEC §2 starting health. */
const TESLA_DAMAGE = 4;
const HERO_HEALTH = 30;

/** Tesla animates into its own lane when open (R383). */
const TESLA_LANE = 3;

/** BUILD M5-T3: act from the required hotseat. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== player) cy.handOver();
  });
}

function setTesla(): Cypress.Chainable<string> {
  ensureSeat("p2");
  cy.playByName("Tesla", { zone: { side: "you", row: "backrow", lane: TESLA_LANE } });
  return cy.instanceAt("p2", "backrow", TESLA_LANE);
}

describe("BUILD M9 29: an Animated Field Trap fires on the opponent's turn and becomes a Unit", () => {
  it("R383: Tesla hits the arrival after its Cry, then animates in Defense; a later arrival is hit again", () => {
    cy.seedGame({ seed: SEED, a: "03-plays-a", b: "29-tesla-b" });

    cy.advanceToTurn(2);
    ensureSeat("p2");
    cy.playByName("The Coin");
    setTesla().then((tesla) => {
      cy.advanceToTurn(3);
      ensureSeat("p1");

      // R33: player 1 cannot identify the face-down card.
      cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find('[data-face-down="true"]').should("exist");
      cy.get(ts(cardId(tesla))).should("not.exist");

      cy.handCardByName("Professor Curvature").then((curvature) => {
        cy.playCard(curvature, {
          zone: { side: "you", row: "units", lane: 1 },
          // §10.3: the trap resolves inside player 1's action.
          expectAnimating: "trapFired",
        });

        // R82 ends the turn after Tesla; the device remains on player 1's seat (BUILD M5-T3).
        cy.gameState().should((state) => {
          expect(state.pending, "Tesla asks nothing").to.eq(null);
        });

        // The Cry installs its modifier before Tesla's hit destroys Curvature.
        cy.get(ts(modifiersId("you"))).should("have.attr", "data-count", "1");
        cy.get(ts(cardId(curvature))).should("not.exist");
        cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
        // R19: Lifesteal can heal player 2 beyond 30.
        cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH + TESLA_DAMAGE)).should("exist");

        cy.get(ts(zoneId("opponent", "units", TESLA_LANE))).find(ts(cardId(tesla))).should("exist");
        cy.get(ts(cardId(tesla))).should("have.attr", "data-position", "DEF");
        cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find(ts(cardId(tesla))).should("not.exist");
        cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find('[data-face-down="true"]').should("not.exist");

        cy.handOver();
        cy.get(ts(zoneId("you", "units", TESLA_LANE))).find(ts(cardId(tesla))).should("exist");
        cy.get(ts(zoneId("you", "units", TESLA_LANE))).find(positionIs("DEF")).should("exist");
        cy.get(ts(cardId(tesla))).should("not.have.attr", "data-unrevealed", "true");
        cy.handOver();

        cy.advanceToTurn(5);
        ensureSeat("p1");
        cy.handCardByName("Mr. Vanilla").then((vanilla) => {
          cy.playCard(vanilla, { zone: { side: "you", row: "units", lane: 2 }, expectAnimating: "trapFired" });
          cy.get(ts(cardId(vanilla))).should("not.exist");
          cy.get(ts(graveyardCountId("you"))).should("have.text", "2");
          cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH + 2 * TESLA_DAMAGE)).should("exist");
          // R383: an animated Tesla neither moves nor changes position.
          cy.get(ts(zoneId("opponent", "units", TESLA_LANE))).find(ts(cardId(tesla))).should("exist");
          cy.get(ts(cardId(tesla))).should("have.attr", "data-position", "DEF");
        });
      });
    });
    cy.replayCheck("29-animated-trap");
  });

  it("R383: with player 2's unit row full, Tesla fires and stays face-up in its backrow zone", () => {
    cy.seedGame({ seed: FULL_ROW_SEED, a: "03-plays-a", b: "29-tesla-b" });

    cy.advanceToTurn(2);
    ensureSeat("p2");
    cy.playByName("Friend of Felinors");
    for (const lane of [1, 2, 3, 4, 5] as const) {
      cy.get(ts(zoneId("you", "units", lane))).find('[data-testid^="card-"]').should("exist");
    }

    cy.advanceToTurn(4);
    setTesla().then((tesla) => {
      cy.advanceToTurn(5);
      ensureSeat("p1");
      cy.get(ts(cardId(tesla))).should("not.exist");

      cy.handCardByName("Tempo Timmy").then((timmy) => {
        cy.playCard(timmy, { zone: { side: "you", row: "units", lane: 1 }, expectAnimating: "trapFired" });
        cy.get(ts(cardId(timmy))).should("not.exist");
        cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH + TESLA_DAMAGE)).should("exist");

        cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find(ts(cardId(tesla))).should("exist");
        cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find('[data-face-down="true"]').should("not.exist");
        cy.gameState().should((state) => {
          const side = state.players.p2 as { units?: ({ defId: string }[] | null)[] };
          const units = (side.units ?? []).map((pile) => pile?.[0]?.defId ?? null);
          expect(units, "player 2's row is still the five Felinor Tokens").to.deep.eq(Array(5).fill("core-t-felinor"));
        });

        cy.handOver();
        cy.get(ts(zoneId("you", "backrow", TESLA_LANE))).find(ts(cardId(tesla))).should("exist");
        cy.get(ts(cardId(tesla))).should("not.have.attr", "data-unrevealed", "true");
      });
    });
    cy.replayCheck("29-animated-trap-full-row");
  });
});
