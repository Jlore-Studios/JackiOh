// R2787000: Field Spells' auras over a real board. Two stand at once over the zones they cover and light
// the aura canvas; four on a phone keep within their share of the phone's particle cap and the frame budget.
// Run: E2E_COMPONENT_PORT=5282 pnpm --dir e2e test:component --spec cypress/component/aura-layer.cy.tsx

import { AURA_PARTICLE_SHARE, FX_ADAPT_SLOW_FACTOR, FX_ADAPT_SLOW_MS, FX_PARTICLE_CAP_MOBILE } from "../../../apps/web/src/fx/constants.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { faceUpBackrow, fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type Seat = PlayerView["you"];

const LAYER = '[data-testid="aura-layer"]';
const CANVAS = '[data-testid="aura-canvas"]';
const AURA = ".aura";
/** The rAF intervals the frame check averages. */
const FRAMES = 90;
/** How far an aura's box may sit from the zones it covers (sub-pixel layout). */
const NEAR_PX = 1;
const FRAMES_TIMEOUT_MS = 10_000;

const noop = (): void => undefined;

/** `fields` (lane → defId) put face-up in a seat's backrow, over whatever stood there. */
function place(seat: Seat, fields: Readonly<Record<number, string>>): Seat {
  return {
    ...seat,
    backrow: seat.backrow.map((slot, index) => {
      const defId = fields[index + 1];
      return defId === undefined ? slot : faceUpBackrow(seat.player, { defId, type: "Field Spell" });
    }),
  };
}

function boardWith(you: Readonly<Record<number, string>>, opponent: Readonly<Record<number, string>>): PlayerView {
  const view = fullBoardView();
  return { ...view, you: place(view.you, you), opponent: place(view.opponent, opponent) };
}

/** The box spanning every zone whose testid starts with `prefix`. */
function zonesBox(doc: Document, prefix: string): DOMRect {
  const rects = [...doc.querySelectorAll(`[data-testid^="${prefix}"]`)].map((zone) => zone.getBoundingClientRect());
  expect(rects.length, `zones ${prefix}*`).to.be.greaterThan(0);
  const left = Math.min(...rects.map((rect) => rect.left));
  const top = Math.min(...rects.map((rect) => rect.top));
  const right = Math.max(...rects.map((rect) => rect.right));
  const bottom = Math.max(...rects.map((rect) => rect.bottom));
  return new DOMRect(left, top, right - left, bottom - top);
}

function expectCovers(aura: Element, box: DOMRect, what: string): void {
  const rect = aura.getBoundingClientRect();
  expect(rect.left, `${what}: left`).to.be.closeTo(box.left, NEAR_PX);
  expect(rect.top, `${what}: top`).to.be.closeTo(box.top, NEAR_PX);
  expect(rect.width, `${what}: width`).to.be.closeTo(box.width, NEAR_PX);
  expect(rect.height, `${what}: height`).to.be.closeTo(box.height, NEAR_PX);
}

function anyLit(canvas: HTMLCanvasElement): boolean {
  const ctx = canvas.getContext("2d");
  if (ctx === null || canvas.width === 0 || canvas.height === 0) return false;
  const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
  for (let i = 3; i < data.length; i += 4) {
    if ((data[i] ?? 0) > 0) return true;
  }
  return false;
}

/** The next FRAMES intervals between animation frames, in ms. */
function frameIntervals(win: Window): Promise<number[]> {
  return new Promise((resolve) => {
    const stamps: number[] = [];
    const tick = (t: number): void => {
      stamps.push(t);
      if (stamps.length > FRAMES) resolve(stamps.slice(1).map((stamp, i) => stamp - (stamps[i] ?? stamp)));
      else win.requestAnimationFrame(tick);
    };
    win.requestAnimationFrame(tick);
  });
}

describe("R2787000 auras over a real board", () => {
  it("R2787000 two auras stand at once over the zones they cover and light the canvas", () => {
    cy.viewport(1280, 720);
    const view = boardWith({ 1: "core-046" }, { 3: "core-014" });
    cy.mount(
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>,
    );
    cy.get(LAYER).should("have.attr", "data-aura", "on");
    cy.get(AURA).should("have.length", 2);
    cy.get(`${AURA}[data-aura-look="void"]`).should("have.attr", "data-aura-scope", "board").and("have.attr", "data-aura-phase", "standing");
    cy.get(`${AURA}[data-aura-look="storm"]`).should("have.attr", "data-aura-scope", "side").and("have.attr", "data-aura-phase", "standing");
    cy.document().should((doc) => {
      const board = doc.querySelector(`${AURA}[data-aura-look="void"]`);
      const side = doc.querySelector(`${AURA}[data-aura-look="storm"]`);
      expect(board, "Suppressive Aura's aura").to.not.eq(null);
      expect(side, "Jlockeed's Weapons' aura").to.not.eq(null);
      if (board === null || side === null) return;
      expectCovers(board, zonesBox(doc, "zone-"), "the board aura spans every zone");
      expectCovers(side, zonesBox(doc, "zone-opponent-"), "the side aura spans the opponent's zones");
    });
    cy.get(CANVAS).should(($canvas) => {
      expect(anyLit($canvas[0] as HTMLCanvasElement), "a lit pixel on the aura canvas").to.eq(true);
    });
    cy.document().should((doc) => {
      expect(doc.documentElement.scrollWidth, "the auras add no scrollable width").to.be.at.most(doc.defaultView?.innerWidth ?? 0);
      expect(doc.querySelector(LAYER)?.innerHTML ?? "", "no definition on the page").not.to.match(/core-\d{3}/);
    });
  });

  it("R2787000 four auras at the phone's particle cap stay within the frame budget", () => {
    cy.viewport(390, 844);
    const view = boardWith({ 1: "classicplus-034", 2: "stinkyscape-155" }, { 4: "stinkyscape-046", 5: "traininggrounds-019" });
    cy.mount(
      <div className="app-shell">
        <Game view={view} legal={[]} onAction={noop} />
      </div>,
    );
    cy.get(AURA).should("have.length", 4);
    cy.get(LAYER).should(($layer) => {
      const motes = Number($layer.attr("data-aura-motes"));
      expect(motes, "motes drawn").to.be.greaterThan(0);
      expect(motes, "within the auras' share of the phone's cap").to.be.at.most(Math.floor(AURA_PARTICLE_SHARE * FX_PARTICLE_CAP_MOBILE));
    });
    cy.get(CANVAS).should(($canvas) => {
      expect(anyLit($canvas[0] as HTMLCanvasElement), "a lit pixel on the aura canvas").to.eq(true);
    });
    cy.window().then((win) => {
      cy.wrap(frameIntervals(win), { timeout: FRAMES_TIMEOUT_MS }).then((intervals) => {
        const list = intervals as number[];
        expect(list).to.have.length(FRAMES);
        const mean = list.reduce((sum, ms) => sum + ms, 0) / list.length;
        const budget = Math.max(FX_ADAPT_SLOW_MS, Math.min(...list) * FX_ADAPT_SLOW_FACTOR);
        expect(mean, `the mean of ${String(FRAMES)} frame intervals`).to.be.at.most(budget);
      });
    });
  });
});
