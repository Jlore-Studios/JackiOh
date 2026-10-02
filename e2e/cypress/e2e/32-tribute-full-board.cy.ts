// BUILD M9-T12 `32-tribute-full-board.cy.ts`: player 1's five unit zones full, C #45 Nature Titan
// (Tribute 1) in hand (R391; issue #66).
//
// What it asserts, read off the DOM player 1's view drew (CLAUDE.md rule 7):
//
//   1. On a full unit row the Tribute card is still playable. The hand card is live, and picking it
//      up opens the Tribute picker.
//   2. The zones it may go into glow as legal: lanes 3, 4 and 5, whose single Felinor Token its own
//      Tribute would empty. Lane 1 is never offered, because its #3 Right-house defender has Reborn,
//      so its zone would be reserved for the return (R64). Lane 2 is never offered either: #92
//      Felinor Fiender sits on a pile over a Felinor Token, and tributing the top frees nothing (R13).
//   3. Paying the lane-4 Felinor plays the Titan into lane 4. The token is gone, and the Titan's Cry
//      resolves: it draws 1 and heals player 1's hero 3, from 30 to 33.
//
// Decks: `32-tribute-a` is spec 03's player-1 deck with Nature Titan swapped in. Its handicap
// (R180) gives player 1 four crystals and a ten-card hand from turn 1, so the whole row is filled on
// player 1's first turn: Right-house defender in lane 1, then Friend of Felinors for lanes 2-5, then
// Felinor Fiender stacked onto lane 2. The fixture's description has the reasoning. `30-quiet-b`
// does nothing. Seed 32-tribute-18 was chosen by replaying these moves against the engine; it deals
// all four cards into player 1's opening hand.

import { seedFor } from "../../support/config.ts";
import { cardId, handCardId, healthIs, heroId, ts, zoneId } from "../../support/testids.ts";
import type { PlayerId } from "../../support/types.ts";

const SEED = seedFor("32-tribute-18");

const HERO_HEALTH = 30;
/** C #45's base `heal`. */
const TITAN_HEAL = 3;

/** R391: the zones a Tribute empties, and the two it never does. */
const OFFERED = [3, 4, 5] as const;
const NEVER = [1, 2] as const;
const INTO = 4;

const GLOWING = '[data-glow="ready"]';

function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== player) cy.handOver();
  });
}

describe("BUILD M9 32: a Tribute card on a full unit row (R391)", () => {
  it("R391: only the zones a Tribute empties are offered, and the card lands in the one it paid for", () => {
    cy.seedGame({ seed: SEED, a: "32-tribute-a", b: "30-quiet-b" });

    // Player 1's first turn: fill the row.
    ensureSeat("p1");
    cy.playByName("Right-house defender", { zone: { side: "you", row: "units", lane: 1 } });
    cy.playByName("Friend of Felinors");
    cy.playByName("Felinor Fiender", { zone: { side: "you", row: "units", lane: 2 } });
    for (const lane of [1, 2, 3, 4, 5] as const) {
      cy.get(ts(zoneId("you", "units", lane))).find('[data-testid^="card-"]').should("exist");
    }
    cy.gameState().should((state) => {
      const side = state.players.p1 as { units?: ({ id: string }[] | null)[] };
      const piles = (side.units ?? []).map((pile) => pile?.length ?? 0);
      expect(piles, "five full zones, lane 2 a pile of two").to.deep.eq([1, 2, 1, 1, 1]);
    });

    cy.advanceToTurn(3);
    ensureSeat("p1");

    cy.instanceAt("p1", "units", INTO).then((paid) => {
      cy.handCardByName("Nature Titan").then((titan) => {
        cy.get(ts(handCardId(titan))).should("have.attr", "data-legal", "true");
        cy.get(ts(handCardId(titan))).click();
        cy.waitForPrompt("tribute");

        for (const lane of OFFERED) {
          cy.get(ts(zoneId("you", "units", lane))).should("have.attr", "data-legal", "true");
          cy.get(ts(zoneId("you", "units", lane))).should("match", GLOWING);
        }
        for (const lane of NEVER) {
          cy.get(ts(zoneId("you", "units", lane))).should("not.have.attr", "data-legal", "true");
          cy.get(ts(zoneId("you", "units", lane))).should("not.match", GLOWING);
        }

        // Paying lane 4's Felinor is the one candidate left, so the play goes at once.
        cy.answerPrompt("tribute", { cards: [paid] });
        cy.get(ts(cardId(paid))).should("not.exist");
        cy.get(ts(zoneId("you", "units", INTO))).find(ts(cardId(titan))).should("exist");
        cy.get(ts(heroId("you"))).find(healthIs(HERO_HEALTH + TITAN_HEAL)).should("exist");
        cy.gameState().should((state) => {
          const side = state.players.p1 as { units?: ({ id: string }[] | null)[] };
          const piles = (side.units ?? []).map((pile) => pile?.length ?? 0);
          expect(piles, "the Titan took the zone it paid for and nothing else moved").to.deep.eq([1, 2, 1, 1, 1]);
        });
      });
    });
    cy.replayCheck("32-tribute-full-board");
  });
});
