// BUILD M9-T12 `31-counter-opponent-turn.cy.ts`: P2's C #17 Counterspell announces for P1's Spell (§10.5, R17, R448).
// Read player 1's DOM and unrendered play counts (CLAUDE.md rule 7). A countered Spell neither
// resolves nor reaches another trap (R17).

import { seedFor } from "../../support/config.ts";
import {
  cardId,
  graveyardCountId,
  handCardId,
  manaId,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { PlayerId } from "../../support/types.ts";

const SEED = seedFor("31-counter-74");

const COUNTER_LANE = 2;
const HONEYPOT_LANE = 3;

type PlayCounts = { game: number; turn: number };

function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== player) cy.handOver();
  });
}

function playCounts(): Cypress.Chainable<PlayCounts> {
  return cy.gameState().then((state) => {
    const counters = (state as { counters?: { played?: number } }).counters;
    const side = state.players.p1 as { turnLog?: { cardsPlayed?: number } };
    return { game: counters?.played ?? -1, turn: side.turnLog?.cardsPlayed ?? -1 };
  });
}

const HAND_CARDS = '[data-testid^="hand-card-"]';

describe("BUILD M9 31: a Counter answers the opponent's Spell on their turn", () => {
  it("§10.5 / R17: Counterspell counters Stockpile on player 1's turn, and the Honeypot beside it never sees it", () => {
    cy.seedGame({ seed: SEED, a: "03-plays-a", b: "31-counter-b" });

    cy.advanceToTurn(2);
    ensureSeat("p2");
    cy.playByName("The Coin");
    cy.playByName("Counterspell", { zone: { side: "you", row: "backrow", lane: COUNTER_LANE } });
    cy.advanceToTurn(4);
    ensureSeat("p2");
    cy.playByName("Bear Honeypot", { zone: { side: "you", row: "backrow", lane: HONEYPOT_LANE } });

    cy.instanceAt("p2", "backrow", COUNTER_LANE).then((counterspell) => {
      cy.instanceAt("p2", "backrow", HONEYPOT_LANE).then((honeypot) => {
        cy.advanceToTurn(5);
        ensureSeat("p1");

        // R33: player 1 cannot identify either face-down card.
        for (const lane of [COUNTER_LANE, HONEYPOT_LANE] as const) {
          cy.get(ts(zoneId("opponent", "backrow", lane))).find('[data-face-down="true"]').should("exist");
        }
        cy.get(ts(cardId(counterspell))).should("not.exist");
        cy.get(ts(cardId(honeypot))).should("not.exist");

        cy.get(HAND_CARDS).its("length").then((handBefore) => {
          playCounts().then((before) => {
            cy.get(ts(manaId("you"))).should("have.attr", "data-current", "3");
            cy.get(ts(graveyardCountId("you"))).should("have.text", "0");

            cy.handCardByName("Stockpile").then((stockpile) => {
              cy.get(ts(handCardId(stockpile))).click();
              // §10.5: announce before the trap responds.
              cy.expectAnimating("cardAnnounced");
              cy.expectAnimating("trapFired");
              cy.expectAnimating("countered");
              cy.settled();

              cy.get(ts(handCardId(stockpile))).should("not.exist");
              cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
              cy.get(HAND_CARDS).should("have.length", handBefore - 1);
              cy.get(ts(manaId("you"))).should("have.attr", "data-current", "2");
              playCounts().should((after) => {
                expect(after, "a countered card is treated as never played (no Combo count)").to.deep.eq(before);
              });

              cy.get(ts(zoneId("opponent", "backrow", COUNTER_LANE))).find('[data-face-down="true"]').should("not.exist");
              cy.get(ts(zoneId("opponent", "backrow", HONEYPOT_LANE))).find('[data-face-down="true"]').should("exist");
              cy.get(ts(cardId(honeypot))).should("not.exist");
              for (const lane of [1, 2, 3, 4, 5] as const) {
                cy.get(ts(zoneId("opponent", "units", lane))).find('[data-testid^="card-"]').should("not.exist");
              }
              cy.gameState().should((state) => {
                expect(state.active, "still player 1's turn").to.eq("p1");
                expect(state.pending).to.eq(null);
              });

              cy.handCardByName("Mr. Vanilla").then((vanilla) => {
                cy.playCard(vanilla, { zone: { side: "you", row: "units", lane: 1 }, expectAnimating: "trapFired" });
                cy.get(ts(zoneId("opponent", "backrow", HONEYPOT_LANE))).find('[data-face-down="true"]').should("not.exist");
                cy.gameState().should((state) => {
                  const side = state.players.p2 as { graveyard?: { id: string }[]; backrow?: unknown[] };
                  const graveyard = (side.graveyard ?? []).map((card) => card.id);
                  expect(graveyard, "both traps are spent").to.include.members([counterspell, honeypot]);
                });
              });
            });
          });
        });
      });
    });
    cy.replayCheck("31-counter-opponent-turn");
  });
});
