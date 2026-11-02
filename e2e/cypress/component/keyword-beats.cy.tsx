// R2789000: the combat keyword beats over a real board, caught mid-flight at the slowest effects speed:
// Cleave's arc over the lanes beside the target, and First Strike's cancelled counter, the dying defender's
// counter-blow breaking off with the attacker never taking a hit. The callout is a DOM badge over the
// attacker, the arc and the break-off canvas cues; the screenshots are the pixel record, and the
// assertions are what they show (the layer's DOM, the board's `data-animating`).
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/keyword-beats.cy.tsx

import { FX_SPEED_DEFAULT, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
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
    // Mid-flight: lane 3's unit is taking the Cleave hit and the keyword's badge is over the attacker.
    cy.get(`[data-testid="card-${neighbour}"]`, SLOW).should("have.attr", "data-animating", "damage");
    cy.get(`${FX_DOM} [data-fx="callout"][data-callout="Cleave"]`).should("exist");
    cy.screenshot("cleave-arc-mid", { capture: "viewport", overwrite: true });
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
    // Mid-flight: the defender is dissolving, its counter-blow breaking off, and the glint's badge is over the attacker.
    cy.get(`[data-testid="card-${defender}"]`, SLOW).should("have.attr", "data-animating", "destroyed");
    cy.get(`${FX_DOM} [data-fx="callout"][data-callout="First Strike"]`).should("exist");
    cy.screenshot("first-strike-cancelled-mid", { capture: "viewport", overwrite: true });
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
