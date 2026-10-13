// BUILD M5-T1 measures the 10-unit, 10-backrow, stacked-pile fixture in a real browser.
// B28 and B46 extend it to the mobile and tablet viewports without horizontal or vertical overflow.
// Mount `Game` in `.app-shell--wide`, its narrowest product container; a bare `Board` gets space it
// never has. The width checks, full-card visibility, and board span distinguish overflow from a
// clipped or collapsed board.

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
  /** Recorded, never asserted — see the note where these are filled in. */
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
        // B46: the game screen fills the viewport without vertical scrolling.
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

// R504: measure empty hand rows in a browser and keep the field in place at every viewport.
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
