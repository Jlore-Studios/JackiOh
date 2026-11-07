// #3347: cards that move like cards, through the real board in a browser: a card played from the hand
// flies to its tile (the effects layer's stand-in, turned toward it), the deck riffles when it is
// shuffled, the opening hand is dealt a card at a time, and a dragged card leans with its speed. All of
// it is client-side presentation keyed on public events and the player's own input, so apps/web's vitest
// suite proves the numbers (game/opening.test.ts, game/drag/tilt.test.ts, test/ux/drag-layer.test.tsx,
// game/animations.riffle.test.ts); what only a browser can say is that each one draws, captured here at
// desktop size and on a phone.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/card-motion.cy.tsx

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, GameEvent } from "@jackioh/shared";

import { FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;

const GAME = '[data-testid="game"]';
const VIEWPORTS = [
  { width: 1280, height: 720 },
  { width: 390, height: 844 },
] as const;

/** fullBoardView's ids: your hand is c11 to c14, and lane 3 of your units is open. */
const HAND_CARD = "c11";
const PLAY: ActionBody = { type: "play", instanceId: HAND_CARD, zone: { row: "units", lane: 3 } };

const played: GameEvent[] = [
  { type: "cardPlayed", player: "p1", instanceId: HAND_CARD, defId: "core-019", costPaid: 3 },
  { type: "summoned", player: "p1", instanceId: HAND_CARD, defId: "core-019", row: "units", lane: 3 },
];
const shuffled: GameEvent[] = [
  { type: "traded", player: "p1" },
  { type: "shuffledIn", player: "p1", instanceId: "c99", defId: "core-070", position: 2 },
  { type: "drawn", player: "p1", instanceId: "c98", defId: "core-055" },
];

const slowly = (): void => {
  // At the slowest effects speed each motion outlasts the command queue, so a capture lands mid-flight.
  setFxSettings({ speed: FX_SPEED_MIN });
};

/** Mounts the board on `initial`, then shows `next` (when given) as the board's next view. */
function mount(initial: PlayerView, next?: PlayerView, legal: readonly ActionBody[] = []): void {
  const lookup = lookupFromDefs(CATALOG);
  const noop = (): void => undefined;
  const board = (view: PlayerView) => (
    <CatalogContext.Provider value={lookup}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={legal} onAction={noop} />
      </div>
    </CatalogContext.Provider>
  );
  cy.mount(board(initial)).then(({ rerender }) => {
    if (next !== undefined) cy.then(() => rerender(board(next)));
  });
}

describe("#3347 cards that move like cards, in a real browser", () => {
  before(() => {
    Cypress.config("defaultCommandTimeout", 20_000);
  });
  after(() => {
    Cypress.config("defaultCommandTimeout", 4_000);
  });
  afterEach(() => {
    setFxSettings({ ...DEFAULT_FX_SETTINGS });
  });

  for (const { width: w, height: h } of VIEWPORTS) {
    const size = `${String(w)}x${String(h)}`;

    it(`#3347 a card played from the hand flies to its tile at ${size}`, () => {
      cy.viewport(w, h);
      cy.then(slowly);
      const before = fullBoardView();
      mount(before, withEvents(before, played));
      cy.get(`${GAME} .fx-hold`).should("exist");
      cy.screenshot(`card-motion-flight-${size}`, { capture: "viewport", overwrite: true });
    });

    it(`#3347 the deck riffles when it is shuffled at ${size}`, () => {
      cy.viewport(w, h);
      cy.then(slowly);
      const before = fullBoardView();
      mount(before, withEvents(before, shuffled));
      cy.get(GAME).should("have.attr", "data-riffle", "you");
      cy.get('[data-testid="library-you"] .pile-riffle').should("have.css", "display", "block");
      cy.screenshot(`card-motion-riffle-${size}`, { capture: "viewport", overwrite: true });
    });

    it(`#3347 the opening hand is dealt a card at a time at ${size}`, () => {
      cy.viewport(w, h);
      cy.then(slowly);
      mount(fullBoardView({ phase: "mulligan" }));
      cy.get(GAME).should("have.attr", "data-dealing", "true");
      cy.get(GAME).should("have.attr", "data-riffle", "you opponent");
      cy.screenshot(`card-motion-deal-${size}`, { capture: "viewport", overwrite: true });
    });

    it(`#3347 a dragged card leans with its speed at ${size}`, () => {
      cy.viewport(w, h);
      // The settle timer would flatten the ghost before the capture: hold it, as a pointer that keeps moving does.
      cy.clock(Date.now(), ["setTimeout", "clearTimeout"]);
      mount(fullBoardView(), undefined, [PLAY]);
      const pointer = (type: "pointerdown" | "pointermove", x: number, y: number): Cypress.Chainable =>
        cy.window().then((win) => {
          const target = type === "pointerdown" ? win.document.querySelector(`[data-testid="hand-card-${HAND_CARD}"]`) : win;
          target?.dispatchEvent(
            new win.PointerEvent(type, { pointerId: 1, button: 0, clientX: x, clientY: y, bubbles: true, cancelable: true }),
          );
        });
      cy.get(`[data-testid="hand-card-${HAND_CARD}"]`).then(($card) => {
        const rect = $card[0]?.getBoundingClientRect();
        const x = (rect?.left ?? 0) + (rect?.width ?? 0) / 2;
        const y = (rect?.top ?? 0) + (rect?.height ?? 0) / 2;
        pointer("pointerdown", x, y);
        pointer("pointermove", x, y - 20);
        cy.wait(30);
        pointer("pointermove", x + 120, y - 60);
      });
      cy.get('[data-testid="drag-ghost"]').should(($ghost) => {
        const tilt = Number.parseFloat($ghost[0]?.style.getPropertyValue("--drag-tilt") ?? "0");
        expect(tilt, "the ghost leans").not.to.eq(0);
      });
      cy.screenshot(`card-motion-drag-${size}`, { capture: "viewport", overwrite: true });
    });
  }
});
