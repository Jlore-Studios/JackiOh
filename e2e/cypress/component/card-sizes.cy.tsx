// #3341: bigger, more vivid cards, measured in a browser on a full board (5 Units and 5 backrow cards a side) and a 10-card
// hand at the five sizes a player holds the game at: the one-screen rule (R1581000) and the tap areas (R2122000) still hold,
// the board's and the hand's cards are larger than the budget they had (game/config.ts's BOARD_CARD_BUDGET, held to
// board.css by game/cardSizes.test.ts), a desktop's hover preview shows a unit's art over twice as wide as its face on the
// board, and a phone's inspect sheet draws a taller card. Specs 46 and 47 (e2e/cypress/e2e) mount no full board.

import Game from "../../../apps/web/src/game/Game.tsx";
import "../../../apps/web/src/practice/practice.css";
import { FACE_ASPECT } from "../../../apps/web/src/cards/constants.ts";
import { INSPECT_FACE, INSPECT_HOVER, INSPECT_SHEET } from "../../../apps/web/src/cards/inspect/testids.ts";
import { HOVER_DELAY_MS, LONG_PRESS_MS, PREVIEW_SHEET_HEIGHT_PX, PREVIEW_SHEET_SHARE } from "../../../apps/web/src/cards/inspect/constants.ts";
import { previewHeight } from "../../../apps/web/src/cards/inspect/placement.ts";
import { BOARD_CARD_BUDGET } from "../../../apps/web/src/game/config.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";
import { card, fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

type View = Parameters<typeof Game>[0]["view"];

/** `--hit-min` (R2122000): every control and zone a finger has to hit is this many px across, at least. */
const HIT_MIN_PX = 44;
/** Subpixel slack for edges the browser rounds. */
const SLACK_PX = 1;
/** A zone's inside is its card's height less its 1px sides and the 2px line (board.css `.lane`). */
const ZONE_EDGE_PX = 3;
/** How much taller a board card must be than the budget it had: more than rounding. */
const LARGER_PX = 1.5;
/** A preview's art against a board unit's whole face. */
const PREVIEW_ART_TIMES = 2;
/** A phone's sheet card before #3341: `min(380px, 56dvh)`. */
const SHEET_BEFORE = { px: 380, share: 0.56 } as const;

const FAN_DEFS = ["core-002", "core-005", "core-008", "core-011", "core-019", "core-020", "core-043", "core-053", "core-055", "core-077"];

/** A full board, and `n` cards in your hand. */
function fanView(n: number): View {
  const full = fullBoardView();
  const hand = Array.from({ length: n }, (_unused, index) =>
    card({ instanceId: `fan${index + 1}`, defId: FAN_DEFS[index % FAN_DEFS.length] ?? "core-001", cost: index % 7 }),
  );
  return { ...full, you: { ...full.you, hand }, opponent: { ...full.opponent, hand: { count: n } } };
}

type Before = { minPx: number; fixedPx: number; rows: number; maxPx: number; widthFixedPx?: number; widthRows?: number };

type Size = {
  label: string;
  width: number;
  height: number;
  layout: keyof typeof BOARD_CARD_BUDGET;
  /** The hand card's height as a multiple of `--card-h` in the layout. */
  handRatio: number;
  /** The layout's `--card-h` budget before #3341. */
  before: Before;
  /** Whether the layout sizes its controls to a finger (board.css's `max-width: 1024px` touch block). */
  touch: boolean;
};

const BEFORE_DESKTOP: Before = { minPx: 56, fixedPx: 215, rows: 5.48, maxPx: 112, widthFixedPx: 334, widthRows: 10.66 };
const BEFORE_PHONE: Before = { minPx: 44, fixedPx: 348, rows: 5.24, maxPx: 90 };
const BEFORE_LANDSCAPE: Before = { minPx: 40, fixedPx: 40, rows: 4.73, maxPx: 62 };

const SIZES: readonly Size[] = [
  { label: "desktop 1280x720", width: 1280, height: 720, layout: "desktop", handRatio: 1.48, before: BEFORE_DESKTOP, touch: false },
  { label: "desktop 1920x1080", width: 1920, height: 1080, layout: "desktop", handRatio: 1.48, before: BEFORE_DESKTOP, touch: false },
  { label: "phone 390x844", width: 390, height: 844, layout: "phone", handRatio: 1.24, before: BEFORE_PHONE, touch: true },
  { label: "phone 360x780", width: 360, height: 780, layout: "phone", handRatio: 1.24, before: BEFORE_PHONE, touch: true },
  { label: "phone on its side 844x390", width: 844, height: 390, layout: "landscape", handRatio: 1.4, before: BEFORE_LANDSCAPE, touch: true },
];

/** The card height a budget gives a board's content box: `clamp(min, min(height term, width term), max)`. */
function cardHeight(budget: Before, box: { w: number; h: number }): number {
  const byHeight = (box.h - budget.fixedPx) / budget.rows;
  const byWidth = budget.widthFixedPx === undefined || budget.widthRows === undefined ? Infinity : (box.w - budget.widthFixedPx) / budget.widthRows;
  return Math.min(budget.maxPx, Math.max(budget.minPx, Math.min(byHeight, byWidth)));
}

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

function mountGame(view: View): void {
  cy.mount(
    <div className="app-shell app-shell--wide">
      <Game view={view} legal={[]} onAction={() => undefined} />
    </div>,
  );
}

/** The board's content box, which is what `cqh` and `cqw` measure in its layouts. */
function contentBox(board: HTMLElement): { w: number; h: number } {
  const style = getComputedStyle(board);
  return {
    w: board.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight),
    h: board.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom),
  };
}

