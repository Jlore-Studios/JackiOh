// R2789000: the combat keyword beats over a real board, caught mid-flight at the slowest effects speed:
// Cleave's arc over the lanes beside the target, and First Strike's cancelled counter, the dying defender's
// counter-blow breaking off with the attacker never taking a hit. The callout is a DOM badge over the
// attacker, the arc and the break-off canvas cues. A shot waits until the canvas holds a lit pixel where the
// cue is drawn (the arc between the two lanes, the break-off path in front of the defender), then is taken with
// CSS animations running (e2e/README.md: by default Cypress jumps them to their end, the badge to opacity 0).
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/keyword-beats.cy.tsx

import { FX_FIRST_STRIKE_BREAK_REACH, FX_SPEED_DEFAULT, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const LAYER = '[data-testid="fx-layer"]';
const FX_DOM = '[data-testid="fx-dom"]';
/** A quarter speed stretches every entry to four times its table time, so a mid-flight frame is there to catch. */
const SLOW = { timeout: 12_000 } as const;

const noop = (): void => undefined;

type Box = { left: number; top: number; right: number; bottom: number };

/** Whether the canvas holds a lit (non-transparent) pixel inside `box`, in viewport pixels. */
function litIn(canvas: HTMLCanvasElement, box: Box): boolean {
  const ctx = canvas.getContext("2d");
  if (ctx === null) return false;
  const rect = canvas.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) return false;
  const sx = canvas.width / rect.width;
  const sy = canvas.height / rect.height;
  const x0 = Math.max(0, Math.floor((box.left - rect.left) * sx));
  const y0 = Math.max(0, Math.floor((box.top - rect.top) * sy));
  const x1 = Math.min(canvas.width, Math.ceil((box.right - rect.left) * sx));
  const y1 = Math.min(canvas.height, Math.ceil((box.bottom - rect.top) * sy));
  if (x1 <= x0 || y1 <= y0) return false;
  const data = ctx.getImageData(x0, y0, x1 - x0, y1 - y0).data;
  for (let i = 3; i < data.length; i += 4) {
    if ((data[i] ?? 0) > 0) return true;
  }
  return false;
}

/** Retries until the canvas is lit inside the box `region` reads, then the shot follows at once, animations running. */
function shootWhenLit(name: string, region: () => Box): void {
  cy.get('[data-testid="fx-canvas"]', SLOW).should(($canvas) => {
    expect(litIn($canvas[0] as HTMLCanvasElement, region()), "the cue is drawn on the canvas here").to.eq(true);
  });
  cy.screenshot(name, { capture: "viewport", overwrite: true, disableTimersAndAnimations: false });
}

const rectOf = (selector: string): DOMRect => {
  const el = Cypress.$(selector)[0];
  if (el === undefined) throw new Error(`${selector} is not on the page`);
  return el.getBoundingClientRect();
};

function unitId(view: PlayerView, side: "you" | "opponent", lane: number): string {
  const unit = view[side].units[lane - 1];
  if (unit === null || unit === undefined) throw new Error(`fullBoardView() has no ${side} unit in lane ${String(lane)}`);
  return unit.instanceId;
}

const board = (shown: PlayerView) => (
  <div className="app-shell app-shell--wide">
    <Game view={shown} legal={[]} onAction={noop} />
  </div>
);

/** Mounts the board, waits for the layer, then plays `events` over it. */
function play(view: PlayerView, events: GameEvent[]): void {
  cy.mount(board(view)).then(({ rerender }) => {
    cy.get(LAYER).should("have.attr", "data-fx", "on");
    cy.then(() => rerender(board(withEvents(view, events))));
  });
}

