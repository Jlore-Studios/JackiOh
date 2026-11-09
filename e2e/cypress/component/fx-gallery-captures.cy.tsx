// R2803000: the audit's gallery capture spec. Through the real board's effects layer, every drawn
// motif renders mid-flight, and every set piece renders mid-flight and at rest.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-gallery-captures.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { FX_CARD_TABLE } from "../../../apps/web/src/fx/cards/tables.ts";
import { FX_SETTINGS_KEY, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { FX_DRAWN_MOTIFS, type DrawnMotif } from "../../../apps/web/src/fx/motifs/names.ts";
import {
  galleryCardEvents,
  galleryFieldView,
  galleryPlayOf,
  type GalleryCardPlay,
} from "../../../apps/web/src/fx/motifs/gallery.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import {
  baseView,
  card,
  dormantStarship,
  emptySide,
  fullBoardView,
  unit,
  withEvents,
} from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';
const BOARD = '[data-testid="board"]';

/** The first card whose gallery play draws `motif` on its play, else on its hook. */
function cardForMotif(motif: DrawnMotif): { defId: string; play: GalleryCardPlay } {
  const hook: { defId: string; play: GalleryCardPlay }[] = [];
  for (const [defId, entry] of FX_CARD_TABLE) {
    const play = galleryPlayOf(defId, entry);
    if (play?.kind === "board" && play.motif === motif) return { defId, play };
    if (play?.kind === "hook" && play.motif === motif) hook.push({ defId, play });
  }
  const found = hook[0];
  if (found === undefined) throw new Error(`no gallery card draws ${motif}`);
  return found;
}

/**
 * Mounts the board, then plays the scene over it, as fx-layer.cy.tsx does. `before` runs on the page's
 * window once the effects layer is on and just before the scene plays: where a test starts watching.
 */
function mountShown(initial: PlayerView, shown: PlayerView, before?: (win: Cypress.AUTWindow) => void): void {
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
    if (before !== undefined) cy.window().then(before);
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
  // Every pixel's alpha: small single-target effects (a mend, a tithe) light only a few pixels.
  for (let i = 3; i < data.length; i += 4) {
    if ((data[i] ?? 0) > 0) return true;
  }
  return false;
}

/**
 * Samples frames until the canvas has drawn anything. The window outlasts slow software
 * frames; the director repaints while cues live and clears right after, so a mid-flight
 * effect always lights at least one sampled frame.
 */
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

/** Where `watchGhosts` keeps its count on the page's window. */
const GHOSTS_SEEN = "__fxGhostsSeen";

type GhostWatch = { count: number; watch: MutationObserver };

/**
 * Counts, in the page itself, every `.fx-ghost` added to the document from now on. A ghost lives for a
 * moment even at 0.25x, and on a loaded runner the page stalls for longer than that between two of
 * Cypress's polls, so a `get` of one can miss every ghost the director ever mounted; a mutation
 * record keeps the node it saw added after the node is gone.
 */
function watchGhosts(win: Cypress.AUTWindow): void {
  (Reflect.get(win, GHOSTS_SEEN) as GhostWatch | undefined)?.watch.disconnect();
  const seen: GhostWatch = {
    count: 0,
    watch: new win.MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) {
          if (!(node instanceof win.Element)) continue;
          seen.count += (node.matches(".fx-ghost") ? 1 : 0) + node.querySelectorAll(".fx-ghost").length;
        }
      }
    }),
  };
  seen.watch.observe(win.document.body, { childList: true, subtree: true });
  Reflect.set(win, GHOSTS_SEEN, seen);
}

/** How many ghosts `watchGhosts` has seen added. */
function ghostsSeen(win: Cypress.AUTWindow): number {
  return (Reflect.get(win, GHOSTS_SEEN) as GhostWatch | undefined)?.count ?? 0;
}