describe("#3341 bigger cards on a full board and a 10-card hand", () => {
  for (const size of SIZES) {
    it(`#3341 at ${size.label} the screen holds, every target is 44px, and the board's and the hand's cards are larger than they were`, () => {
      cy.viewport(size.width, size.height);
      mountGame(fanView(10));
      cy.get(ts("board")).should("be.visible");
      cy.get(`${ts("hand-you")} [data-testid^="hand-card-"]`).should("have.length", 10);

      cy.document().should((doc) => {
        const board = doc.querySelector<HTMLElement>(ts("board")) as HTMLElement;

        // One screen (R1581000): nothing scrolls, End turn is on it.
        expect(doc.documentElement.scrollHeight, "nothing scrolls vertically").to.be.at.most(size.height);
        expect(doc.documentElement.scrollWidth, "nothing scrolls sideways").to.be.at.most(size.width);
        const endTurn = (doc.querySelector(ts("end-turn")) as Element).getBoundingClientRect();
        expect(endTurn.bottom, "End turn is on the screen").to.be.at.most(size.height + SLACK_PX);

        // Tap areas (R2122000): every zone, and on a touch layout every control.
        const zones = [...doc.querySelectorAll(`${ts("board")} .zone`)];
        expect(zones, "every zone of a full board").to.have.length(20);
        for (const zone of zones) {
          const box = zone.getBoundingClientRect();
          expect(Math.min(box.width, box.height), `a zone is a ${HIT_MIN_PX}px target`).to.be.at.least(HIT_MIN_PX - SLACK_PX / 2);
        }
        if (size.touch) {
          for (const control of doc.querySelectorAll<HTMLElement>(`${ts("board")} .control`)) {
            const box = control.getBoundingClientRect();
            // A control folded into a menu has no box until the menu opens.
            if (getComputedStyle(control).display === "none" || box.width === 0 || box.height === 0) continue;
            expect(Math.min(box.width, box.height), `${control.textContent ?? "a control"} is a ${HIT_MIN_PX}px target`).to.be.at.least(HIT_MIN_PX - SLACK_PX / 2);
          }
        }

        // The cards: the budget's card height at this board, and the hand's and the field's at least that, and more than before.
        const box = contentBox(board);
        const now = cardHeight(BOARD_CARD_BUDGET[size.layout], box);
        const was = cardHeight(size.before, box);
        const hand = [...doc.querySelectorAll<HTMLElement>(`${ts("hand-you")} [data-testid^="hand-card-"]`)];
        for (const handCard of hand) {
          expect(handCard.offsetHeight, "a hand card is the budget's card times the hand's ratio").to.be.closeTo(now * size.handRatio, SLACK_PX + 0.5);
          expect(handCard.offsetHeight, "a hand card is larger than it was").to.be.at.least(was * size.handRatio + LARGER_PX);
        }
        const field = [...doc.querySelectorAll<HTMLElement>(`${ts("board")} .field .zone > .card`)];
        expect(field.length, "the field's cards").to.be.greaterThan(10);
        const shortest = Math.min(...field.map((fieldCard) => fieldCard.offsetHeight));
        expect(shortest, "a field card is at least the budget's card inside its zone").to.be.at.least(now - ZONE_EDGE_PX - SLACK_PX);
        expect(shortest, "a field card is larger than it was").to.be.at.least(was - ZONE_EDGE_PX + LARGER_PX);
      });
    });
  }
});

