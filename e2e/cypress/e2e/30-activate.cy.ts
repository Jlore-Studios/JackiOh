// BUILD M9-T12 `30-activate.cy.ts`: C #81 Thrive and C #21 Turtinator activation (R384, R510).
// Read player 1's DOM only (CLAUDE.md rule 7). Thrive is once per turn; each Turtinator use
// Tributes another Unit (R683).

import { seedFor } from "../../support/config.ts";
import {
  PROMPT,
  activateId,
  activateUsesId,
  cardId,
  graveyardCountId,
  healthIs,
  heroId,
  manaId,
  promptOf,
  promptOptionId,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { PlayerId } from "../../support/types.ts";

const SEED = seedFor("30-activate-353");

const HERO_HEALTH = 30;
/** SPEC §8.6: #8 Mr. Vanilla's Attack. */
const VANILLA_ATTACK = 4;
const THRIVE_MODES = ["heal", "draw", "mana"] as const;
const OPPONENT_HERO = "hero:p2";

function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== player) cy.handOver();
  });
}

function manaIs(current: number): void {
  cy.get(ts(manaId("you"))).should("have.attr", "data-current", String(current));
}

describe("BUILD M9 30: Activate controls (R384, R510)", () => {
  it("R384: once per turn on The Power to Thrive, a Tribute each time on Turtinator's Activate ♾️", () => {
    cy.seedGame({ seed: SEED, a: "30-activate-a", b: "30-quiet-b" });

    cy.advanceToTurn(3);
    ensureSeat("p1");
    cy.playByName("The Power to Thrive", { zone: { side: "you", row: "backrow", lane: 2 } });
    manaIs(0);

    cy.instanceAt("p1", "backrow", 2).then((thrive) => {
      const control = ts(activateId(thrive));
      cy.get(ts(zoneId("you", "backrow", 2))).find(control).should("exist");
      cy.get(control).should("have.attr", "data-legal", "true");
      cy.get(ts(activateUsesId(thrive))).should("have.text", "1");

      // R81 and R384: Activate opens the mode picker.
      cy.get(control).click();
      cy.waitForPrompt("discover");
      for (const mode of THRIVE_MODES) cy.get(promptOf("discover")).find(ts(promptOptionId(mode))).should("exist");
      cy.get(promptOf("discover")).find(ts(promptOptionId("mana"))).click();
      cy.expectAnimating("activated");
      cy.settled();
      manaIs(1);

      cy.get(ts(activateUsesId(thrive))).should("have.text", "0");
      cy.get(control).should("have.attr", "data-legal", "false");
      cy.get(control).invoke("attr", "title").should("contain", "—");
      cy.get(control).click();
      cy.get(PROMPT).should("not.exist");
      manaIs(1);

      cy.playByName("Mr. Vanilla", { zone: { side: "you", row: "units", lane: 1 } });
      manaIs(0);

      cy.advanceToTurn(5);
      ensureSeat("p1");
      cy.get(control).should("have.attr", "data-legal", "true");
      cy.get(ts(activateUsesId(thrive))).should("have.text", "1");
    });

    cy.playByName("Turtinator", { zone: { side: "you", row: "units", lane: 2 } });
    cy.instanceAt("p1", "units", 1).then((vanilla) => {
      cy.instanceAt("p1", "units", 2).then((turtinator) => {
        const control = ts(activateId(turtinator));
        cy.get(ts(cardId(turtinator))).find(control).should("exist");
        cy.get(control).should("have.attr", "data-legal", "true");
        cy.get(ts(activateUsesId(turtinator))).should("have.text", "∞");

        // R683: Turtinator cannot Tribute itself.
        cy.get(control).click();
        cy.get(promptOf("tribute")).should("not.exist");
        cy.waitForPrompt("target");
        cy.get(promptOf("target")).find(ts(promptOptionId(OPPONENT_HERO))).click();
        cy.expectAnimating("activated");
        cy.settled();
        cy.get(ts(cardId(vanilla))).should("not.exist");
        cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
        cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH - VANILLA_ATTACK)).should("exist");

        // No other Unit remains to pay the Tribute.
        cy.get(ts(activateUsesId(turtinator))).should("have.text", "∞");
        cy.get(control).should("have.attr", "data-legal", "false");
        cy.get(control).invoke("attr", "title").should("contain", "Tribute");
        cy.get(control).click();
        cy.get(PROMPT).should("not.exist");
        cy.get(ts(cardId(turtinator))).should("exist");
        cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
        cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH - VANILLA_ATTACK)).should("exist");
      });
    });
    cy.replayCheck("30-activate");
  });
});