describe("R2803000 the gallery captures every motif mid-flight", () => {
  // Software rendering stretches commits and frames past the default command timeout;
  // restored after so later specs keep theirs.
  before(() => {
    Cypress.config("defaultCommandTimeout", 20_000);
  });
  after(() => {
    Cypress.config("defaultCommandTimeout", 4_000);
  });
  // The effects speed `shift` sets is the settings store's; put back after every test, a failed one included.
  afterEach(() => {
    setFxSettings({ ...DEFAULT_FX_SETTINGS });
    window.localStorage.removeItem(FX_SETTINGS_KEY);
  });
  // R2803000: `shift` proves its own visual. Its entry plans 20 ghosts + 20 canvas bursts
  // (unit), the director fires all of them in the browser (20 ghosts mount), but the canvas
  // stays dark there: firing 40 cues in one tick forces a full-board reflow per cue in software
  // rendering, stalling the frame loop ~700 ms, so the short-lived bursts age out before a frame
  // paints them (measured with rAF + canvas-pixel + director-play/fire timelines in headless
  // Chromium; see the audit report §7). The ghosts are the motif's signature — every moved
  // card's path walked in the DOM (board.ts) — so `shift` asserts them, at the slowest effects
  // speed (FX_SPEED_MIN) so their window outlasts the command queue, and through `watchGhosts` so
  // a runner that stalls the page past even that window still counts the ghosts it mounted; the
  // tier test proves every cue's timing. The speed is set through the store (`setFxSettings`):
  // `getFxSettings()` keeps its first read in memory, so a value written to storage after an
  // earlier test had loaded it was never read. The ghosts kept their full-speed 200 ms, mounting
  // all twenty outlasted it, and a frame removed them unseen.
  for (const motif of FX_DRAWN_MOTIFS) {
    if (motif === "shift") {
      it(`R2803000 shift renders mid-flight`, () => {
        cy.viewport(1280, 720);
        cy.then(() => {
          setFxSettings({ speed: FX_SPEED_MIN });
        });
        const { defId, play } = cardForMotif(motif);
        const view = fullBoardView();
        const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
        const staged: PlayerView = {
          ...view,
          you: { ...view.you, hand: [...hand, card({ instanceId: "fx-gallery-card-1", defId })] },
        };
        const events = galleryCardEvents(defId, play, view, 1);
        mountShown(view, withEvents(galleryFieldView(defId, staged), events), watchGhosts);
        cy.window().should((win) => {
          expect(ghostsSeen(win), "the ghosts of the moved cards mount").to.be.greaterThan(0);
        });
        cy.screenshot(`gallery-motif-shift-mid`, { capture: "viewport", overwrite: true });
      });
      continue;
    }
    it(`R2803000 ${motif} renders mid-flight`, () => {
      cy.viewport(1280, 720);
      const { defId, play } = cardForMotif(motif);
      const view = fullBoardView();
      // The motif's canvas cues anchor to the casting card, so it stands in the hand as played.
      const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
      const staged: PlayerView = {
        ...view,
        you: { ...view.you, hand: [...hand, card({ instanceId: "fx-gallery-card-1", defId })] },
      };
      const events = galleryCardEvents(defId, play, view, 1);
      mountShown(view, withEvents(galleryFieldView(defId, staged), events));
      waitLit().then((lit) => {
        expect(lit, `${motif} by ${defId} draws`).to.eq(true);
      });
      cy.screenshot(`gallery-motif-${motif}-mid`, { capture: "viewport", overwrite: true });
    });
  }
});

