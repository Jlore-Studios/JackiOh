// BUILD M5-T1 measures the 10-unit, 10-backrow, stacked-pile fixture in a real browser; B28 and B46
// extend it to the mobile and tablet viewports, without overflow either way. Mount `Game` in
// `.app-shell--wide`, its narrowest product container; a bare `Board` gets space it never has.

import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

/** BUILD M5-T1's viewports plus B28's landscape phone and portrait tablet. */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "landscape", width: 844, height: 390 },
  { label: "tablet", width: 768, height: 1024 },
  { label: "phone", width: 390, height: 844 },
  { label: "tablet landscape", width: 1024, height: 768 },
  { label: "phone, Safari's visible viewport", width: 390, height: 664 },
] as const;

type Viewport = (typeof VIEWPORTS)[number];

/** `[data-testid="board"]` — `apps/web/src/game/contract.ts` `testid.board`. */
const BOARD = '[data-testid="board"]';

/** BUILD M5-T1's 10 units and 10 backrow cards. */
const FIELD_CARDS = `${BOARD} .field .card`;
const FIELD_CARD_COUNT = 20;

/** One measured viewport, as it reaches the terminal. */
type Measurement = {
  viewport: string;
  viewportWidth: number;
  documentScrollWidth: number;
  bodyScrollWidth: number;
  boardScrollWidth: number;
  boardRight: number;
  boardWidth: number;
  documentScrollHeight: number;
  boardHeight: number;
  boardBottom: number;
};

function measure(doc: Document, viewport: Viewport, board: Element): Measurement {
  const box = board.getBoundingClientRect();
  return {
    viewport: `${viewport.label} ${viewport.width}x${viewport.height}`,
    viewportWidth: viewport.width,
    documentScrollWidth: doc.documentElement.scrollWidth,
    bodyScrollWidth: doc.body.scrollWidth,
    boardScrollWidth: board.scrollWidth,
    boardRight: Math.ceil(box.right),
    boardWidth: Math.round(box.width),
    documentScrollHeight: doc.documentElement.scrollHeight,
    boardHeight: Math.round(box.height),
    boardBottom: Math.ceil(box.bottom),
  };
}

describe("BUILD M5-T1 — the full fixture board fits 1280x720, 844x390, 768x1024 and 390x844", () => {
  beforeEach(() => {
    // Assert BUILD M5-T1's dense fixture before measuring it; a sparse board is an easy pass.
    const view = fullBoardView();
    const units = [...view.you.units, ...view.opponent.units].filter((unit) => unit !== null);
    const backrow = [...view.you.backrow, ...view.opponent.backrow].filter((slot) => slot !== null);

    expect(units, "BUILD M5-T1: the fixture board holds 10 units").to.have.length(10);
    expect(backrow, "BUILD M5-T1: the fixture board holds 10 backrow cards").to.have.length(10);
    expect(
      units.filter((unit) => unit.buried > 0),
      "BUILD M5-T1: a stacked pile",
    ).to.have.length.greaterThan(0);

    // `legal: []` changes borders and opacity, never geometry; this fixture has no prompt.
    cy.mount(
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={() => undefined} />
      </div>,
    );
  });

  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`B28 draws all 20 field cards with no horizontal overflow at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);

      cy.get(BOARD).should("be.visible");
      // Count and per-card visibility prevent missing or clipped lanes from making a sparse board pass.
      cy.get(FIELD_CARDS)
        .should("have.length", FIELD_CARD_COUNT)
        .each(($card) => {
          cy.wrap($card, { log: false }).should("be.visible");
        });

      const out: { measured?: Measurement } = {};

      // `should` retries until the viewport change has settled.
      cy.document({ log: false }).should((doc) => {
        const board = doc.querySelector(BOARD);
        expect(board, "the board is mounted").to.not.eq(null);
        if (board === null) return;

        const m = measure(doc, viewport, board);

        expect(m.documentScrollWidth, `the document fits ${where}`).to.be.at.most(viewport.width);
        expect(m.bodyScrollWidth, `the body fits ${where}`).to.be.at.most(viewport.width);
        expect(m.boardScrollWidth, `the board's contents fit ${where}`).to.be.at.most(viewport.width);
        expect(m.boardRight, `the board's right edge is inside ${where}`).to.be.at.most(viewport.width);
        expect(m.documentScrollHeight, `the document fits ${where} vertically`).to.be.at.most(viewport.height);

        // Prevent a collapsed board from trivially fitting.
        expect(m.boardWidth, `the board spans ${where} (minus .app-shell's padding)`).to.be.at.least(
          viewport.width - 40,
        );

        out.measured = m;
      });

      cy.then(() => {
        cy.task("layout:report", out.measured, { log: false });
      });
    });
  }
});

