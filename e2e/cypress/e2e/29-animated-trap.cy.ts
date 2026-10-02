// BUILD M9-T12 `29-animated-trap.cy.ts`: "P2 has C #5 Tesla set; P1 plays a Unit" (R383, issue #66).
//
// What it asserts, read off the DOM each seat's view drew (CLAUDE.md rule 7):
//
//   1. While Tesla is set, player 1's screen has a face-down card in player 2's backrow and no
//      `card-<instanceId>` for it (R33).
//   2. Player 1 plays #77 Professor Curvature. Tesla fires inside player 1's turn (`trapFired`), and
//      only after the Unit has resolved, its Cry included: Curvature's modifier badge stands by
//      player 1's hero after Curvature is gone. The 4 damage destroys the 3/3. Tesla's printed
//      Lifesteal heals player 2's hero for the 4 dealt (30 to 34; R19 puts no cap on a hero heal).
//   3. Tesla then steps into player 2's unit zone in its own lane (`animated`), in Defense Position,
//      face-up on both seats, and its backrow zone is empty.
//   4. A later arrival is hit again while Tesla is a Unit: #8 Mr. Vanilla (4/4) dies to the second
//      4, player 2's hero goes to 38, and Tesla neither moves nor changes position (R383: "one
//      already a Unit when it fires again neither moves nor changes position").
//   5. A second game, with player 2's unit row already filled by #62 Friend of Felinors: Tesla fires
//      and has no unit zone to step into, so it stays face-up in its backrow zone, on both seats.
//
// Decks: player 1 plays spec 03's `03-plays-a`. Player 2 plays `29-tesla-b`, which is spec 03's
// player-2 deck with Sheepish swapped for Tesla. Seeds were chosen by replaying these moves against
// the engine, as issue #66 describes:
//   * 29-tesla-45: Tesla and The Coin are in player 2's hand on player-turn 2 (2 mana with the Coin).
//     Curvature is in player 1's hand on player-turn 3 and Mr. Vanilla on player-turn 5.
//   * 29-full-65: Friend of Felinors is in player 2's hand on player-turn 2, Tesla on player-turn 4,
//     and #11 Tempo Timmy (3/3, no Cry) in player 1's hand on player-turn 5.

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

/** C #5's base damage (its `damage` param) and SPEC §2's starting health. */
const TESLA_DAMAGE = 4;
const HERO_HEALTH = 30;

/** Tesla's own lane: the unit zone it animates into is the one in its lane when that is open (R383). */
const TESLA_LANE = 3;

/** BUILD M5-T3: a hotseat device is handed over, so put it on the seat that has to act. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== player) cy.handOver();
  });
}

/** Player 2 sets Tesla face-down in backrow lane 3 and yields its instance id. */
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
    // Player 2's first turn has one crystal; The Coin makes the two that Tesla costs.
    cy.playByName("The Coin");
    setTesla().then((tesla) => {
      cy.advanceToTurn(3);
      ensureSeat("p1");

      // R33: player 1 sees a face-down card and nothing that names it.
      cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find('[data-face-down="true"]').should("exist");
      cy.get(ts(cardId(tesla))).should("not.exist");

      cy.handCardByName("Professor Curvature").then((curvature) => {
        cy.playCard(curvature, {
          zone: { side: "you", row: "units", lane: 1 },
          // The trap flips inside player 1's action (§10.3), then steps into its unit zone.
          expectAnimating: "trapFired",
        });

        // Curvature spends player 1's last crystal, so R82 ends the turn by itself once Tesla is
        // done; the device stays on player 1's seat until it is handed over (BUILD M5-T3).
        cy.gameState().should((state) => {
          expect(state.pending, "Tesla asks nothing").to.eq(null);
        });

        // After the Cry: the Cry installed its modifier before the hit destroyed the 3/3.
        cy.get(ts(modifiersId("you"))).should("have.attr", "data-count", "1");
        cy.get(ts(cardId(curvature))).should("not.exist");
        cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
        // Lifesteal: the 4 dealt heals player 2's hero, with no cap at 30 (R19).
        cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH + TESLA_DAMAGE)).should("exist");

        // Animated: face-up in player 2's unit zone in its own lane, in Defense Position, and gone
        // from the backrow, as player 1 sees it…
        cy.get(ts(zoneId("opponent", "units", TESLA_LANE))).find(ts(cardId(tesla))).should("exist");
        cy.get(ts(cardId(tesla))).should("have.attr", "data-position", "DEF");
        cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find(ts(cardId(tesla))).should("not.exist");
        cy.get(ts(zoneId("opponent", "backrow", TESLA_LANE))).find('[data-face-down="true"]').should("not.exist");

        // …and as player 2 sees it.
        cy.handOver();
        cy.get(ts(zoneId("you", "units", TESLA_LANE))).find(ts(cardId(tesla))).should("exist");
        cy.get(ts(zoneId("you", "units", TESLA_LANE))).find(positionIs("DEF")).should("exist");
        cy.get(ts(cardId(tesla))).should("not.have.attr", "data-unrevealed", "true");
        cy.handOver();

        // A later arrival, two turns on: Tesla is a Unit now and still fires.
        cy.advanceToTurn(5);
        ensureSeat("p1");
        cy.handCardByName("Mr. Vanilla").then((vanilla) => {
          cy.playCard(vanilla, { zone: { side: "you", row: "units", lane: 2 }, expectAnimating: "trapFired" });
          cy.get(ts(cardId(vanilla))).should("not.exist");
          cy.get(ts(graveyardCountId("you"))).should("have.text", "2");
          cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH + 2 * TESLA_DAMAGE)).should("exist");
          // R383: already a Unit, it neither moves nor changes position.
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

        // No open unit zone: it stays where it is, face-up, so player 1 now sees what it is.
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
