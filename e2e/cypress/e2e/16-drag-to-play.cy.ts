// B40 drag-to-play on a seeded hotseat game (docs/polish/7-mobile-ux.md).
// `support/ux.ts` supplies pointer gestures and new attributes; support/testids.ts supplies the rest.
// The seed puts #11 Tempo Timmy in player 1's opening hand; the decks make no autonomous board changes.
// §8 #11 Tempo Timmy (3/3, Rush, First Strike) attacks heroes on player-turn 3 (§6.1). From
// HERO_HEALTH (30, §2), one hit leaves 27. B39 preserves click-click with dragging enabled.

import { constants, seedFor } from "../../support/config.ts";
import { BOARD, attackIs, cardId, handCardId, healthIs, heroId, ts, zoneId } from "../../support/testids.ts";
import type { PlayerId } from "../../support/types.ts";
import {
  DRAGGING_ATTR,
  DRAG_ARROW,
  DRAG_ATTR,
  DRAG_FROM_ATTR,
  DRAG_GHOST,
  DRAG_INSTANCE_ATTR,
  DRAG_KIND_ATTR,
  DRAG_LAYER,
  DRAG_RETICLE,
  DRAG_TARGET_ATTR,
  DRAG_VALID_ATTR,
  GLOW_ATTR,
  GLOW_READY,
  ROOT,
  SETTINGS_CLOSE,
  SETTINGS_OPEN_GAME,
  SETTINGS_PANEL,
  dragTo,
  hoverOver,
  pressAndLift,
  releaseOver,
  settingId,
} from "../../support/ux.ts";

const SEED = seedFor("04-combat-1604");
const DECK_A = "04-combat-a";
const DECK_B = "04-combat-b";

/** §8 #11 is in player 1's opening hand under this seed. */
const TIMMY = "Tempo Timmy";
const TIMMY_ATTACK = 3;

const LANE_1 = zoneId("you", "units", 1);
const LANE_1_REF = { side: "you", row: "units", lane: 1 } as const;

const OPPONENT_HERO = heroId("opponent");

