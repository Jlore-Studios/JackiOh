// R3395000: Stinkyscape 001 to 090's 24 signatures (apps/web/src/fx/cards/stinkyscape/) through the real
// board's effects layer, as the /dev/fx gallery plays them: each card over its gallery board, its entrance or
// its own cast and its text's beat drawing on the canvas, at desktop size and on a phone. apps/web's vitest
// suite proves the cues (fx/cards/stinkyscape/signatures.test.ts) and their budget (fx/cards/tiers.test.ts);
// what only a browser can say is that each scene draws, and (R3395001) a Mythic's takeover its own art,
// captured here. Every
// scene is checked to draw before any is captured: once a spec has taken a screenshot, the page's frames
// run about once a second for the rest of it, and a burst due after a takeover's hold would fire already spent.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-signature-stinkyscape-1.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { FX_CARD_TABLE, isSignature } from "../../../apps/web/src/fx/cards/tables.ts";
import { FX_SETTINGS_KEY, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { takeoverLookOf } from "../../../apps/web/src/fx/takeover.ts";
import { galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../../apps/web/src/fx/motifs/gallery.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;

/** Every signature of Stinkyscape 001 to 090, in catalog order. */
const STINKYSCAPE_SIGNATURES = [...FX_CARD_TABLE].flatMap(([defId, entry]) => (/^stinkyscape-0\d\d$/.test(defId) && isSignature(entry) ? [defId] : []));
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const GAME = '[data-testid="game"]';
const FX_DOM = '[data-testid="fx-dom"]';
/**
 * How often the canvas is read for a lit pixel. Reading every frame slows the frames enough that the
 * director's adaptive quality sheds the particles of a scene whose first ones land late (a Mythic's,
 * after its takeover's hold), so the reader would starve what it reads.
 */
const LIT_POLL_MS = 200;
/** How long a scene may take to draw anything, its takeover's hold at the slowest speed included. */
const LIT_WAIT_MS = 15_000;
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

/** Reads the canvas every LIT_POLL_MS until it has drawn anything, at most LIT_WAIT_MS. */
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
          if (win.performance.now() - start > LIT_WAIT_MS) resolve(false);
          else win.setTimeout(tick, LIT_POLL_MS);
        };
        tick();
      }),
  );
}

describe("R3395000 Stinkyscape 001 to 090's signatures in a real browser", () => {
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

  it("R3395000 Stinkyscape 001 to 090 has 24 signatures to capture", () => {
    expect(STINKYSCAPE_SIGNATURES).to.have.length(24);
  });

  // Each play is new to the board, as the gallery's n-th play is (`galleryPlayId`).
  let plays = 0;
  /** Plays `defId`'s gallery play at `w`x`h`, at the slowest effects speed, and waits for it to be in flight. */
  const playOut = (defId: string, name: string, w: number, h: number): void => {
    cy.viewport(w, h);
    // At the slowest effects speed the scene outlasts the command queue, so a capture is mid-flight.
    cy.then(() => {
      setFxSettings({ speed: FX_SPEED_MIN });
    });
    const entry = FX_CARD_TABLE.get(defId);
    const play = entry === undefined ? null : galleryPlayOf(defId, entry);
    if (play === null) throw new Error(`${name} has no gallery play`);
    // Its own recipe, or (Paragon Mk2's, Paragon-X's) its entrance alone.
    expect(["recipe", "entrance"]).to.include(play.kind);
    const board = galleryBoardOf(defId, fullBoardView());
    plays += 1;
    mountShown(board, withEvents(galleryFieldView(defId, board), galleryCardEvents(defId, play, board, plays)));
    cy.get(GAME).find("[data-animating]").should("exist");
    // R3395001: a Mythic's takeover wears its own look and draws its own art over the dark.
    if (CATALOG[defId]?.rarity === "Mythic") {
      cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", takeoverLookOf(defId));
      cy.get(`${FX_DOM} [data-fx="takeover"] .takeover-art`).should("have.attr", "data-look", takeoverLookOf(defId));
    }
  };

  // Every scene draws, each checked before the first capture.
  for (const defId of STINKYSCAPE_SIGNATURES) {
    const name = CATALOG[defId]?.name ?? defId;
    for (const { width: w, height: h } of VIEWPORTS) {
      it(`R3395000 ${name}'s signature draws at ${String(w)}x${String(h)}`, () => {
        playOut(defId, name, w, h);
        waitLit().then((lit) => {
          expect(lit, `${name}'s signature draws on the canvas`).to.eq(true);
        });
      });
    }
  }

  // Then each is captured mid-flight.
  for (const defId of STINKYSCAPE_SIGNATURES) {
    const name = CATALOG[defId]?.name ?? defId;
    for (const { width: w, height: h } of VIEWPORTS) {
      it(`R3395000 ${name}'s signature at ${String(w)}x${String(h)}`, () => {
        playOut(defId, name, w, h);
        cy.screenshot(`fx-signature-${defId}-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
      });
    }
  }
});
