// R3332001: Felinor Fiender's signature (apps/web/src/fx/cards/core/felinorFiender.ts) through the real
// board's effects layer, as the /dev/fx gallery plays it: over the gallery's board with its three
// Felinors, Felinor Fiender lands and its streaks of cat light draw on the canvas, at desktop size and on
// a phone. apps/web's vitest suite proves the cues (felinorFiender.test.ts) and their budget
// (fx/cards/tiers.test.ts); what only a browser can say is that the scene draws, captured here. R3399000:
// so does every Classic+ signature, each Mythic's takeover drawing its own picture (R3399002).
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-signature.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { GALLERY_FELINORS, GALLERY_FELINOR_LANES } from "../../../apps/web/src/fx/cards/core/felinorFiender.ts";
import { FX_CARD_TABLE } from "../../../apps/web/src/fx/cards/tables.ts";
import { FX_SETTINGS_KEY, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryHandOf, galleryPlayOf } from "../../../apps/web/src/fx/motifs/gallery.ts";
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

/** R3399000: every Classic+ signature, in table order, with its Mythic's own takeover look. */
const CLASSIC_PLUS = [...FX_CARD_TABLE].flatMap(([defId, entry]) =>
  defId.startsWith("classicplus-") && entry.kind !== "none" && entry.signature === true ? [{ defId, entry, look: entry.takeover }] : [],
);

/** True once the effects canvas has drawn or the DOM layer holds a cue (a ghost, a sheen, a takeover). */
function drawsSomething(): Cypress.Chainable<boolean> {
  return cy.window({ timeout: 20_000 }).then(
    (win) =>
      new Cypress.Promise<boolean>((resolve) => {
        const start = win.performance.now();
        const tick = (): void => {
          const dom = win.document.querySelector('[data-testid="fx-dom"]');
          if (canvasLit(win) || (dom !== null && dom.childElementCount > 0)) {
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

describe("R3399000 Classic+'s signatures in a real browser", () => {
  before(() => {
    Cypress.config("defaultCommandTimeout", 20_000);
  });
  after(() => {
    Cypress.config("defaultCommandTimeout", 4_000);
  });
  afterEach(() => {
    setFxSettings({ ...DEFAULT_FX_SETTINGS });
    window.localStorage.removeItem(FX_SETTINGS_KEY);
  });

  it("R3399000 the spec plays all 35 of Classic+'s Legendaries and Mythics", () => {
    expect(CLASSIC_PLUS).to.have.length(35);
  });

  for (const { defId, entry, look } of CLASSIC_PLUS) {
    for (const { width: w, height: h } of VIEWPORTS) {
      it(`R3399000 ${defId}'s signature at ${String(w)}x${String(h)}`, () => {
        cy.viewport(w, h);
        cy.then(() => {
          setFxSettings({ speed: FX_SPEED_MIN });
        });
        const play = galleryPlayOf(defId, entry);
        if (play === null) throw new Error(`${defId} has no gallery play`);
        const board = galleryBoardOf(defId, fullBoardView());
        const events = galleryCardEvents(defId, play, board, 1);
        // As a real play starts: the card in its player's hand, so its cast and a Mythic's takeover rise from it.
        mountShown(galleryHandOf(defId, board, 1), withEvents(galleryFieldView(defId, board), events));
        cy.get(GAME).find("[data-animating]").should("exist");
        // R3399002: a Mythic's takeover wears its own look and draws its own picture.
        if (look !== undefined) cy.get(`[data-testid="fx-dom"] [data-fx="takeover"][data-look="${look}"] .fx-takeover-art`).should("exist");
        drawsSomething().then((drawn) => {
          expect(drawn, `${defId}'s signature draws`).to.eq(true);
        });
        cy.screenshot(`fx-signature-${defId}-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
      });
    }
  }
});
