// R3332001: Felinor Fiender's signature (apps/web/src/fx/cards/core/felinorFiender.ts) through the real
// board's effects layer, as the /dev/fx gallery plays it: over the gallery's board with its three
// Felinors, Felinor Fiender lands and its streaks of cat light draw on the canvas, at desktop size and on
// a phone. apps/web's vitest suite proves the cues (felinorFiender.test.ts) and their budget
// (fx/cards/tiers.test.ts); what only a browser can say is that the scene draws, captured here.
// R3400001: Meditative+'s and Training Grounds' eight signatures, each played as the gallery plays it, at
// both sizes: a Mythic's own takeover draws its art, the rest their figures or their canvas.
// R3402001: Core's ten other signatures (fx/cards/core/), each played as the gallery plays it at both
// sizes, its effects mounting on the page, and each Mythic's takeover drawing its own art (R3402000).
// R3405001: Meditative's 32 signatures the same way, at both sizes: each Mythic's own takeover draws its
// own art, every other card its figure.
// R3399000: so does every Classic+ signature, at both sizes, each Mythic's takeover drawing its own picture
// (R3399002) in its art layer.
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
/** R3400001: the eight signatures, each with its Mythic's own takeover look. */
const SIGNATURES: readonly { defId: string; name: string; look?: string }[] = [
  { defId: "meditativeplus-001", name: "Bob's Plan", look: "summit" },
  { defId: "meditativeplus-024", name: "Bob, Wisconsin's Strongest Soldier" },
  { defId: "meditativeplus-026", name: "Scratch Pad", look: "canvas" },
  { defId: "traininggrounds-037", name: "Seance" },
  { defId: "traininggrounds-038", name: "Tutorial Master" },
  { defId: "traininggrounds-039", name: "Call to Chaos (Training-Grounds Edition)" },
  { defId: "traininggrounds-039-1", name: "Training-Grounds Golem" },
  { defId: "traininggrounds-040", name: "Goetzmann Game Design", look: "napkin" },
];
/** R3405001: Meditative's 32 signatures, each with its Mythic's own takeover look. */
const MEDITATIVE: readonly { defId: string; name: string; look?: string }[] = [
  { defId: "meditative-007", name: "Introspection" },
  { defId: "meditative-008", name: "Reach the Summit", look: "ascent" },
  { defId: "meditative-011", name: "Double Header" },
  { defId: "meditative-017", name: "True Craft a Card", look: "workshop" },
  { defId: "meditative-020", name: "Aestheticize the Game", look: "brush" },
  { defId: "meditative-026", name: "Alternate Fate" },
  { defId: "meditative-039-4", name: "Red Jade", look: "mine" },
  { defId: "meditative-039-5", name: "Jade Beauty", look: "quarry" },
  { defId: "meditative-040", name: "Feng Shui" },
  { defId: "meditative-042", name: "CN Flea Market" },
  { defId: "meditative-045", name: "Knowledge Breaker" },
  { defId: "meditative-045-1", name: "Knowledge Breaker Prime" },
  { defId: "meditative-047", name: "Taotie" },
  { defId: "meditative-049", name: "YileGPT", look: "ai" },
  { defId: "meditative-049-1", name: "YileGPT Prime", look: "pavilion" },
  { defId: "meditative-049-2", name: "Yile's Virus", look: "virus" },
  { defId: "meditative-049-3", name: "AI Girlfriend", look: "dojo" },
  { defId: "meditative-051", name: "Devin Bot" },
  { defId: "meditative-063", name: "Skull of J'Nari" },
  { defId: "meditative-065", name: "Keymaster Keenus" },
  { defId: "meditative-071", name: "Pareto Optimality", look: "studyHall" },
  { defId: "meditative-071-1", name: "The Cane", look: "caneRack" },
  { defId: "meditative-075", name: "Ever Growing Tree" },
  { defId: "meditative-079", name: "Touched by KY" },
  { defId: "meditative-080", name: "Aluneth" },
  { defId: "meditative-086", name: "Mayor Medinamogger" },
  { defId: "meditative-087", name: "Tatches the Totem" },
  { defId: "meditative-095", name: "Call to Chaos (Meditative Edition)" },
  { defId: "meditative-095-1", name: "CN Golem" },
  { defId: "meditative-097", name: "Jlockeed's Evil Blueprints", look: "warRoom" },
  { defId: "meditative-097-8", name: "Prison" },
  { defId: "meditative-097-9", name: "Jlockeed's Headquarters", look: "fortress" },
];
const FX_DOM = '[data-testid="fx-dom"]';
/** R3402001: Core's ten signatures of issue #3402, in catalog order, and which of them are Mythics. */
const CORE_SIGNATURES = ["core-052", "core-083", "core-085", "core-087", "core-093", "core-095", "core-096", "core-097", "core-098", "core-099"] as const;
const CORE_MYTHICS: ReadonlySet<string> = new Set(["core-096", "core-097", "core-098", "core-099"]);
/** R3402000: a Core Mythic's own scene, a drawing in its takeover's art layer (R3399002). */
const TAKEOVER_ART = `${FX_DOM} [data-fx="takeover"] .takeover-art .takeover-art-drawing`;
/** How often and how long `watchScene` samples a scene. */
const SCENE_SAMPLE_MS = 50;
const SCENE_WATCH_MS = 15_000;
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

