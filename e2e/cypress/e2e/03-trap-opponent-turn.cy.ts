// BUILD M8: P2's Sheepish interrupts then resumes P1's play (R1, R17, R23, R118, R427; §10.3).
// #15 Me and Mr Token triggers it; #19 Midrange Menace and #81 Radiant Saintess verify later plays.
// The trap validates hidden information (R33; CLAUDE.md rule 7, §10.8); the modifier validates face-up views (R169).
// R48's discount is dormant until its controller's next turn, then expires.

import { seedFor } from "../../support/config.ts";
import {
  END_TURN,
  MODIFIER_BADGE,
  cardId,
  graveyardCountId,
  handCardId,
  modifierBadgeOf,
  modifiersId,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { PlayerId, Side } from "../../support/types.ts";

const SEED = seedFor("03-sheep-19");

/** §8 seed brings #77 in on player-turn 3. */
const CURVATURE_SEED = seedFor("03-curvature-4");

/** §8 #77's card label. */
const CURVATURE = "Professor Curvature";

/** R48 / R65: exact text distinguishes a dormant discount from a live one. */
const CURVATURE_DISCOUNT = "(4)+ Cost cards cost (1) less";
const CURVATURE_DORMANT = `${CURVATURE_DISCOUNT} (next turn)`;

/** BUILD M5-T1 viewports. */
const DESKTOP = { width: 1280, height: 720 } as const;
const PHONE = { width: 390, height: 844 } as const;

const SIDES: readonly Side[] = ["you", "opponent"];

/** BUILD M5-T4 stat hooks. */
const attackIs = (n: number): string => `[data-attack="${n}"]`;
const healthIs = (n: number): string => `[data-health="${n}"]`;

/** BUILD M5-T3: hand the hotseat to the actor. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat (the hotseat handle names the seat holding it)").to.not.eq(
      undefined,
    );
    if (handle.seat !== player) cy.handOver();
  });
}

/** R82 can auto-end a turn, so advance by the §10.1 turn counter. */
function advanceToTurn(turn: number, budget = 8): void {
  cy.gameState().then((state) => {
    expect(budget, `turn ${turn} is reachable inside the budget`).to.be.greaterThan(0);
    ensureSeat(state.active);
    if (state.turn === turn) return;
    cy.endTurn();
    advanceToTurn(turn, budget - 1);
  });
}

describe("BUILD M8 03 — a trap fires on the other player's turn; a face-up modifier shows on both seats", () => {
  it("R17 / R118 — Sheepish flips during P1's turn, the unit is a Sheep, and P1 plays on", () => {
    cy.seedGame({ seed: SEED, a: "03-plays-a", b: "03-sheepish-b" });

    advanceToTurn(2);

    // §5.1 sets Sheepish face-down.
    cy.playByName("Sheepish", { zone: { side: "you", row: "backrow", lane: 3 } });
    cy.instanceAt("p2", "backrow", 3).then((trap) => {
      // R33 lets only the controller identify a set trap.
      cy.get(ts(zoneId("you", "backrow", 3))).find(ts(cardId(trap))).should("exist");

      advanceToTurn(3);

      // R33 / §10.8: the other seat never receives the set trap's identity.
      cy.get(ts(zoneId("opponent", "backrow", 3))).find(ts(cardId(trap))).should("not.exist");
      cy.get(ts(cardId(trap))).should("not.exist");

      cy.handCardByName("Me and Mr Token").then((played) => {
        // Click through the play to observe the in-flight trap animation.
        cy.get(ts(handCardId(played))).click();
        cy.get(ts(zoneId("you", "units", 1))).click();

        // BUILD M5-T4 / §10.3: the flip resolves inside player 1's action.
        cy.expectAnimating("trapFired");
        cy.settled();
        cy.gameState().should((state) => {
          expect(state.active, "the trap fired inside player 1's turn (§10.3)").to.eq("p1");
          expect(state.pending, "Sheepish asks nothing, so nothing is paused").to.eq(null);
        });

        // §6.3 Transform replaces the card in its chosen zone.
        cy.get(ts(cardId(played))).should("not.exist");
        cy.get(ts(handCardId(played))).should("not.exist");
        cy.fieldCardByName("Sheep Token").then((sheep) => {
          cy.get(ts(zoneId("you", "units", 1))).find(ts(cardId(sheep))).should("exist");
          // §7: Sheep is 1/1.
          cy.get(ts(cardId(sheep))).find(attackIs(1)).should("exist");
          cy.get(ts(cardId(sheep))).find(healthIs(1)).should("exist");

          // R17 / R427 resolve the Cry before Transform; R64 chooses the token's lane.
          cy.gameState().should((state) => {
            const side = state.players.p1 as { units?: ({ id: string }[] | null)[] };
            const occupied = (side.units ?? []).filter((pile) => pile !== null && pile.length > 0);
            expect(occupied, "R427: the Cry resolved before the Transform, so a Rush Token was summoned").to.have.length(
              2,
            );
          });
          cy.fieldCardByName("Rush Token").should("exist");

          // A spent trap goes to its owner's graveyard (§3.2, §5.1).
          cy.get(ts(cardId(trap))).should("not.exist");
          cy.get(ts(graveyardCountId("opponent"))).should("have.text", "1");

          // R118 resumes player 1's turn.
          cy.get(ts(END_TURN)).should("not.be.disabled");
          cy.playByName("Radiant Saintess", { zone: { side: "you", row: "units", lane: 3 } });
          cy.fieldCardByName("Radiant Saintess").should("exist");

          cy.get(ts(cardId(sheep))).should("not.have.attr", "data-radiant", "true");
          cy.fieldCardByName("Radiant Saintess").then((saintess) => {
            cy.get(ts(cardId(saintess))).should("not.have.attr", "data-radiant", "true");
          });

          advanceToTurn(4);
          cy.gameState().should((state) => {
            expect(state.turn, "player 1's turn finished").to.eq(4);
            expect(state.active, "and player 2 is up").to.eq("p2");
            expect(state.result, "the game is still running").to.eq(null);
          });
        });
      });
    });
  });

  it("R169 / R48 — Professor Curvature's badge is on both seats, says it is not live yet, and goes at cleanup", () => {
    cy.seedGame({ seed: CURVATURE_SEED, a: "03-plays-a", b: "03-sheepish-b" });

    // Empty modifier containers persist for the departing-badge animation.
    for (const side of SIDES) {
      cy.get(ts(modifiersId(side))).should("exist").and("have.attr", "data-count", "0");
      cy.get(ts(modifiersId(side))).find(MODIFIER_BADGE).should("not.exist");
    }

    cy.advanceToTurn(3);
    ensureSeat("p1");

    cy.playByName(CURVATURE, {
      zone: { side: "you", row: "units", lane: 2 },
      // BUILD M5-T4 observes `modifierChanged` before the command settles (R169).
      expectAnimating: "modifierChanged",
    });

    // R169: controller sees the view's one badge.
    cy.get(ts(modifiersId("you"))).should("have.attr", "data-count", "1");
    cy.get(ts(modifiersId("you"))).find(MODIFIER_BADGE).should("have.length", 1);
    // R48: the landing turn's discount remains dormant.
    cy.get(ts(modifiersId("you"))).find(MODIFIER_BADGE).should("have.text", CURVATURE_DORMANT);
    cy.get(ts(modifiersId("opponent"))).should("have.attr", "data-count", "0");

    cy.get(ts(modifiersId("you")))
      .find(MODIFIER_BADGE)
      .invoke("attr", "data-modifier-id")
      .should("be.a", "string")
      .then((raw) => {
        const modifierId = String(raw);

        // R169 shows the same modifier to both seats without exposing `mods` (§10.8).
        cy.handOver();
        cy.get(ts(modifiersId("opponent"))).should("have.attr", "data-count", "1");
        cy.get(ts(modifiersId("opponent"))).find(modifierBadgeOf(modifierId)).should("exist");
        cy.get(ts(modifiersId("opponent")))
          .find(MODIFIER_BADGE)
          .should("have.text", CURVATURE_DORMANT);
        cy.get(ts(modifiersId("you"))).should("have.attr", "data-count", "0");
        cy.get(ts(modifiersId("you"))).find(MODIFIER_BADGE).should("not.exist");

        // This is the only browser check for a visible badge at phone width.
        cy.viewport(PHONE.width, PHONE.height);
        cy.get(ts(modifiersId("opponent"))).find(modifierBadgeOf(modifierId)).should("be.visible");
        cy.get(ts(modifiersId("opponent")))
          .find(MODIFIER_BADGE)
          .should("have.text", CURVATURE_DORMANT);
        cy.document({ log: false }).should((doc) => {
          expect(
            doc.documentElement.scrollWidth,
            `the badge did not widen the document past ${String(PHONE.width)}px`,
          ).to.be.at.most(PHONE.width);
          expect(doc.body.scrollWidth, `nor the body past ${String(PHONE.width)}px`).to.be.at.most(
            PHONE.width,
          );
        });
        cy.viewport(DESKTOP.width, DESKTOP.height);

        // R48 remains dormant on the opponent's intervening turn.
        cy.advanceToTurn(4);
        cy.gameState().should((state) => {
          expect(state.active, "player 2's turn").to.eq("p2");
        });
        cy.get(ts(modifiersId("opponent"))).find(modifierBadgeOf(modifierId)).should("exist");
        cy.get(ts(modifiersId("opponent")))
          .find(MODIFIER_BADGE)
          .should("have.text", CURVATURE_DORMANT);

        // The controller's next turn activates the existing badge.
        cy.advanceToTurn(5);
        ensureSeat("p1");
        cy.get(ts(modifiersId("you"))).should("have.attr", "data-count", "1");
        cy.get(ts(modifiersId("you"))).find(modifierBadgeOf(modifierId)).should("exist");
        cy.get(ts(modifiersId("you"))).find(MODIFIER_BADGE).should("have.text", CURVATURE_DISCOUNT);

        // R48 expires at that turn's cleanup (§2.2).
        cy.advanceToTurn(6);
        cy.get(modifierBadgeOf(modifierId)).should("not.exist");
        for (const side of SIDES) {
          cy.get(ts(modifiersId(side))).should("exist").and("have.attr", "data-count", "0");
          cy.get(ts(modifiersId(side))).find(MODIFIER_BADGE).should("not.exist");
        }
        cy.gameState().should((state) => {
          const side = state.players.p1 as { mods?: unknown[] };
          expect(side.mods ?? [], "R48: expired at the cleanup of player 1's next turn").to.have.length(
            0,
          );
        });
      });
  });
});