describe("#3341 a desktop's hover preview shows the art over twice as wide as the board's", () => {
  for (const size of SIZES.filter((entry) => !entry.touch)) {
    it(`#3341 at ${size.label} a unit's preview art is at least ${String(PREVIEW_ART_TIMES)} times its face's on the board`, () => {
      cy.viewport(size.width, size.height);
      mountGame(fanView(10));
      // Your fifth Unit prints no full art, so its board face is the art up to the frame.
      const unit = `${ts("zone-you-units-5")} > .card`;
      cy.get(unit).should("be.visible");
      cy.get(unit).trigger("pointerover", { eventConstructor: "PointerEvent", pointerType: "mouse", pointerId: 1, isPrimary: true, force: true });
      cy.get(ts(INSPECT_HOVER), { timeout: HOVER_DELAY_MS + 4000 }).should("be.visible");
      cy.get(ts(INSPECT_HOVER)).should(($preview) => {
        const doc = $preview[0]?.ownerDocument as Document;
        const face = doc.querySelector<HTMLElement>(`${ts(INSPECT_FACE)}`) as HTMLElement;
        expect(face.offsetHeight, "the preview's card").to.be.closeTo(previewHeight(size.height), SLACK_PX);
        // A Unit's art is 74% of the face's width in the preview (cards.css, 13cqw each side), and the face is 5:7.
        const preview = face.querySelector<HTMLElement>(".cf-art-frame > .cf-art") as HTMLElement;
        const board = doc.querySelector<HTMLElement>(`${unit} .cf-portrait > .cf-art`) as HTMLElement;
        expect(board.offsetWidth, "the board's art").to.be.greaterThan(0);
        expect(preview.offsetWidth, "the preview's art is most of its card's width").to.be.at.least(face.offsetHeight * FACE_ASPECT * 0.7);
        expect(preview.offsetWidth / board.offsetWidth, "the preview's art against the board's").to.be.at.least(PREVIEW_ART_TIMES);
      });
    });
  }
});

describe("#3341 a phone's inspect sheet draws a taller card", () => {
  afterEach(() => {
    __resetSettingsForTests();
  });

  for (const size of SIZES.filter((entry) => entry.layout === "phone")) {
    it(`#3341 at ${size.label} a long press on a unit opens a sheet whose card is taller than the 380px it was`, () => {
      cy.viewport(size.width, size.height);
      // With hover previews off, a hold opens the sheet (R2617000), as a touch screen without them reads a card.
      cy.then(() => {
        writeSettings({ hoverPreviews: false });
      });
      mountGame(fanView(10));
      const unit = `${ts("zone-you-units-5")} > .card`;
      cy.get(unit).should("be.visible");
      cy.get(unit).then(($unit) => {
        const box = ($unit[0] as HTMLElement).getBoundingClientRect();
        const at = { clientX: box.left + box.width / 2, clientY: box.top + box.height / 2 };
        cy.get(unit).trigger("pointerdown", { eventConstructor: "PointerEvent", pointerType: "touch", pointerId: 7, isPrimary: true, button: 0, ...at });
      });
      cy.get(ts(INSPECT_SHEET), { timeout: LONG_PRESS_MS + 4000 }).should("be.visible");
      cy.get(`${ts(INSPECT_SHEET)} .inspect-face--sheet`).should(($face) => {
        const face = $face[0] as HTMLElement;
        const wanted = Math.min(PREVIEW_SHEET_HEIGHT_PX, size.height * PREVIEW_SHEET_SHARE, ((size.width - 2 * 24) * 7) / 5);
        const was = Math.min(SHEET_BEFORE.px, size.height * SHEET_BEFORE.share);
        expect(face.offsetHeight, "the sheet's card").to.be.closeTo(wanted, SLACK_PX + 1);
        expect(face.offsetHeight, "taller than it was").to.be.at.least(was + LARGER_PX);
        expect(face.getBoundingClientRect().width, "inside the screen").to.be.at.most(size.width);
      });
      cy.get(unit).trigger("pointerup", { eventConstructor: "PointerEvent", pointerType: "touch", pointerId: 7, isPrimary: true, button: 0, force: true });
    });
  }
});