/** What a scene drew: its canvas, its effects on the page and, for a Mythic, its takeover's own art. */
type SceneSeen = { lit: boolean; dom: boolean; art: boolean };

/**
 * R3402001: watches a scene from before it plays, sampling every SCENE_SAMPLE_MS, until it has drawn on
 * the canvas or mounted an effect on the page and, for a Mythic, its takeover's own art has shown
 * (R3402000), or SCENE_WATCH_MS have passed. A takeover's art stands only through its hold, so it is
 * watched alongside rather than after. The canvas alone is no proof here: a frame a software-rendered
 * runner stalls past a burst's life ages its particles out before they draw (R200: no backlog replays),
 * while every one of the ten also mounts an effect of its own on the page (rays, a fracture, a ghost, a
 * sheen, its roll or its takeover).
 */
function watchScene(win: Window, mythic: boolean): Promise<SceneSeen> {
  return new Promise((resolve) => {
    const start = win.performance.now();
    const seen: SceneSeen = { lit: false, dom: false, art: !mythic };
    const tick = (): void => {
      seen.lit = seen.lit || canvasLit(win);
      // A turn banner is the board's, not the play's.
      seen.dom = seen.dom || win.document.querySelector(`${FX_DOM} [data-fx]:not([data-fx="banner"])`) !== null;
      seen.art = seen.art || win.document.querySelector(TAKEOVER_ART) !== null;
      if ((seen.lit && seen.dom && seen.art) || win.performance.now() - start > SCENE_WATCH_MS) resolve(seen);
      else win.setTimeout(tick, SCENE_SAMPLE_MS);
    };
    tick();
  });
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

describe("R3400001 Meditative+'s and Training Grounds' signatures in a real browser", () => {
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

  for (const { defId, name, look } of SIGNATURES) {
    for (const { width: w, height: h } of VIEWPORTS) {
      it(`R3400001 ${name}'s signature at ${String(w)}x${String(h)}`, () => {
        cy.viewport(w, h);
        // At the slowest effects speed the signature outlasts the command queue, so the capture is mid-flight.
        cy.then(() => {
          setFxSettings({ speed: FX_SPEED_MIN });
        });
        const entry = FX_CARD_TABLE.get(defId);
        const play = entry === undefined ? null : galleryPlayOf(defId, entry);
        if (play === null) throw new Error(`${defId} has no gallery play`);
        const board = galleryBoardOf(defId, fullBoardView());
        mountShown(board, withEvents(galleryFieldView(defId, board), galleryCardEvents(defId, play, board, 1)));
        cy.get(GAME).find("[data-animating]").should("exist");
        if (look !== undefined) {
          // A Mythic's own takeover, its art drawn over the dark.
          cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", look);
          cy.get(`${FX_DOM} [data-fx="takeover"] .takeover-art`).should("have.attr", "data-look", look);
        } else {
          // Its own figure: a snowplow, candles, a chalkboard, the dice, the Golem's plank.
          cy.get(`${FX_DOM} [data-fx="figure"]`).should("exist");
        }
        cy.screenshot(`fx-signature-${defId}-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
      });
    }
  }
});

describe("R3405001 Meditative's signatures in a real browser", () => {
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

  for (const { defId, name, look } of MEDITATIVE) {
    for (const { width: w, height: h } of VIEWPORTS) {
      it(`R3405001 ${name}'s signature at ${String(w)}x${String(h)}`, () => {
        cy.viewport(w, h);
        // At the slowest effects speed the signature outlasts the command queue, so the capture is mid-flight.
        cy.then(() => {
          setFxSettings({ speed: FX_SPEED_MIN });
        });
        const entry = FX_CARD_TABLE.get(defId);
        const play = entry === undefined ? null : galleryPlayOf(defId, entry);
        if (play === null) throw new Error(`${defId} has no gallery play`);
        const board = galleryBoardOf(defId, fullBoardView());
        mountShown(board, withEvents(galleryFieldView(defId, board), galleryCardEvents(defId, play, board, 1)));
        cy.get(GAME).find("[data-animating]").should("exist");
        if (look !== undefined) {
          // A Mythic's own takeover, its own art drawn over the dark.
          cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", look);
          cy.get(`${FX_DOM} [data-fx="takeover"] .takeover-art`).should("have.attr", "data-look", look);
        } else {
          // Its own figure: a still pool, floodlights, a compass, a porcelain mask, a key ring, a jade die.
          cy.get(`${FX_DOM} [data-fx="figure"]`).should("exist");
        }
        cy.screenshot(`fx-signature-${defId}-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
      });
    }
  }
});

describe("R3402001 R3402000 Core's signatures in a real browser, each Mythic's takeover drawing its own art", () => {
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

  for (const defId of CORE_SIGNATURES) {
    for (const { width: w, height: h } of VIEWPORTS) {
      it(`R3402001 ${defId}'s signature at ${String(w)}x${String(h)}`, () => {
        cy.viewport(w, h);
        cy.then(() => {
          setFxSettings({ speed: FX_SPEED_MIN });
        });
        const entry = FX_CARD_TABLE.get(defId);
        const play = entry === undefined ? null : galleryPlayOf(defId, entry);
        if (play === null) throw new Error(`${defId} has no gallery play`);
        const board = galleryBoardOf(defId, fullBoardView());
        const events = galleryCardEvents(defId, play, board, 1);
        const mythic = CORE_MYTHICS.has(defId);
        let seen: Promise<SceneSeen> = Promise.resolve({ lit: false, dom: false, art: false });
        cy.window().then((win) => {
          seen = watchScene(win, mythic);
        });
        // As /dev/fx plays it: over the board with the card still in the hand, where a Spell's takeover lands.
        mountShown(galleryHandOf(defId, board, 1), withEvents(galleryFieldView(defId, board), events));
        cy.get(GAME).find("[data-animating]").should("exist");
        cy.then({ timeout: SCENE_WATCH_MS + 5_000 }, () => seen).then((result) => {
          cy.log(`${defId}: canvas ${result.lit ? "drew" : "stayed dark"}`);
          expect(result.dom, `${defId}'s signature mounts its effects`).to.eq(true);
          // R3402000: a Mythic's takeover draws its own scene over the dark.
          expect(result.art, `${defId}'s takeover draws its own art`).to.eq(true);
        });
        cy.screenshot(`fx-signature-${defId}-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
        // The scene plays out before the next mounts, so nothing of it runs on into the next test.
        cy.get("[data-animating]", { timeout: SCENE_WATCH_MS * 2 }).should("not.exist");
      });
    }
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
        // R3399002: a Mythic's takeover wears its own look and draws its own picture in its art layer.
        if (look !== undefined) cy.get(`${FX_DOM} [data-fx="takeover"][data-look="${look}"] .takeover-art[data-look="${look}"] .takeover-art-drawing`).should("exist");
        drawsSomething().then((drawn) => {
          expect(drawn, `${defId}'s signature draws`).to.eq(true);
        });
        cy.screenshot(`fx-signature-${defId}-${String(w)}x${String(h)}`, { capture: "viewport", overwrite: true });
      });
    }
  }
});