describe("R2789000 the combat keyword beats over a real board", () => {
  beforeEach(() => {
    cy.viewport(1280, 720);
    setFxSettings({ speed: FX_SPEED_MIN });
  });

  afterEach(() => {
    setFxSettings({ speed: FX_SPEED_DEFAULT });
  });

  it("R2789000 Cleave's arc mid-flight", () => {
    const view = fullBoardView();
    const attacker = unitId(view, "you", 1);
    const target = unitId(view, "opponent", 2);
    const neighbour = unitId(view, "opponent", 3);
    play(view, [
      { type: "attackDeclared", attackerId: attacker, targetId: target, forced: false },
      { type: "damage", sourceId: attacker, targetId: target, amount: 1, combat: true },
      { type: "damage", sourceId: attacker, targetId: neighbour, amount: 1, combat: true, via: "Cleave" },
    ]);
    // Mid-flight: lane 3's unit is taking the Cleave hit, the keyword's badge is over the attacker, and the arc
    // is drawn across the gap from the struck lane to its neighbour (it lands at the entry's Land phase, after
    // which the lit pixels there would be the splat's burst; the shot follows the first lit frame at once).
    cy.get(`[data-testid="card-${neighbour}"]`, SLOW).should("have.attr", "data-animating", "damage");
    cy.get(`${FX_DOM} [data-fx="callout"][data-callout="Cleave"]`).should("exist");
    shootWhenLit("cleave-arc-mid", () => {
      const from = rectOf(`[data-testid="card-${target}"]`);
      const to = rectOf(`[data-testid="card-${neighbour}"]`);
      return {
        left: Math.min(from.right, to.left),
        right: Math.max(from.right, to.left),
        top: Math.min(from.top, to.top),
        bottom: Math.max(from.bottom, to.bottom),
      };
    });
    // At rest: nothing animating and every cue gone.
    cy.get("[data-animating]", SLOW).should("not.exist");
    cy.get(FX_DOM, { timeout: 4_000 }).should(($root) => {
      expect(($root[0] as HTMLElement).childElementCount, "fx-dom is empty").to.eq(0);
    });
  });

  it("R2789000 First Strike's cancelled counter mid-flight", () => {
    const view = fullBoardView();
    const attacker = unitId(view, "you", 1);
    const defender = unitId(view, "opponent", 2);
    const attackerCard = `[data-testid="card-${attacker}"]`;
    // What the attacker's card was animating, in order: it must never be a damage.
    const log: string[] = [];
    let observer: MutationObserver | null = null;
    cy.mount(board(view)).then(({ rerender }) => {
      cy.get(LAYER).should("have.attr", "data-fx", "on");
      cy.get(attackerCard).then(($card) => {
        const card = $card[0] as HTMLElement;
        observer = new MutationObserver(() => {
          const now = card.getAttribute("data-animating");
          if (now !== null && log[log.length - 1] !== now) log.push(now);
        });
        observer.observe(card, { attributes: true, attributeFilter: ["data-animating"] });
      });
      cy.then(() =>
        rerender(
          board(
            withEvents(view, [
              { type: "attackDeclared", attackerId: attacker, targetId: defender, forced: false },
              { type: "damage", sourceId: attacker, targetId: defender, amount: 9, combat: true, via: "First Strike" },
              {
                type: "destroyed",
                instanceId: defender,
                defId: "core-013",
                owner: "p2",
                controller: "p2",
                attack: 3,
                maxHealth: 5,
                killerId: attacker,
                via: "First Strike",
              },
            ]),
          ),
        ),
      );
    });
    // Mid-flight: the defender is dissolving, its counter-blow is in the air on the path toward the attacker that
    // breaks off FX_FIRST_STRIKE_BREAK_REACH card heights past the defender, and the glint's badge is over the attacker.
    cy.get(`[data-testid="card-${defender}"]`, SLOW).should("have.attr", "data-animating", "destroyed");
    cy.get(`${FX_DOM} [data-fx="callout"][data-callout="First Strike"]`).should("exist");
    shootWhenLit("first-strike-cancelled-mid", () => {
      const card = rectOf(`[data-testid="card-${defender}"]`);
      return {
        left: card.left,
        right: card.right,
        top: card.bottom,
        bottom: card.bottom + card.height * FX_FIRST_STRIKE_BREAK_REACH,
      };
    });
    cy.get("[data-animating]", SLOW).should("not.exist");
    cy.then(() => {
      observer?.disconnect();
      expect(log, "the attacker lunged and never took a hit").to.include("attackDeclared");
      expect(log, "no counter-blow landed on the attacker").to.not.include("damage");
    });
    cy.get(FX_DOM, { timeout: 4_000 }).should(($root) => {
      expect(($root[0] as HTMLElement).childElementCount, "fx-dom is empty").to.eq(0);
    });
  });
});
