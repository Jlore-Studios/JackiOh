// BUILD M8 computes combat results before reading the UI (§3, §4.1, §4.3, §4.4, §4.5; R93).
// BUILD M5-T4 damage and heal pops are asserted while the animation runs, then against durable stats.

import { seedFor } from "../../support/config.ts";
import {
  DAMAGE_POP,
  HEAL_POP,
  cardId,
  graveyardCountId,
  heroId,
  ts,
} from "../../support/testids.ts";
import type { PlayerId } from "../../support/types.ts";

const SEED = seedFor("04-combat-1604");

/** BUILD M5-T4 card hooks. */
const attackIs = (n: number): string => `[data-attack="${n}"]`;
const healthIs = (n: number): string => `[data-health="${n}"]`;
const maxHealthIs = (n: number): string => `[data-max-health="${n}"]`;
const armorIs = (n: number): string => `[data-armor="${n}"]`;
const keywordIs = (keyword: string): string => `[data-keyword="${keyword}"]`;

/** BUILD M5-T3: hand the hotseat to the actor. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat (the hotseat handle names the seat holding it)").to.not.eq(
      undefined,
    );
    if (handle.seat !== player) cy.handOver();
  });
}

/** Reach a §10.1 player-turn. */
function advanceToTurn(turn: number, budget = 12): void {
  cy.gameState().then((state) => {
    expect(budget, `turn ${turn} is reachable inside the budget`).to.be.greaterThan(0);
    ensureSeat(state.active);
    if (state.turn === turn) return;
    cy.endTurn();
    advanceToTurn(turn, budget - 1);
  });
}

/** BUILD M5-T2 / CLAUDE.md rule 7: highlights render `legalActions`. */
function expectHighlight(testid: string, legal: boolean, why: string): void {
  cy.get(ts(testid)).should(($element) => {
    expect($element.attr("data-legal"), why).to.eq(legal ? "true" : "false");
  });
}

/** BUILD M5-T2 attack selection. */
function selectAttacker(instanceId: string): void {
  cy.get(ts(cardId(instanceId))).click();
}