describe("R2803000 the gallery captures every set piece mid-flight and at rest", () => {
  /** Every Unit on the board, as the removals of `type` a wipe streams. */
  function removals(view: PlayerView, type: "destroyed" | "exiled"): GameEvent[] {
    return [...view.you.units, ...view.opponent.units].flatMap((u): GameEvent[] => {
      if (u === null) return [];
      return type === "destroyed"
        ? [{ type, instanceId: u.instanceId, defId: u.defId, owner: u.owner, controller: u.controller, attack: u.attack, maxHealth: u.maxHealth, killerId: null }]
        : [{ type, instanceId: u.instanceId, defId: u.defId, owner: u.owner }];
    });
  }

  function spellClear(key: string, defId: string): { view: PlayerView; events: GameEvent[] } {
    const view = fullBoardView();
    const me = view.viewer;
    return {
      view,
      events: [
        { type: "cardPlayed", player: me, instanceId: "fx-clear", defId, costPaid: 4 },
        { type: "hookResolving", sourceId: "fx-clear", defId, controller: me, hook: "cry" },
        ...removals(view, "destroyed"),
        { type: "cardResolved", player: me, instanceId: "fx-clear", defId, permanent: false, costPaid: 4 },
      ],
    };
  }

  function voidClear(): { view: PlayerView; events: GameEvent[] } {
    const full = fullBoardView();
    const view: PlayerView = { ...full, you: { ...full.you, units: full.you.units.map((u, i) => (i === 2 ? null : u)) } };
    const me = view.viewer;
    return {
      view,
      events: [
        { type: "cardPlayed", player: me, instanceId: "fx-void", defId: "core-100", costPaid: 0 },
        { type: "summoned", player: me, instanceId: "fx-void", defId: "core-100", row: "units", lane: 3 },
        { type: "hookResolving", sourceId: "fx-void", defId: "core-100", controller: me, hook: "cry" },
        ...removals(view, "exiled"),
        { type: "cardResolved", player: me, instanceId: "fx-void", defId: "core-100", permanent: true, costPaid: 0 },
      ],
    };
  }

  function launchScene(): { before: PlayerView; after: PlayerView; events: GameEvent[] } {
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
    return {
      before: ship(0),
      after: ship(1),
      events: [
        { type: "awakened", player: "p1", instanceId: "fx-ship", defId: HULL },
        { type: "starshipLaunched", player: "p1", instanceId: "fx-ship", defId: HULL, name: "Ember of Departure", pieces: fused },
      ],
    };
  }

  const pieces: readonly (readonly [string, string, () => { view: PlayerView; events: GameEvent[] }])[] = [
    ["nether", "vortex", () => spellClear("nether", "core-088")],
    ["voidFold", "fold", voidClear],
    ["vortex", "vortex", () => spellClear("vortex", "classic-043")],
  ];

  for (const [key, shape, make] of pieces) {
    it(`R2803000 ${key} mid-flight and at rest`, () => {
      cy.viewport(1280, 720);
      const { view, events } = make();
      mountShown(view, withEvents(view, events));
      // Mid-flight: the queue holds the wipe, and its cue stands over the board.
      cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", key);
      cy.get(`${FX_DOM} [data-fx="setPiece"]`).should("have.attr", "data-shape", shape);
      cy.get(`${BOARD} [data-fx-pulled]`).should("have.length.greaterThan", 0);
      cy.screenshot(`gallery-set-piece-${key}-mid`, { capture: "viewport", overwrite: true });
      // At rest: nothing animating, the hold over, and every layer of the set piece gone.
      cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
      cy.get(GAME).should("not.have.attr", "data-set-piece");
      cy.get(`${FX_DOM} [data-fx="setPiece"]`, { timeout: 4_000 }).should("not.exist");
      cy.screenshot(`gallery-set-piece-${key}-rest`, { capture: "viewport", overwrite: true });
    });
  }

  it("R2803000 starship launch mid-flight and at rest", () => {
    cy.viewport(1280, 720);
    const { before, after, events } = launchScene();
    mountShown(before, withEvents(after, events));
    cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "starship");
    // R3334001: the launch's 3-2-1 countdown is gone; its flame lights the ship's tile.
    cy.get(`${FX_DOM} [data-fx="launch"] .fx-launch-flame`).should("have.length", 1);
    cy.get(`${FX_DOM} [data-fx="launch"] .fx-launch-count`).should("not.exist");
    cy.screenshot("gallery-set-piece-starship-mid", { capture: "viewport", overwrite: true });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
    cy.get(GAME).should("not.have.attr", "data-set-piece");
    cy.get(`${FX_DOM} [data-fx="launch"]`, { timeout: 4_000 }).should("not.exist");
    cy.screenshot("gallery-set-piece-starship-rest", { capture: "viewport", overwrite: true });
  });
});
