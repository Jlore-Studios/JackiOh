// R3407000: Stinkyscape part two's 23 signatures (apps/web/src/fx/cards/stinkyscape/, #3407) through the real
// board's effects layer, each played as the /dev/fx gallery plays it, at desktop size and on a phone: a
// Mythic's own takeover draws its card's own picture over the dark, the rest draw on the canvas or as the
// effects layer's DOM cues. apps/web's
// vitest suite proves the cues, their budgets and R202 (fx/cards/stinkyscape/*.test.ts, fx/cards/tiers.test.ts);
// what only a browser can say is that each scene draws, captured here.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-signature-stinkyscape.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { STINKYSCAPE_TWO_SIGNATURES } from "../../../apps/web/src/fx/cards/stinkyscape/signature.ts";
import { FX_CARD_TABLE } from "../../../apps/web/src/fx/cards/tables.ts";
import { FX_SETTINGS_KEY, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../../apps/web/src/fx/motifs/gallery.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { takeoverLookOf, TAKEOVER_ART_LOOKS } from "../../../apps/web/src/fx/takeover.ts";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;

const FX_DOM = '[data-testid="fx-dom"]';
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const GAME = '[data-testid="game"]';
const VIEWPORTS = [
  { width: 1280, height: 720 },
  { width: 390, height: 844 },
] as const;

/**
 * Mounts the board, then plays the scene over it, as fx-gallery-captures.cy.tsx does; `onShown` runs as the
 * scene is handed to the board, before its first frame.
 */
function mountShown(initial: PlayerView, shown: PlayerView, onShown: () => void = () => undefined): void {
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
    cy.then(() => {
      onShown();
      rerender(board(shown));
    });
  });
}

/** True once the effects canvas has drawn anything. */
function canvasLit(win: Window): boolean {
  const canvas = win.document.querySelector<HTMLCanvasElement>(CANVAS);
  if (canvas === null || canvas.width === 0 || canvas.height === 0) return false;
  const ctx = canvas.getContext("2d");
  if (ctx === null) return false;
  const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
  for (let i = 3; i < data.length; i += 4) {
    if ((data[i] ?? 0) > 0) return true;
  }
  return false;
}

/** How often the canvas is sampled, and for how long at most. */
const LIT_POLL_MS = 50;
const LIT_WAIT_MS = 15_000;

/**
 * Samples the effects layer from now on until it has drawn anything, on its canvas or as one of its DOM
 * cues (a figure, a sheen, a ghost), within LIT_WAIT_MS. Armed as the scene is handed to the board: a
 * signature whose canvas beats are brief and late (Paycheck's chips, the Wardrum Blob's drums) was not
 * always caught on the canvas in a run of many scenes, while its DOM beats were there.
 */
function watchLit(win: Window): PromiseLike<boolean> {
  return new Cypress.Promise<boolean>((resolve) => {
    const start = win.performance.now();
    const tick = (): void => {
      if (canvasLit(win) || win.document.querySelector(`${FX_DOM} [data-fx]`) !== null) {
        resolve(true);
        return;
      }
      if (win.performance.now() - start > LIT_WAIT_MS) resolve(false);
      else win.setTimeout(tick, LIT_POLL_MS);
    };
    tick();
  });
}

describe("R3407000 Stinkyscape part two's signatures in a real browser", () => {
  // Software rendering stretches commits and frames past the default command timeout; restored after.
  before(() => {
    Cypress.config("defaultCommandTimeout", 20_000);
  });
  after(() => {
    Cypress.config("defaultCommandTimeout", 4_000);
  });
  // The effects speed is the settings store's; put back after every test, a failed one included.
  afterEach(() => {
    setFxSettings({ ...DEFAULT_FX_SETTINGS });
    window.localStorage.removeItem(FX_SETTINGS_KEY);
  });

  for (const defId of STINKYSCAPE_TWO_SIGNATURES) {
    const name = CATALOG[defId]?.name ?? defId;
    const look = takeoverLookOf(defId);
    for (const { width: w, height: h } of VIEWPORTS) {
      it(`R3407000 ${name}'s signature at ${String(w)}x${String(h)}`, () => {
        cy.viewport(w, h);
        // At the slowest effects speed the signature outlasts the command queue, so the capture is mid-flight.
        cy.then(() => {
          setFxSettings({ speed: FX_SPEED_MIN });
        });
        const entry = FX_CARD_TABLE.get(defId);
        const play = entry === undefined ? null : galleryPlayOf(defId, entry);
        if (play === null) throw new Error(`${defId} has no gallery play`);
        const board = galleryBoardOf(defId, fullBoardView());
        let lit: PromiseLike<boolean> = Promise.resolve(false);
        cy.window().then((win) => {
          mountShown(board, withEvents(galleryFieldView(defId, board), galleryCardEvents(defId, play, board, 1)), () => {
            lit = watchLit(win);
          });
        });
        cy.get(GAME).find("[data-animating]").should("exist");
        if (TAKEOVER_ART_LOOKS.has(look)) {
          // A Mythic's own takeover, its card's own picture over the dark.
          cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", look);
          cy.get(`${FX_DOM} [data-fx="takeover"] img.fx-takeover-art`).should("have.attr", "src").and("match", /^data:image\/svg\+xml/);
        } else {
          cy.then({ timeout: 20_000 }, () => lit).then((drawn) => {
            expect(drawn, `${name}'s signature draws`).to.eq(true);
          });
        }
        cy.screenshot(`fx-signature-${defId}-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
      });
    }
  }
});
