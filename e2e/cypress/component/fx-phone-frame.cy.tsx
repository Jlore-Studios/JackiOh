// R2803000: the audit's phone frame-time spec. At the phone particle cap with a phone viewport,
// the heaviest set pieces (Twisting Nether, the Starship launch, a Mythic takeover) and a board
// with 4 auras hold their mean frame time within FX_PHONE_FRAME_BUDGET_MS.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-phone-frame.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { planAuras } from "../../../apps/web/src/fx/aura/plan.ts";
import { FX_PARTICLE_CAP_MOBILE, FX_PHONE_FRAME_BUDGET_MS } from "../../../apps/web/src/fx/constants.ts";
import { capacityFor } from "../../../apps/web/src/fx/director.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import {
  baseView,
  dormantStarship,
  emptySide,
  faceUpBackrow,
  fullBoardView,
  unit,
  withEvents,
} from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const PHONE = { width: 390, height: 844 };
const BOARD = '[data-testid="board"]';
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';
/** Long enough to cross a set piece's hold, short enough to stay mid-flight. */
const SAMPLE_MS = 2_000;

/** rAF deltas over SAMPLE_MS in the browser: what the player feels, as the director's adapt loop sees it. */
function sampleFrames(): Cypress.Chainable<number[]> {
  return cy.window().then(
    (win) =>
      new Cypress.Promise<number[]>((resolve) => {
        const deltas: number[] = [];
        const start = win.performance.now();
        let last = start;
        const tick = (): void => {
          const now = win.performance.now();
          deltas.push(now - last);
          last = now;
          if (now - start < SAMPLE_MS) win.requestAnimationFrame(tick);
          else resolve(deltas);
        };
        win.requestAnimationFrame(tick);
      }),
  );
}

function mean(values: readonly number[]): number {
  return values.reduce((a, b) => a + b, 0) / Math.max(1, values.length);
}

function expectBudget(deltas: readonly number[]): void {
  const frames = deltas.slice(1);
  expect(frames.length, "frames sampled").to.be.greaterThan(10);
  const sorted = [...frames].sort((a, b) => a - b);
  const dist = `min=${sorted[0]?.toFixed(0)} p50=${sorted[Math.floor(sorted.length / 2)]?.toFixed(0)} p90=${sorted[Math.floor(sorted.length * 0.9)]?.toFixed(0)} max=${sorted[sorted.length - 1]?.toFixed(0)}`;
  expect(mean(frames), `mean frame ${mean(frames).toFixed(1)}ms over ${String(frames.length)} frames [${dist}]`).to.be.lessThanOrEqual(FX_PHONE_FRAME_BUDGET_MS);
}

/** Mounts the board, then plays `events` over `shown`. */
function mountScene(initial: PlayerView, shown: PlayerView, events: GameEvent[]): void {
  const lookup = lookupFromDefs(CATALOG);
  const noop = (): void => undefined;
  const board = (view: PlayerView) => (
    <CatalogContext.Provider value={lookup}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>
    </CatalogContext.Provider>
  );
  cy.mount(board(initial)).then(({ rerender }) => {
    cy.get(LAYER).should("have.attr", "data-fx", "on");
    cy.then(() => rerender(board(withEvents(shown, events))));
  });
}

/** Every Unit on the board, as the removals of `type` a wipe streams. */
function removals(view: PlayerView, type: "destroyed" | "exiled"): GameEvent[] {
  return [...view.you.units, ...view.opponent.units].flatMap((u): GameEvent[] => {
    if (u === null) return [];
    return type === "destroyed"
      ? [{ type, instanceId: u.instanceId, defId: u.defId, owner: u.owner, controller: u.controller, attack: u.attack, maxHealth: u.maxHealth, killerId: null }]
      : [{ type, instanceId: u.instanceId, defId: u.defId, owner: u.owner }];
  });
}

describe("R2803000 the heaviest scenes hold the phone frame budget", () => {
  it("R2803000 the phone viewport draws at the phone particle cap", () => {
    expect(capacityFor(PHONE.width)).to.eq(FX_PARTICLE_CAP_MOBILE);
    cy.viewport(PHONE.width, PHONE.height);
    cy.window().then((win) => {
      expect(win.innerWidth).to.eq(PHONE.width);
    });
  });

  it("R2803000 Twisting Nether holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    const me = view.viewer;
    mountScene(view, view, [
      { type: "cardPlayed", player: me, instanceId: "fx-clear", defId: "core-088", costPaid: 4 },
      { type: "hookResolving", sourceId: "fx-clear", defId: "core-088", controller: me, hook: "cry" },
      ...removals(view, "destroyed"),
      { type: "cardResolved", player: me, instanceId: "fx-clear", defId: "core-088", permanent: false, costPaid: 4 },
    ]);
    cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "nether");
    sampleFrames().then(expectBudget);
  });

  it("R2803000 the Starship launch holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const HULL = "stinkyscape-t-dormant-starship";
    const fused = [HULL, "stinkyscape-140", "stinkyscape-141"];
    const ship = (launched: number): PlayerView =>
      baseView({
        you: emptySide("p1", {
          units: [
            null,
            null,
            null,
            null,
            launched === 0
              ? dormantStarship("p1", fused, { instanceId: "fx-ship" })
              : unit("p1", { instanceId: "fx-ship", defId: HULL, attack: 0, starship: { pieces: fused.map((defId) => ({ defId })), name: "Ember of Departure", launched } }),
          ],
        }),
        opponent: emptySide("p2"),
      });
    mountScene(ship(0), ship(1), [
      { type: "awakened", player: "p1", instanceId: "fx-ship", defId: HULL },
      { type: "starshipLaunched", player: "p1", instanceId: "fx-ship", defId: HULL, name: "Ember of Departure", pieces: fused },
    ]);
    cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "starship");
    cy.get(`${FX_DOM} [data-fx="launch"] .fx-launch-count`).should("have.length", 3);
    sampleFrames().then(expectBudget);
  });

  it("R2803000 a Mythic takeover holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    mountScene(view, view, [
      { type: "cardPlayed", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", costPaid: 2 },
      { type: "summoned", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", row: "units", lane: 3 },
    ]);
    cy.get(GAME).should("have.attr", "data-takeover", "mythic");
    cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", "ai");
    sampleFrames().then(expectBudget);
  });

  it("R2803000 a board with 4 auras holds the phone frame budget", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    view.you.backrow = [
      faceUpBackrow("p1", { defId: "core-014", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "classic-042", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "classic-075", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "meditative-040", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "core-084", type: "Trap", radiant: true }),
    ];
    expect(planAuras(view, new Map()).map((aura) => aura.defId)).to.have.length(4);
    mountScene(view, view, []);
    cy.get(`[data-aura-look]`, { timeout: 4_000 }).should("have.length.greaterThan", 0);
    cy.get(CANVAS).should("exist");
    sampleFrames().then(expectBudget);
  });
});
