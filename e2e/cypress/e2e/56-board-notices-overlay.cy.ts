// Spec 56 — board notices draw in an overlay over the board, not in the control rail's flow (issue #3868).
// Showing, stacking or clearing notices alters zero board dimensions, leaving the field, seats,
// hands and End turn at fixed positions and sizes across desktop, phone portrait and phone landscape.

import { seedFor } from "../../support/config.ts";
import {
  ACTION_ERROR,
  DRAW_ACCEPT,
  DRAW_DECLINE,
  DRAW_OFFER,
  DRAW_TOAST,
  END_TURN,
  RESULT_OVERLAY,
  ts,
  zoneId,
} from "../../support/testids.ts";
import type { PlayerId } from "../../support/types.ts";

const SEED = seedFor("56-board-notices");
const ATTACK_REASON = "attack-reason";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 800 },
  { label: "phone-portrait", width: 390, height: 844 },
  { label: "phone-landscape", width: 844, height: 390 },
] as const;

type Rect = {
  top: number;
  left: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
};

type BoardBoxes = {
  field: Rect;
  seatOpponent: Rect;
  seatYou: Rect;
  handOpponent: Rect;
  handYou: Rect;
  endTurn: Rect;
};

function toRect(domRect: DOMRect): Rect {
  return {
    top: domRect.top,
    left: domRect.left,
    right: domRect.right,
    bottom: domRect.bottom,
    width: domRect.width,
    height: domRect.height,
  };
}

function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== player) cy.handOver();
  });
}

function measureBoxes(): Cypress.Chainable<BoardBoxes> {
  return cy.document().then((doc) => {
    const field = doc.querySelector<HTMLElement>(".field");
    const seatOpponent = doc.querySelector<HTMLElement>(".seat-opponent");
    const seatYou = doc.querySelector<HTMLElement>(".seat-you");
    const handOpponent = doc.querySelector<HTMLElement>(".hand-opponent");
    const handYou = doc.querySelector<HTMLElement>(".hand-you");
    const endTurn = doc.querySelector<HTMLElement>(`[data-testid="${END_TURN}"]`);

    expect(field, "field element exists").to.not.eq(null);
    expect(seatOpponent, "seatOpponent element exists").to.not.eq(null);
    expect(seatYou, "seatYou element exists").to.not.eq(null);
    expect(handOpponent, "handOpponent element exists").to.not.eq(null);
    expect(handYou, "handYou element exists").to.not.eq(null);
    expect(endTurn, "endTurn element exists").to.not.eq(null);

    return {
      field: toRect(field!.getBoundingClientRect()),
      seatOpponent: toRect(seatOpponent!.getBoundingClientRect()),
      seatYou: toRect(seatYou!.getBoundingClientRect()),
      handOpponent: toRect(handOpponent!.getBoundingClientRect()),
      handYou: toRect(handYou!.getBoundingClientRect()),
      endTurn: toRect(endTurn!.getBoundingClientRect()),
    };
  });
}

function expectBoxesUnchanged(current: BoardBoxes, baseline: BoardBoxes, context: string): void {
  const keys: (keyof BoardBoxes)[] = [
    "field",
    "seatOpponent",
    "seatYou",
    "handOpponent",
    "handYou",
    "endTurn",
  ];
  for (const key of keys) {
    const cur = current[key];
    const base = baseline[key];
    for (const prop of ["top", "left", "width", "height"] as const) {
      expect(
        Math.abs(cur[prop] - base[prop]),
        `${key}.${prop} shifted by > 1px (${context}): baseline=${base[prop]}, current=${cur[prop]}`,
      ).to.be.at.most(1);
    }
  }
}

function expectNoticeValid(
  selector: string,
  viewport: { width: number; height: number },
  boxes: BoardBoxes,
): Cypress.Chainable<Rect> {
  return cy.get(selector).should("be.visible").then(($el) => {
    const el = $el[0];
    if (el === undefined) throw new Error(`${selector} not found`);
    const rect = toRect(el.getBoundingClientRect());

    // 1. Not clipped by the viewport
    expect(rect.top, `${selector} top should be within viewport`).to.be.at.least(0);
    expect(rect.left, `${selector} left should be within viewport`).to.be.at.least(0);
    expect(rect.bottom, `${selector} bottom should be within viewport`).to.be.at.most(viewport.height);
    expect(rect.right, `${selector} right should be within viewport`).to.be.at.most(viewport.width);

    // 2. Painted above the field (anchored within field bounds)
    expect(rect.top, `${selector} top must be >= field top`).to.be.at.least(boxes.field.top - 1);
    expect(rect.bottom, `${selector} bottom must be <= field bottom`).to.be.at.most(boxes.field.bottom + 1);

    // 3. Does not overlap End turn
    const overlapsEndTurn = !(
      rect.right <= boxes.endTurn.left ||
      rect.left >= boxes.endTurn.right ||
      rect.bottom <= boxes.endTurn.top ||
      rect.top >= boxes.endTurn.bottom
    );
    expect(overlapsEndTurn, `${selector} must not overlap End turn`).to.eq(false);

    // 4. Does not overlap the viewer's hand
    const overlapsHandYou = !(
      rect.right <= boxes.handYou.left ||
      rect.left >= boxes.handYou.right ||
      rect.bottom <= boxes.handYou.top ||
      rect.top >= boxes.handYou.bottom
    );
    expect(overlapsHandYou, `${selector} must not overlap viewer's hand`).to.eq(false);

    return rect;
  });
}

