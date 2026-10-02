// BUILD M9-T12 `30-activate.cy.ts`: C #81 The Power to Thrive and C #21 Turtinator on the field
// (R384, R510; issue #66).
//
// What it asserts, read off the DOM player 1's view drew (CLAUDE.md rule 7):
//
//   1. The Power to Thrive, a face-up Field Spell, wears one Activate control (`activate-<id>`). It is
//      live (`data-legal="true"`) and its badge reads 1 use left.
//   2. Pressing it opens the mode picker with the card's three modes. Choosing "mana" resolves:
//      `activated` plays and the tray goes from 0 to 1, which pays for #8 Mr. Vanilla straight away.
//   3. "Activate" is once per turn. Afterwards the badge reads 0 and the control is greyed
//      (`data-legal="false"`, with the engine's reason in its tooltip). Pressing it sends nothing:
//      no picker opens and the mana stays where it was.
//   4. Next turn the control is live again with 1 use.
//   5. Turtinator ("Activate ♾️: Tribute a Unit. Deal damage equal to its Attack…") shows "∞" and is
//      live while summoning sick: activating is not attacking. Each use pays a Tribute. Mr. Vanilla
//      (4 Attack) pays the first, so player 2's hero goes 30 to 26. Turtinator pays the second
//      itself (5 Attack), so 26 to 21. Then no Unit is left to pay, and the control is gone with
//      its card.
//
// Decks: `30-activate-a` is spec 03's player-1 deck with Thrive and Turtinator swapped in;
// `30-quiet-b` is a do-nothing deck. Seed 30-activate-353 was chosen by replaying these moves
// against the engine. It has Thrive and Mr. Vanilla in player 1's hand on player-turn 3, and
// Turtinator there on player-turn 5.

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
/** SPEC §8.6: #8 Mr. Vanilla's and C #21 Turtinator's Attack, each times the base multiplier 1. */
const VANILLA_ATTACK = 4;
const TURTINATOR_ATTACK = 5;
/** C #81's three modes, as its script names them and the picker keys them. */
const THRIVE_MODES = ["heal", "draw", "mana"] as const;
/** The target picker's key for player 2's hero (`selectionKey` in apps/web/src/game/actions.ts). */
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

    // Player-turn 3 is player 1's second turn: two crystals, Thrive's cost.
    cy.advanceToTurn(3);
    ensureSeat("p1");
    cy.playByName("The Power to Thrive", { zone: { side: "you", row: "backrow", lane: 2 } });
    manaIs(0);

    cy.instanceAt("p1", "backrow", 2).then((thrive) => {
      const control = ts(activateId(thrive));
      cy.get(ts(zoneId("you", "backrow", 2))).find(control).should("exist");
      cy.get(control).should("have.attr", "data-legal", "true");
      cy.get(ts(activateUsesId(thrive))).should("have.text", "1");

      // The mode is chosen on the control, in the picker it opens (R81, R384).
      cy.get(control).click();
      cy.waitForPrompt("mode");
      for (const mode of THRIVE_MODES) cy.get(promptOf("mode")).find(ts(promptOptionId(mode))).should("exist");
      cy.get(promptOf("mode")).find(ts(promptOptionId("mana"))).click();
      cy.expectAnimating("activated");
      cy.settled();
      manaIs(1);

      // Once per turn: spent, greyed, and saying why.
      cy.get(ts(activateUsesId(thrive))).should("have.text", "0");
      cy.get(control).should("have.attr", "data-legal", "false");
      cy.get(control).invoke("attr", "title").should("contain", "—");
      cy.get(control).click();
      cy.get(PROMPT).should("not.exist");
      manaIs(1);

      // The mana the mode gave pays for a play.
      cy.playByName("Mr. Vanilla", { zone: { side: "you", row: "units", lane: 1 } });
      manaIs(0);

      // Live again on player 1's next turn.
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

        // First use: Mr. Vanilla pays the Tribute, and its Attack hits player 2's hero.
        cy.get(control).click();
        cy.answerPrompt("tribute", { cards: [vanilla] });
        cy.waitForPrompt("target");
        cy.get(promptOf("target")).find(ts(promptOptionId(OPPONENT_HERO))).click();
        cy.expectAnimating("activated");
        cy.settled();
        cy.get(ts(cardId(vanilla))).should("not.exist");
        cy.get(ts(graveyardCountId("you"))).should("have.text", "1");
        cy.get(ts(heroId("opponent"))).find(healthIs(HERO_HEALTH - VANILLA_ATTACK)).should("exist");

        // ♾️: still live after a use, and the only Unit left to pay with is Turtinator itself.
        cy.get(control).should("have.attr", "data-legal", "true");
        cy.get(ts(activateUsesId(turtinator))).should("have.text", "∞");
        cy.get(control).click();
        cy.waitForPrompt("target");
        cy.get(promptOf("target")).find(ts(promptOptionId(OPPONENT_HERO))).click();
        cy.settled();
        cy.get(ts(cardId(turtinator))).should("not.exist");
        cy.get(control).should("not.exist");
        cy.get(ts(graveyardCountId("you"))).should("have.text", "2");
        cy.get(ts(heroId("opponent")))
          .find(healthIs(HERO_HEALTH - VANILLA_ATTACK - TURTINATOR_ATTACK))
          .should("exist");
      });
    });
    cy.replayCheck("30-activate");
  });
});
