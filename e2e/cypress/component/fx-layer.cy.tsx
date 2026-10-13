// Polish 1: B28 checks clamped DPR, B35 board shake, and B40 that browser effects draw without blocking the board.
// Mount a real `Game`, append events, then watch page frames before the effects begin.
// The watcher must see the splat before synchronous `getImageData`: a delayed read can miss R200's short-lived effect.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-layer.cy.tsx

import { FX_MAX_DPR } from "../../../apps/web/src/fx/constants.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

const BOARD = '[data-testid="board"]';
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const FX_DOM = '[data-testid="fx-dom"]';
const SPLAT = '[data-fx="splat"]';

const NEAR_PX = 48;

/** Fixture-derived IDs keep the burst valid if the fixture changes. */
function burstFor(view: PlayerView): { events: GameEvent[]; targetId: string } {
  const target = view.opponent.units[4];
  if (target === null || target === undefined) throw new Error("fullBoardView() has no enemy unit in lane 5");
  if (target.health <= 8) throw new Error("the lane-5 enemy must survive 8 damage so its card stays on the board");
  const spell = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
  if (spell === undefined) throw new Error("fullBoardView() has no card in the viewer's hand");
  return {
    targetId: target.instanceId,
    events: [
      { type: "damage", sourceId: spell.instanceId, targetId: target.instanceId, amount: 8, combat: false },
      { type: "summoned", player: view.viewer, instanceId: "fx-summoned", defId: "core-019", row: "units", lane: 3 },
      { type: "turnStarted", player: view.viewer, turn: view.turn + 1 },
    ],
  };
}

function litNear(canvas: HTMLCanvasElement, box: DOMRect, margin: number): boolean {
  const ctx = canvas.getContext("2d");
  if (ctx === null) return false;
  const rect = canvas.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) return false;
  const sx = canvas.width / rect.width;
  const sy = canvas.height / rect.height;
  const x0 = Math.max(0, Math.floor((box.left - margin - rect.left) * sx));
  const y0 = Math.max(0, Math.floor((box.top - margin - rect.top) * sy));
  const x1 = Math.min(canvas.width, Math.ceil((box.right + margin - rect.left) * sx));
  const y1 = Math.min(canvas.height, Math.ceil((box.bottom + margin - rect.top) * sy));
  if (x1 <= x0 || y1 <= y0) return false;
  const data = ctx.getImageData(x0, y0, x1 - x0, y1 - y0).data;
  for (let i = 3; i < data.length; i += 4) {
    if ((data[i] ?? 0) > 0) return true;
  }
  return false;
}

/** Page-time watcher budget, beyond every effect tail. */
const WATCH_MS = 6_000;

type Seen = {
  splatOverIdleCard: boolean;
  clickLandsOnCard: boolean;
  litNearCard: boolean;
  lastHit: string;
};

function unseen(): Seen {
  return { splatOverIdleCard: false, clickLandsOnCard: false, litNearCard: false, lastHit: "" };
}

/** Samples animation frames until complete or WATCH_MS; reads pixels only after the splat. */
function watchFrames(win: Window, card: string, seen: Seen): () => void {
  const doc = win.document;
  const start = win.performance.now();
  let handle = 0;
  let stopped = false;
  const tick = (): void => {
    if (stopped) return;
    const target = doc.querySelector<HTMLElement>(card);
    if (target !== null && !seen.clickLandsOnCard) {
      const splat = doc.querySelector(`${FX_DOM} ${SPLAT}`);
      if (splat !== null && !target.hasAttribute("data-animating")) {
        seen.splatOverIdleCard = true;
        const box = target.getBoundingClientRect();
        const hit = doc.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
        const fits = doc.documentElement.scrollWidth <= win.innerWidth;
        seen.lastHit = hit === null ? "nothing" : (hit.getAttribute("data-testid") ?? hit.getAttribute("class") ?? hit.tagName);
        if (hit !== null && target.contains(hit) && fits) seen.clickLandsOnCard = true;
      }
    }
    if (target !== null && seen.splatOverIdleCard && !seen.litNearCard) {
      const canvas = doc.querySelector<HTMLCanvasElement>(CANVAS);
      if (canvas !== null && litNear(canvas, target.getBoundingClientRect(), NEAR_PX)) seen.litNearCard = true;
    }
    const done = seen.clickLandsOnCard && seen.litNearCard;
    if (!done && win.performance.now() - start < WATCH_MS) handle = win.requestAnimationFrame(tick);
  };
  handle = win.requestAnimationFrame(tick);
  return () => {
    stopped = true;
    win.cancelAnimationFrame(handle);
  };
}

