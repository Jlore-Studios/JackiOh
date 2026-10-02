// BUILD M9-T12 `31-counter-opponent-turn.cy.ts`: "P2 has C #17 Counterspell set; P1 plays a Spell"
// (§10.5's announce window, R17, R448; issue #66).
//
// What it asserts, read off the DOM player 1's view drew (CLAUDE.md rule 7), plus the play counts,
// which no element draws:
//
//   1. Player 1 plays #5 Stockpile (1 mana: "Draw 2. Heal your hero 2."). It is announced
//      (`cardAnnounced`), Counterspell fires inside player 1's turn (`trapFired`), and the Spell is
//      `countered`.
//   2. It never resolved. It is in player 1's graveyard, the hand is one card smaller and did not draw
//      2, the crystal it cost stays spent, and neither the game's nor the turn's play count moved
//      (no `cardPlayed`: "treated as never played", so Combo has nothing to count). Counterspell is
//      spent from player 2's backrow.
//   3. A countered card reaches no other trap (R17). #60 Bear Honeypot, set beside Counterspell,
//      springs on any 1-cost card, but it is still face-down and player 2's unit row is still empty.
//      Issue #66 names Sheepish for this. Sheepish answers only Units and C #17 counters only
//      Spells, so no countered card could ever reach Sheepish; the Honeypot is the trap a countered
//      Spell would have sprung.
//   4. The Honeypot was armed all along: the 1-cost Unit player 1 plays next is not countered, and it
//      springs the Honeypot (`trapFired`), which goes to player 2's graveyard.
//
// Decks: player 1 plays spec 03's `03-plays-a`. `31-counter-b` is spec 03's player-2 deck with
// Counterspell and Bear Honeypot as its only Traps. Seed 31-counter-74 was chosen by replaying these
// moves against the engine. It has Counterspell and The Coin in player 2's hand on player-turn 2,
// Bear Honeypot there on player-turn 4, and Stockpile and Mr. Vanilla in player 1's hand on
// player-turn 5.

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

/** The game's plays (`state.counters.played`) and player 1's this turn (`turnLog.cardsPlayed`). */
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

        // R33: two face-down cards, neither named on player 1's screen.
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
              // §10.5: announced first, then the trap answers inside the announce window.
              cy.expectAnimating("cardAnnounced");
              cy.expectAnimating("trapFired");
              cy.expectAnimating("countered");
              cy.settled();

              // Countered: to its owner's graveyard, unresolved, its crystal spent.
              cy.get(ts(handCardId(stockpile))).should("not.exist");
              cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
              cy.get(HAND_CARDS).should("have.length", handBefore - 1);
              cy.get(ts(manaId("you"))).should("have.attr", "data-current", "2");
              playCounts().should((after) => {
                expect(after, "a countered card is treated as never played (no Combo count)").to.deep.eq(before);
              });

              // Counterspell is spent; the Honeypot beside it never saw the card.
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

              // Counterspell answers Spells only: the Unit is not countered, and it springs the Honeypot.
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
