// BUILD M9-T12: C #45 Nature Titan's Tribute follows R391; R64 and R13 exclude lanes 1 and 2.
// #3 Right-house defender reserves lane 1; #92 Felinor Fiender stacks in lane 2.
// R180 provides the full-row setup. DOM assertions use player 1's view (CLAUDE.md rule 7).

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

        // The last candidate resolves the Tribute immediately.
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