describe("BUILD M8 04 — Taunt, Defense Position, First Strike and Divine Shield on the board", () => {
  it("§4.2 step 3 — Taunt leaves every other target un-highlighted", () => {
    cy.seedGame({ seed: SEED, a: "04-combat-a", b: "04-combat-b" });

    ensureSeat("p1");
    cy.playByName("Tempo Timmy", { zone: { side: "you", row: "units", lane: 1 } });

    advanceToTurn(2);
    cy.playByName("Mr. Vanilla", { zone: { side: "you", row: "units", lane: 1 } });
    advanceToTurn(4);
    cy.playByName("Jilliax", { zone: { side: "you", row: "units", lane: 2 } });

    advanceToTurn(5);
    cy.instanceAt("p1", "units", 1).then((timmy) => {
      cy.instanceAt("p2", "units", 1).then((vanilla) => {
        cy.instanceAt("p2", "units", 2).then((jilliax) => {
          cy.get(ts(cardId(jilliax))).find(keywordIs("Taunt")).should("exist");

          selectAttacker(timmy);
          expectHighlight(cardId(jilliax), true, "the Taunt unit is the one legal target (§4.2 step 3)");
          expectHighlight(cardId(vanilla), false, "a non-Taunt enemy unit is not a legal target");
          expectHighlight(heroId("opponent"), false, "the enemy hero is not a legal target while Taunt is up");
          expectHighlight(cardId(timmy), true, "the selected attacker stays clickable (M5-T2)");
        });
      });
    });
  });

  it("§4.1 / §4.4 step 2 — a Defense Position card is rotated, taunts, and its Armor 1 is in the numbers", () => {
    cy.seedGame({ seed: SEED, a: "04-combat-a", b: "04-combat-b" });

    ensureSeat("p1");
    cy.playByName("Mr. Vanilla", { zone: { side: "you", row: "units", lane: 1 } });
    advanceToTurn(2);
    cy.playByName("Mr. Vanilla", { zone: { side: "you", row: "units", lane: 1 } });

    advanceToTurn(3);
    cy.playByName("Tempo Timmy", { zone: { side: "you", row: "units", lane: 2 } });

    cy.instanceAt("p1", "units", 1).then((defender) => {
      cy.instanceAt("p1", "units", 2).then((timmy) => {
        cy.instanceAt("p2", "units", 1).then((attacker) => {
          cy.get(ts(cardId(defender))).should("have.attr", "data-position", "ATK");
          cy.switchPosition(defender);

          cy.get(ts(cardId(defender))).should("have.attr", "data-position", "DEF");
          cy.get(ts(cardId(defender))).should("have.attr", "style").and("contain", "rotate(90deg)");
          cy.get(ts(cardId(defender))).find(keywordIs("Taunt")).should("exist");
          cy.get(ts(cardId(defender))).find(armorIs(1)).should("exist");

          advanceToTurn(4);
          selectAttacker(attacker);
          expectHighlight(cardId(defender), true, "the Defense Position unit taunts (§4.1, §4.2 step 3)");
          expectHighlight(cardId(timmy), false, "player 1's other unit is not a legal target");
          expectHighlight(heroId("opponent"), false, "player 1's hero is not a legal target while Taunt is up");

          cy.get(ts(cardId(defender))).click();
          cy.get(ts(cardId(defender))).find(DAMAGE_POP).should("have.text", "3");
          cy.get(ts(cardId(attacker))).find(DAMAGE_POP).should("have.text", "4");
          cy.settled();

          cy.get(ts(cardId(defender))).find(healthIs(1)).should("exist");
          cy.get(ts(cardId(defender))).find(maxHealthIs(4)).should("exist");
          cy.get(ts(cardId(defender))).should("have.attr", "data-position", "DEF");
          cy.get(ts(cardId(attacker))).should("not.exist");
          cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
        });
      });
    });
  });

  it("§4.3 / §4.4 — First Strike, Divine Shield and Lifesteal produce the pipeline's numbers", () => {
    cy.seedGame({ seed: SEED, a: "04-combat-a", b: "04-combat-b" });

    ensureSeat("p1");
    cy.playByName("Tempo Timmy", { zone: { side: "you", row: "units", lane: 1 } });
    advanceToTurn(3);
    cy.playByName("Pointmaster", { zone: { side: "you", row: "units", lane: 2 } });
    advanceToTurn(4);
    cy.playByName("Jilliax", { zone: { side: "you", row: "units", lane: 1 } });

    advanceToTurn(5);
    cy.instanceAt("p1", "units", 1).then((timmy) => {
      cy.instanceAt("p1", "units", 2).then((pointmaster) => {
        cy.instanceAt("p2", "units", 1).then((jilliax) => {
          cy.get(ts(cardId(timmy))).find(keywordIs("First Strike")).should("exist");
          cy.get(ts(cardId(jilliax))).find(keywordIs("Divine Shield")).should("exist");
          cy.get(ts(heroId("opponent"))).find(healthIs(30)).should("exist");

          // Divine Shield negates the first hit, so Jilliax has no damage pop.
          selectAttacker(timmy);
          cy.get(ts(cardId(jilliax))).click();
          cy.expectAnimating("divineShieldLost");
          cy.get(ts(cardId(timmy))).find(DAMAGE_POP).should("have.text", "3");
          cy.get(ts(heroId("opponent"))).find(HEAL_POP).should("have.text", "3");
          cy.settled();

          cy.get(ts(cardId(jilliax))).find(healthIs(2)).should("exist");
          cy.get(ts(cardId(jilliax))).find(maxHealthIs(2)).should("exist");
          cy.get(ts(cardId(jilliax))).find(keywordIs("Divine Shield")).should("not.exist");
          cy.get(ts(cardId(timmy))).should("not.exist");
          cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
          cy.get(ts(heroId("opponent"))).find(healthIs(33)).should("exist");

          // The first-strike lethal hit prevents a return strike.
          selectAttacker(pointmaster);
          cy.get(ts(cardId(jilliax))).click();
          cy.get(ts(cardId(jilliax))).find(DAMAGE_POP).should("have.text", "7");
          cy.settled();

          cy.get(ts(cardId(jilliax))).should("not.exist");
          cy.get(ts(graveyardCountId("opponent"))).should("have.text", "1");
          cy.get(ts(cardId(pointmaster))).find(attackIs(7)).should("exist");
          cy.get(ts(cardId(pointmaster))).find(healthIs(1)).should("exist");
          cy.get(ts(cardId(pointmaster))).find(DAMAGE_POP).should("not.exist");
          cy.get(ts(heroId("opponent"))).find(healthIs(33)).should("exist");
        });
      });
    });
  });
});
