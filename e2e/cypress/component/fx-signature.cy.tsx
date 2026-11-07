// R3332001: Felinor Fiender's signature (apps/web/src/fx/cards/core/felinorFiender.ts) through the real
// board's effects layer, as the /dev/fx gallery plays it: over the gallery's board with its three
// Felinors, Felinor Fiender lands and its streaks of cat light draw on the canvas, at desktop size and on
// a phone. apps/web's vitest suite proves the cues (felinorFiender.test.ts) and their budget
// (fx/cards/tiers.test.ts); what only a browser can say is that the scene draws, captured here.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-signature.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { GALLERY_FELINORS, GALLERY_FELINOR_LANES } from "../../../apps/web/src/fx/cards/core/felinorFiender.ts";
import { FX_CARD_TABLE } from "../../../apps/web/src/fx/cards/tables.ts";
import { FX_SETTINGS_KEY, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../../apps/web/src/fx/motifs/gallery.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;

const FIENDER = "core-092";
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const GAME = '[data-testid="game"]';
const VIEWPORTS = [
  { width: 1280, height: 720 },
  { width: 390, height: 844 },
] as const;

/** Mounts the board, then plays the scene over it, as fx-gallery-captures.cy.tsx does. */
function mountShown(initial: PlayerView, shown: PlayerView): void {
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
    cy.then(() => rerender(board(shown)));
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

/** Samples frames until the canvas has drawn anything (fx-gallery-captures.cy.tsx's). */
function waitLit(): Cypress.Chainable<boolean> {
  return cy.window({ timeout: 20_000 }).then(
    (win) =>
      new Cypress.Promise<boolean>((resolve) => {
        const start = win.performance.now();
        const tick = (): void => {
          if (canvasLit(win)) {
            resolve(true);
            return;
          }
          if (win.performance.now() - start > 15_000) resolve(false);
          else win.requestAnimationFrame(tick);
        };
        win.requestAnimationFrame(tick);
      }),
  );
}

describe("R3332001 Felinor Fiender's signature in a real browser", () => {
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

  for (const { width: w, height: h } of VIEWPORTS) {
    it(`R3332001 Felinor Fiender's signature at ${String(w)}x${String(h)}`, () => {
      cy.viewport(w, h);
      // At the slowest effects speed the streaks outlast the command queue, so the capture is mid-flight.
      cy.then(() => {
        setFxSettings({ speed: FX_SPEED_MIN });
      });
      const entry = FX_CARD_TABLE.get(FIENDER);
      const play = entry === undefined ? null : galleryPlayOf(FIENDER, entry);
      if (play === null) throw new Error("Felinor Fiender has no gallery play");
      expect(play).to.deep.eq({ kind: "entrance", key: "felinorAssembly" });
      const board = galleryBoardOf(FIENDER, fullBoardView());
      const events = galleryCardEvents(FIENDER, play, board, 1);
      mountShown(board, withEvents(galleryFieldView(FIENDER, board), events));
      // The board it assembles from: a Felinor face up in each of the gallery's lanes.
      GALLERY_FELINOR_LANES.forEach((lane, k) => {
        cy.get(`[data-testid="zone-you-units-${String(lane)}"] [data-def-id="${GALLERY_FELINORS[k] ?? ""}"]`).should("exist");
      });
      cy.get(GAME).find("[data-animating]").should("exist");
      waitLit().then((lit) => {
        expect(lit, "the assembly draws on the canvas").to.eq(true);
      });
      cy.screenshot(`fx-signature-felinor-fiender-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
    });
  }
});