describe("56 — board notices draw in an overlay over the board (#3868)", () => {
  for (const vp of VIEWPORTS) {
    describe(`viewport: ${vp.label} (${vp.width}x${vp.height})`, () => {
      beforeEach(() => {
        cy.viewport(vp.width, vp.height);
      });

      it("notices take no board space: attack-reason, action-error, and draw offer leave boxes unchanged", () => {
        cy.seedGame({ seed: SEED, a: "32-tribute-a", b: "30-quiet-b" });
        ensureSeat("p1");

        // Play a 1-cost unit into lane 3 (the center lane where notices overlay).
        cy.playByName("Right-house defender", { zone: { side: "you", row: "units", lane: 3 } });
        cy.settled();

        // Record baseline bounding boxes for all 6 board elements.
        measureBoxes().then((baseline) => {
          // --- Test 1: attack-reason ---
          // Tap the asleep unit: it cannot attack, so attack-reason appears.
          const cardSelector = `${ts(zoneId("you", "units", 3))} .card`;
          cy.get(cardSelector).click();

          const attackReasonSelector = `[data-testid="${ATTACK_REASON}"]`;
          cy.get(attackReasonSelector).should("be.visible").and("contain.text", "asleep");

          // Check boxes while attack-reason is visible.
          measureBoxes().then((boxesWhileNotice) => {
            expectBoxesUnchanged(boxesWhileNotice, baseline, "while attack-reason is visible");
          });

          // Check notice bounds and lack of overlap with End turn or hand.
          expectNoticeValid(attackReasonSelector, vp, baseline);

          // Click pass-through: with attack-reason showing, clicking the card under it reaches the card.
          cy.get(cardSelector).click();

          // Click empty zone to clear notice.
          cy.get(ts(zoneId("you", "units", 1))).click();
          cy.get(attackReasonSelector).should("not.exist");

          // Check boxes after attack-reason clears.
          measureBoxes().then((boxesAfterNoticeClear) => {
            expectBoxesUnchanged(boxesAfterNoticeClear, baseline, "after attack-reason clears");
          });

          // --- Test 2: action-error ---
          // Trigger an action error via an illegal action through dev dispatch.
          cy.jackioh().then((handle) => {
            handle.dispatch({ type: "play", playerId: "p1", instanceId: "nonexistent-card", zone: { row: "units", lane: 1 } });
          });

          const actionErrorSelector = `[data-testid="${ACTION_ERROR}"]`;
          cy.get(actionErrorSelector).should("be.visible");

          // Check boxes while action-error is visible.
          measureBoxes().then((boxesWhileError) => {
            expectBoxesUnchanged(boxesWhileError, baseline, "while action-error is visible");
          });

          expectNoticeValid(actionErrorSelector, vp, baseline);

          // Clear the action error by re-affirming the seat.
          cy.jackioh().then((handle) => {
            handle.setSeat?.("p1");
          });
          cy.get(actionErrorSelector).should("not.exist");

          // Check boxes after action-error clears.
          measureBoxes().then((boxesAfterErrorClear) => {
            expectBoxesUnchanged(boxesAfterErrorClear, baseline, "after action-error clears");
          });

          // --- Test 3: draw-offer ---
          // p1 offers a draw, and device hands over to p2 to answer it.
          cy.jackioh().then((handle) => {
            handle.dispatch({ type: "offerDraw", playerId: "p1" });
            handle.setSeat?.("p2");
          });

          const drawToastSelector = `[data-testid="${DRAW_TOAST}"]`;
          const drawOfferSelector = `[data-testid="${DRAW_OFFER}"]`;
          cy.get(drawToastSelector).should("be.visible");
          cy.get(drawOfferSelector).should("be.visible");

          // Check boxes while draw offer is visible.
          measureBoxes().then((boxesWhileDraw) => {
            expectBoxesUnchanged(boxesWhileDraw, baseline, "while draw offer is visible");
          });

          expectNoticeValid(drawToastSelector, vp, baseline);

          // Accept and Decline buttons are active; click Decline.
          cy.get(`[data-testid="${DRAW_ACCEPT}"]`).should("be.visible");
          cy.get(`[data-testid="${DRAW_DECLINE}"]`).should("be.visible").click();

          // Offer dismissed.
          cy.get(drawOfferSelector).should("not.exist");

          // Check boxes after draw offer is declined.
          measureBoxes().then((boxesAfterDecline) => {
            expectBoxesUnchanged(boxesAfterDecline, baseline, "after draw offer is declined");
          });

          // P2 offers draw back to P1, and P1 accepts.
          cy.jackioh().then((handle) => {
            handle.dispatch({ type: "offerDraw", playerId: "p2" });
            handle.setSeat?.("p1");
          });

          cy.get(drawOfferSelector).should("be.visible");
          cy.get(`[data-testid="${DRAW_ACCEPT}"]`).should("be.visible").click();
          cy.get(ts(RESULT_OVERLAY)).should("be.visible");
        });
      });
    });
  }
});
