// Issue #477: the death screen's rematch and double-or-nothing offers (routes/Rematch.tsx, R672), in
// the result panel at 1280x800 and 390x844, measured in a real browser.
//
// Before the fix the offers were a column beside the panel's row of ways on: the panel's "first way
// on is the primary" rule painted the whole column gold and gave its ink to Double or nothing (dark
// on dark wood), Rematch wore the lobby's blue call to action, both offers were 38 px tall beside
// 44 px buttons, the row stretched Back to lobby and View the board to the column's height (89 px
// at 1280x800 with an offer in), and the incoming line set the panel's width (824 px). Now:
//
//   - the offers are the panel's buttons: at least TOUCH_PX tall, as tall as Back to lobby, and on
//     its row (all four on one row at 1280x800; the two offers on one row, the route's two below,
//     at 390x844);
//   - the block draws no box of its own, Rematch wears the panel's gold primary and Double or
//     nothing the same face and ink as the route's other buttons;
//   - the incoming offer is a line above the buttons and the wait or the note a line below, and
//     neither stretches the panel past its buttons;
//   - everything sits inside the panel, the panel inside the screen, and nothing scrolls sideways.
//
// jsdom has no layout engine; apps/web/src/routes/Rematch.test.tsx holds the markup and reads the
// rules as text. The mount is the match route's (routes/match.tsx `resultActions`): `Game` inside
// `.app-shell.app-shell--wide` with the offers, then Back to lobby. The rematch status is answered by
// a stubbed `fetch`; no server runs. Reduced motion is on, so the panel comes in at once.
//
// Every measurement is taken inside a `should` callback so it is retried until the layout settles.

import Game from "../../../apps/web/src/game/Game.tsx";
import RematchButtons from "../../../apps/web/src/routes/Rematch.tsx";
import type { RematchStatusResponse } from "../../../apps/web/src/net/api.ts";
import { writeSettings, __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { baseView } from "../../../apps/web/src/test/fixtures.ts";

/** WCAG 2.5.5 / Apple HIG, the floor S11 sets for every touch target. */
const TOUCH_PX = 44;
/** Subpixel slack for edges the browser rounds; never enough to hide a real overflow. */
const EPSILON = 0.5;
/** `--primary-ink` (apps/web/src/index.css), as the browser computes it. */
const PRIMARY_INK = "rgb(43, 24, 0)";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 800, oneRow: true },
  { label: "phone", width: 390, height: 844, oneRow: false },
] as const;

const IDLE: RematchStatusResponse = { youOffered: null, opponentOffer: null, opponentHere: true, matchId: null };

const STATES = [
  { label: "a ranked match, nothing offered", ranked: true, status: IDLE, lines: 0 },
  { label: "an unranked match, with its note", ranked: false, status: IDLE, lines: 1 },
  {
    label: "their double-or-nothing in, our rematch waiting",
    ranked: true,
    status: { ...IDLE, youOffered: 1, opponentOffer: 2 },
    lines: 2,
  },
] as const;

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** Answers `GET /api/matches/:id/rematch` with `status`; anything else goes out as it would. */
function stubRematchStatus(status: RematchStatusResponse): void {
  cy.window({ log: false }).then((win) => {
    const passThrough = win.fetch.bind(win);
    cy.stub(win, "fetch").callsFake((input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof win.Request ? input.url : String(input);
      if (!url.includes("/rematch")) return passThrough(input, init);
      return Promise.resolve(
        new win.Response(JSON.stringify(status), { status: 200, headers: { "content-type": "application/json" } }),
      );
    });
  });
}

function mountPanel(ranked: boolean): void {
  writeSettings({ reduceMotion: true });
  cy.mount(
    <div className="app-shell app-shell--wide">
      <Game
        view={baseView({ result: { winner: "p1", reason: "hero-death" } })}
        legal={[]}
        onAction={() => undefined}
        resultActions={
          <>
            <RematchButtons token="token-1" matchId="match-1" connection="open" ranked={ranked} />
            <button type="button" data-testid="result-back">
              Back to lobby
            </button>
          </>
        }
      />
    </div>,
  );
}

function rectOf(doc: Document, selector: string): DOMRect {
  const element = doc.querySelector(selector);
  expect(element, selector).to.not.eq(null);
  return (element as Element).getBoundingClientRect();
}

function styleOf(doc: Document, selector: string): CSSStyleDeclaration {
  const element = doc.querySelector(selector);
  const win = doc.defaultView;
  expect(element, selector).to.not.eq(null);
  expect(win, "the window").to.not.eq(null);
  return (win as Window).getComputedStyle(element as Element);
}

function inside(inner: DOMRect, outer: DOMRect, what: string): void {
  expect(inner.left, `${what}: left edge`).to.be.at.least(outer.left - EPSILON);
  expect(inner.right, `${what}: right edge`).to.be.at.most(outer.right + EPSILON);
  expect(inner.top, `${what}: top edge`).to.be.at.least(outer.top - EPSILON);
  expect(inner.bottom, `${what}: bottom edge`).to.be.at.most(outer.bottom + EPSILON);
}