describe("R504 an empty hand keeps its place at every size", () => {
  function handRows(doc: Document): { you: number; opponent: number; fieldTop: number } {
    const height = (id: string): number => doc.querySelector(`[data-testid="${id}"]`)?.getBoundingClientRect().height ?? -1;
    return {
      you: height("hand-you"),
      opponent: height("hand-opponent"),
      fieldTop: doc.querySelector(`${BOARD} .field`)?.getBoundingClientRect().top ?? -1,
    };
  }

  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`R504 both hands keep their height with no cards, and the field stays put, at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const full = fullBoardView();
      const empty = { ...full, you: { ...full.you, hand: [] }, opponent: { ...full.opponent, hand: { count: 0 } } };
      const seen: { full?: ReturnType<typeof handRows> } = {};

      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={full} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get(`${BOARD} [data-testid="hand-you"] .hand-slot`).should("have.length", Array.isArray(full.you.hand) ? full.you.hand.length : 0);
      cy.document({ log: false }).then((doc) => {
        seen.full = handRows(doc);
      });

      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={empty} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get('[data-testid="hand-empty-you"]').should("exist");
      cy.get('[data-testid="hand-empty-opponent"]').should("exist");
      cy.document({ log: false }).should((doc) => {
        const before = seen.full;
        expect(before, "the full board was measured").to.not.eq(undefined);
        if (before === undefined) return;
        const after = handRows(doc);
        expect(before.you, `your hand has a height at ${where}`).to.be.greaterThan(0);
        expect(after.you, `your empty hand keeps its height at ${where}`).to.be.closeTo(before.you, 1);
        expect(after.opponent, `their empty hand keeps its height at ${where}`).to.be.closeTo(before.opponent, 1);
        expect(after.fieldTop, `the field does not move at ${where}`).to.be.closeTo(before.fieldTop, 1);
      });
    });
  }
});

// #2330: where the board has the room the four rows are one height, the lanes fill the field and a DEF
// card is drawn at full size; on a phone the backrow keeps B29's 44 px floor. No field card leaves its zone.
describe("#2330 equal board rows, and DEF cards only as small as their zone needs", () => {
  /** The issue's sizes; `roomy` is the 761px-and-up layouts (a desktop, a tablet held upright). */
  const SIZES = [
    { label: "desktop", width: 1440, height: 900, roomy: true },
    { label: "desktop", width: 1280, height: 720, roomy: true },
    { label: "tablet", width: 768, height: 1024, roomy: true },
    { label: "phone", width: 390, height: 844, roomy: false },
    { label: "landscape", width: 844, height: 390, roomy: false },
  ] as const;
  /** B29's touch floor, which a phone's backrow row keeps. */
  const BACKROW_FLOOR_PX = 44;
  /** Sub-pixel rounding between layout boxes and their transformed rects. */
  const SLACK_PX = 1;

  function inside(inner: DOMRect, outer: DOMRect): boolean {
    return (
      inner.left >= outer.left - SLACK_PX &&
      inner.right <= outer.right + SLACK_PX &&
      inner.top >= outer.top - SLACK_PX &&
      inner.bottom <= outer.bottom + SLACK_PX
    );
  }

  for (const size of SIZES) {
    const where = `${size.label} ${size.width}x${size.height}`;

    it(`#2330 rows, lanes and every DEF card's footprint at ${where}`, () => {
      cy.viewport(size.width, size.height);
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={fullBoardView()} legal={[]} onAction={() => undefined} />
        </div>,
      );
      cy.get(FIELD_CARDS).should("have.length", FIELD_CARD_COUNT);

      cy.document({ log: false }).should((doc) => {
        const field = doc.querySelector<HTMLElement>(`${BOARD} .field`);
        expect(field, "the field is mounted").to.not.eq(null);
        if (field === null) return;
        const zones = [...field.querySelectorAll<HTMLElement>(".zone")];
        expect(zones, "4 rows of 5 zones").to.have.length(20);

        if (size.roomy) {
          const first = zones[0]?.getBoundingClientRect().height ?? 0;
          expect(first, `a row has a height at ${where}`).to.be.greaterThan(0);
          for (const zone of zones) {
            expect(zone.getBoundingClientRect().height, `${zone.dataset.testid ?? "a zone"} is as tall as the first at ${where}`).to.be.closeTo(first, SLACK_PX);
          }
          const style = getComputedStyle(field);
          const inner = field.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
          for (const lane of field.querySelectorAll<HTMLElement>(".lane")) {
            expect(lane.getBoundingClientRect().height, `lane ${lane.dataset.lane ?? "?"} fills the field at ${where}`).to.be.closeTo(inner, SLACK_PX);
          }
        } else {
          for (const zone of zones.filter((each) => each.dataset.row === "backrow")) {
            expect(zone.getBoundingClientRect().height, `${zone.dataset.testid ?? "a zone"} keeps the touch floor at ${where}`).to.be.at.least(BACKROW_FLOOR_PX);
          }
        }

        for (const card of field.querySelectorAll<HTMLElement>(".zone > .card")) {
          const zone = card.parentElement;
          if (zone === null) continue;
          expect(inside(card.getBoundingClientRect(), zone.getBoundingClientRect()), `${card.dataset.testid ?? "a back"} stays inside its zone at ${where}`).to.eq(true);
        }

        const defs = [...field.querySelectorAll<HTMLElement>('.card[data-position="DEF"]')];
        expect(defs, "fullBoardView's two DEF units").to.have.length(2);
        if (!size.roomy) return;
        for (const def of defs) {
          // Turned at full size, the footprint is the card's own box on its side.
          const box = def.getBoundingClientRect();
          expect(box.width, `${def.dataset.testid ?? "?"} footprint width at ${where}`).to.be.closeTo(def.offsetHeight, SLACK_PX);
          expect(box.height, `${def.dataset.testid ?? "?"} footprint height at ${where}`).to.be.closeTo(def.offsetWidth, SLACK_PX);
        }
      });
    });
  }
});