function expectNoHorizontalOverflow(doc: Document, where: string): void {
  const win = doc.defaultView;
  expect(win, "the document has a window").to.not.eq(null);
  if (win === null) return;
  expect(doc.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(win.innerWidth);
}

describe("Polish 1: the effects layer over a real board", () => {
  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`B40 B35 B28: a spell hit lights the canvas, shakes the board, blocks no click and leaves nothing behind at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);

      const view = fullBoardView();
      const { events, targetId } = burstFor(view);
      const card = `[data-testid="card-${targetId}"]`;
      const noop = (): void => undefined;

      const record = { translate: [] as string[], rotate: [] as string[] };
      const prior = { translate: "", rotate: "" };
      let observer: MutationObserver | null = null;
      // `should` retries the mutable watcher result; an assertion after `.then()` would not.
      const seen = unseen();
      let stopWatching: (() => void) | null = null;

      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={view} legal={[]} onAction={noop} />
        </div>,
      ).then(({ rerender }) => {
        cy.get(LAYER).should("have.attr", "data-fx", "on");
        cy.get(CANVAS).should("exist");
        cy.get(FX_DOM).should("exist");
        cy.document().should((doc) => expectNoHorizontalOverflow(doc, where));

        cy.get(BOARD).then(($board) => {
          const board = $board[0] as HTMLElement;
          prior.translate = board.style.translate;
          prior.rotate = board.style.rotate;
          observer = new MutationObserver(() => {
            record.translate.push(board.style.translate);
            record.rotate.push(board.style.rotate);
          });
          observer.observe(board, { attributes: true, attributeFilter: ["style"] });
          const win = board.ownerDocument.defaultView;
          if (win === null) throw new Error("the board has no window");
          stopWatching = watchFrames(win, card, seen);
        });

        cy.then(() => rerender(
          <div className="app-shell app-shell--wide">
            <Game view={withEvents(view, events)} legal={[]} onAction={noop} />
          </div>,
        ));
      });

      // B40: the splat leaves the idle card clickable and lights its canvas area.
      cy.wrap(seen).should((saw: Seen) => {
        expect(saw.splatOverIdleCard, "its damage splat is on screen after the card's own damage motion has finished").to.eq(true);
        expect(saw.clickLandsOnCard, `elementFromPoint at the card's centre is inside the card, and the board fits (last hit: ${saw.lastHit})`).to.eq(true);
        expect(saw.litNearCard, "a lit canvas pixel near the damaged card").to.eq(true);
      });

      // B28: backing store equals CSS size times clamped device pixel ratio.
      cy.window().then((win) => {
        const dpr = Math.min(Math.max(win.devicePixelRatio || 1, 1), FX_MAX_DPR);
        cy.get(CANVAS).should(($canvas) => {
          const canvas = $canvas[0] as HTMLCanvasElement;
          const rect = canvas.getBoundingClientRect();
          expect(rect.width, "the canvas has a CSS width").to.be.greaterThan(0);
          expect(canvas.width, "backing width = CSS width x DPR").to.be.closeTo(rect.width * dpr, 1);
          expect(canvas.height, "backing height = CSS height x DPR").to.be.closeTo(rect.height * dpr, 1);
        });
      });

      cy.get("[data-animating]").should("not.exist");

      cy.get(FX_DOM).should(($root) => {
        expect(($root[0] as HTMLElement).childElementCount, "fx-dom is empty").to.eq(0);
      });

      // B35: shake mutates `translate` then restores prior inline values.
      cy.get(BOARD).should(($board) => {
        const board = $board[0] as HTMLElement;
        expect(
          record.translate.some((value) => value !== "" && value !== prior.translate),
          "the board carried an inline translate while it shook",
        ).to.eq(true);
        expect(board.style.translate, "translate restored").to.eq(prior.translate);
        expect(board.style.rotate, "rotate restored").to.eq(prior.rotate);
      });

      cy.document().should((doc) => expectNoHorizontalOverflow(doc, where));
      cy.then(() => {
        observer?.disconnect();
        stopWatching?.();
      });
    });
  }
});