describe("#477 the rematch offers in the result panel", () => {
  afterEach(() => {
    __resetSettingsForTests();
  });

  for (const viewport of VIEWPORTS) {
    for (const state of STATES) {
      const where = `${state.label} at ${viewport.label} ${viewport.width}x${viewport.height}`;

      it(`renders cleanly: ${where}`, () => {
        cy.viewport(viewport.width, viewport.height);
        stubRematchStatus(state.status);
        mountPanel(state.ranked);

        cy.get(ts("result-overlay")).should("have.attr", "data-revealed", "true");
        cy.get(ts("rematch-offer")).should("be.visible");

        cy.document({ log: false }).should((doc) => {
          const offer = rectOf(doc, ts("rematch-offer"));
          const double = rectOf(doc, ts("rematch-double"));
          const back = rectOf(doc, ts("result-back"));
          const view = rectOf(doc, ts("result-view-board"));
          const panel = rectOf(doc, ts("result-overlay"));
          const actions = rectOf(doc, ".result-overlay__actions");

          // The panel's buttons, all four: touch-sized and one height.
          for (const [name, box] of [
            ["Rematch", offer],
            ["Double or nothing", double],
            ["Back to lobby", back],
            ["View the board", view],
          ] as const) {
            expect(box.height, `${name} is a touch target`).to.be.at.least(TOUCH_PX - EPSILON);
            expect(box.height, `${name} is as tall as Back to lobby`).to.be.closeTo(back.height, EPSILON);
            inside(box, panel, `${name} inside the panel`);
          }

          // One row of ways on where it fits; on a phone, the offers' row over the route's.
          expect(double.top, "the two offers share a row").to.be.closeTo(offer.top, EPSILON);
          expect(view.top, "Back to lobby and View the board share a row").to.be.closeTo(back.top, EPSILON);
          if (viewport.oneRow) {
            expect(back.top, "the four ways on share one row").to.be.closeTo(offer.top, EPSILON);
            // The lines wrap to the buttons' width: each adds at most the row's gap, never its text.
            const gap = Number.parseFloat(styleOf(doc, ".result-overlay__actions").columnGap) || 0;
            expect(actions.width, "the incoming line and the wait do not widen the panel").to.be.at.most(
              view.right - offer.left + state.lines * gap + EPSILON,
            );
          } else {
            expect(back.top, "the route's ways on sit below the offers").to.be.greaterThan(offer.bottom);
          }

          // The block is no box; Rematch is the panel's gold primary and Double or nothing is drawn
          // as the route's other buttons are.
          expect(styleOf(doc, ".result-overlay__actions > .rematch").display, "the block draws no box").to.eq("contents");
          const rematchStyle = styleOf(doc, ts("rematch-offer"));
          expect(rematchStyle.backgroundImage, "Rematch wears the primary face").to.contain("linear-gradient");
          expect(rematchStyle.color, "Rematch's ink").to.eq(PRIMARY_INK);
          const doubleStyle = styleOf(doc, ts("rematch-double"));
          const backStyle = styleOf(doc, ts("result-back"));
          expect(doubleStyle.color, "Double or nothing reads like Back to lobby").to.eq(backStyle.color);
          expect(doubleStyle.backgroundImage, "Double or nothing has no primary face").to.eq(backStyle.backgroundImage);
          expect(doubleStyle.backgroundColor, "Double or nothing's face").to.eq(backStyle.backgroundColor);

          // The lines: the incoming offer above the buttons, the wait or the note below, in the panel.
          const incoming = doc.querySelector(ts("rematch-incoming"));
          if (incoming !== null) {
            const box = incoming.getBoundingClientRect();
            inside(box, panel, "the incoming offer inside the panel");
            expect(box.bottom, "the incoming offer is above the buttons").to.be.at.most(offer.top + EPSILON);
          }
          for (const line of doc.querySelectorAll(".rematch-status, .rematch-note")) {
            const box = line.getBoundingClientRect();
            inside(box, panel, "the wait or the note inside the panel");
            expect(box.top, "the wait or the note is below the buttons").to.be.at.least(Math.max(offer.bottom, view.bottom) - EPSILON);
          }
          expect(doc.querySelectorAll(".rematch-incoming, .rematch-status, .rematch-note").length, "the state's lines").to.eq(state.lines);

          // The panel is on the screen and nothing scrolls sideways.
          expect(panel.left, "the panel's left edge").to.be.at.least(-EPSILON);
          expect(panel.right, "the panel's right edge").to.be.at.most(viewport.width + EPSILON);
          expect(doc.documentElement.scrollWidth, "the document fits").to.be.at.most(viewport.width);
        });
      });
    }
  }
});