/** BUILD M5-T3: hand the device to the acting seat. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat (the hotseat handle names the seat holding it)").to.not.eq(
      undefined,
    );
    if (handle.seat !== player) cy.handOver();
  });
}

function startGame(): void {
  cy.seedGame({ seed: SEED, a: DECK_A, b: DECK_B });
  ensureSeat("p1");
}

function expectNoDrag(): void {
  cy.get(ts(DRAG_LAYER)).should("not.exist");
  cy.get(ROOT).should("not.have.attr", DRAGGING_ATTR);
}

describe("Polish 7 B40 — drag to play in a seeded hotseat game", () => {
  it("B40 dragging a glowing hand card onto a glowing zone plays it", () => {
    startGame();

    cy.handCardByName(TIMMY).then((timmy) => {
      const source = ts(handCardId(timmy));
      cy.get(source).should("have.attr", GLOW_ATTR, GLOW_READY);

      pressAndLift(source);
      cy.get(ts(DRAG_LAYER)).should("have.attr", DRAG_KIND_ATTR, "play");
      cy.get(ts(DRAG_GHOST)).should("have.attr", DRAG_INSTANCE_ATTR, timmy);
      cy.get(ROOT).should("have.attr", DRAGGING_ATTR, "play");
      cy.get(ts(LANE_1)).should("have.attr", GLOW_ATTR, GLOW_READY);

      hoverOver(ts(LANE_1));
      cy.get(ts(DRAG_RETICLE)).should("have.attr", DRAG_TARGET_ATTR, LANE_1);

      releaseOver(ts(LANE_1));
      cy.settled();

      expectNoDrag();
      cy.get(source).should("not.exist");
      cy.get(ts(LANE_1)).should("contain.text", TIMMY);
      cy.instanceAt("p1", "units", 1).then((placed) => {
        cy.get(ts(LANE_1)).find(ts(cardId(placed))).should("exist");
      });
    });
  });

  it("B40 dragging a unit onto the enemy hero attacks", () => {
    startGame();
    cy.playByName(TIMMY, { zone: LANE_1_REF });
    cy.advanceToTurn(3);
    ensureSeat("p1");

    cy.instanceAt("p1", "units", 1).then((timmy) => {
      const source = ts(cardId(timmy));
      const hero = ts(OPPONENT_HERO);
      cy.get(source).should("have.attr", GLOW_ATTR, GLOW_READY).find(attackIs(TIMMY_ATTACK)).should("exist");
      cy.get(hero).find(healthIs(constants.HERO_HEALTH)).should("exist");

      pressAndLift(source);
      cy.get(ts(DRAG_LAYER)).should("have.attr", DRAG_KIND_ATTR, "attack");
      cy.get(ts(DRAG_ARROW)).should("have.attr", DRAG_FROM_ATTR, cardId(timmy));
      cy.get(ROOT).should("have.attr", DRAGGING_ATTR, "attack");
      cy.get(hero).should("have.attr", GLOW_ATTR, GLOW_READY);

      hoverOver(hero);
      cy.get(ts(DRAG_RETICLE)).should("have.attr", DRAG_TARGET_ATTR, OPPONENT_HERO);
      cy.get(ts(DRAG_ARROW)).should("have.attr", DRAG_VALID_ATTR, "true");

      releaseOver(hero);
      cy.settled();

      expectNoDrag();
      cy.get(hero).find(healthIs(constants.HERO_HEALTH - TIMMY_ATTACK)).should("exist");
    });
  });

  it("B40 releasing a lifted card outside the board cancels it", () => {
    startGame();

    cy.handCardByName(TIMMY).then((timmy) => {
      const source = ts(handCardId(timmy));

      pressAndLift(source);
      cy.get(ts(DRAG_LAYER)).should("exist");
      cy.get(ts(LANE_1)).should("have.attr", GLOW_ATTR, GLOW_READY);

      hoverOver("outside");
      cy.get(ts(DRAG_RETICLE)).should("not.exist");

      releaseOver("outside");
      cy.settled();

      expectNoDrag();
      cy.get(source).should("exist").and("not.have.attr", "data-selected");
      cy.get(ts(LANE_1)).should("not.have.attr", GLOW_ATTR);
      cy.unitIds("p1").should("have.length", 0);
    });
  });

  it("B40 with drag to play turned off in the settings panel, still off after a reload, click-click plays a card", () => {
    startGame();
    cy.get(ts(BOARD)).should("have.attr", DRAG_ATTR, "on");

    cy.get(ts(SETTINGS_OPEN_GAME)).click();
    cy.get(ts(SETTINGS_PANEL)).should("be.visible");
    cy.get(ts(settingId("dragToPlay"))).should("be.checked").uncheck({ force: true });
    cy.get(ts(settingId("dragToPlay"))).should("not.be.checked");
    cy.get(ts(SETTINGS_CLOSE)).click();
    cy.get(ts(SETTINGS_PANEL)).should("not.exist");
    cy.get(ts(BOARD)).should("have.attr", DRAG_ATTR, "off");

    // Reload restores the same seed, decks, and setting.
    cy.reload();
    cy.jackioh().should((handle) => {
      expect(handle.seed, "the same seed after the reload").to.eq(SEED);
    });
    cy.settled();
    cy.keepMulligans();
    ensureSeat("p1");

    cy.get(ts(BOARD)).should("have.attr", DRAG_ATTR, "off");
    cy.get(ts(SETTINGS_OPEN_GAME)).click();
    cy.get(ts(settingId("dragToPlay"))).should("not.be.checked");
    cy.get(ts(SETTINGS_CLOSE)).click();
    cy.get(ts(SETTINGS_PANEL)).should("not.exist");

    cy.handCardByName(TIMMY).then((timmy) => {
      const source = ts(handCardId(timmy));

      pressAndLift(source);
      expectNoDrag();
      hoverOver(ts(LANE_1));
      // Asked again after more travel: an overlay that rendered late would be caught here.
      expectNoDrag();
      cy.get(ts(LANE_1)).should("not.have.attr", GLOW_ATTR);
      cy.get(ts(DRAG_RETICLE)).should("not.exist");
      releaseOver(ts(LANE_1));
      cy.settled();
      expectNoDrag();
      cy.get(source).should("exist");
      cy.unitIds("p1").should("have.length", 0);

      cy.playCard(timmy, { zone: LANE_1_REF });
      cy.get(source).should("not.exist");
      cy.get(ts(LANE_1)).should("contain.text", TIMMY);
    });
  });

  it("B40 the whole drag in one call, dragTo, plays a card and a drag cancelled outside leaves it in hand", () => {
    startGame();

    cy.handCardByName(TIMMY).then((timmy) => {
      const source = ts(handCardId(timmy));

      dragTo(source, "outside");
      expectNoDrag();
      cy.get(source).should("exist");
      cy.unitIds("p1").should("have.length", 0);

      dragTo(source, ts(LANE_1));
      expectNoDrag();
      cy.get(source).should("not.exist");
      cy.get(ts(LANE_1)).should("contain.text", TIMMY);
    });
  });
});
