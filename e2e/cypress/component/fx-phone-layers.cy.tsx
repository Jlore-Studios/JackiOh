// R4493000, R4493001 (issue #4493): the phone scenes' composited layers, measured in a real browser at 390x844 and 3x
// density, the per-pull-request estimate's twin (apps/web/src/fx/layerBudget.test.ts, from fx.css alone). Each scene of
// R2803000's phone set (Twisting Nether, a Mythic takeover, a takeover through Ceaseless Void's diorama, Ceaseless Void's
// fold, the Starship launch, the Drone Battleship's arrival, the Anomaly cast) and The Big Crunch's collapse plays
// through the board's effects layer while every frame reads each part of the effects' DOM a browser holds a layer for:
// every element and pseudo-element an animation runs on (`document.getAnimations()`) and every element with
// `will-change`, its box times its scale that frame, squared, times the device's pixels. No part may cover more than
// FX_LAYER_MAX_SCREENS at its largest, no frame may hold more than FX_PHONE_LAYER_BUDGET_MB, and the diorama's stage
// draws at most FX_PHONE_STAGE_LAYERS planes. Chrome's device metrics are overridden over the debugger protocol for the
// density, as fx-phone-living-game.cy.tsx throttles its CPU.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-phone-layers.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { FX_CARD_TABLE } from "../../../apps/web/src/fx/cards/tables.ts";
import {
  FX_BYTES_PER_MB,
  FX_LAYER_BYTES_PER_PX,
  FX_LAYER_MAX_SCREENS,
  FX_PHONE_LAYER_BUDGET_MB,
  FX_PHONE_STAGE_LAYERS,
  FX_SETTINGS_KEY,
} from "../../../apps/web/src/fx/constants.ts";
import { galleryCardEvents } from "../../../apps/web/src/fx/motifs/gallery.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { BIG_CRUNCH } from "../../../apps/web/src/fx/terminus/ids.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, dormantStarship, emptySide, fullBoardView, unit, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const PHONE = { width: 390, height: 844 };
/** An iPhone's density, which the layers are held at. */
const DPR = 3;
const LAYER = '[data-testid="fx-layer"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';
/** The longest a scene is read for: past its set piece's hold and its tail. */
const SAMPLE_MAX_MS = 8_000;
/** How long Cypress waits on the frames' read: the whole sample and a margin for the last frame. */
const READ_TIMEOUT_MS = SAMPLE_MAX_MS + 4_000;
/** How many of the fullest frame's parts a scene's log names, largest first. */
const PEAK_PARTS_SHOWN = 6;

/** What the frames read of a scene: each part's largest cover in screens, the fullest frame's bytes and its parts, and the most planes a stage drew. */
type Layers = { parts: { label: string; screens: number }[]; peakBytes: number; peakParts: { label: string; screens: number }[]; planes: number };

/** A part's scale this frame: its transform's largest column times its `scale` property's largest factor. */
function scaleOf(win: Cypress.AUTWindow, style: CSSStyleDeclaration): number {
  const matrix = style.transform === "none" || style.transform === "" ? new win.DOMMatrix() : new win.DOMMatrix(style.transform);
  const transform = Math.max(Math.hypot(matrix.a, matrix.b), Math.hypot(matrix.c, matrix.d));
  const own = style.scale === "none" || style.scale === "" ? [1] : style.scale.split(/\s+/).map((v) => Math.abs(Number(v)));
  return transform * Math.max(...own);
}

/** A part's border box, CSS px: an element's offset box (an svg's client box), a pseudo-element's used size with its edges. */
function areaOf(win: Cypress.AUTWindow, el: Element, pseudo: string): number {
  if (pseudo === "") {
    if (el instanceof win.HTMLElement) return el.offsetWidth * el.offsetHeight;
    return el.clientWidth * el.clientHeight;
  }
  const style = win.getComputedStyle(el, pseudo);
  const px = (value: string): number => Number.parseFloat(value) || 0;
  const edges = style.boxSizing === "border-box" ? [0, 0] : [px(style.paddingLeft) + px(style.paddingRight) + px(style.borderLeftWidth) + px(style.borderRightWidth), px(style.paddingTop) + px(style.paddingBottom) + px(style.borderTopWidth) + px(style.borderBottomWidth)];
  return (px(style.width) + (edges[0] ?? 0)) * (px(style.height) + (edges[1] ?? 0));
}

/** Reads the effects' layers on every frame until the layer has drawn and emptied again, or SAMPLE_MAX_MS. */
function measureLayers(): Cypress.Chainable<Layers> {
  return cy.window().then(
    { timeout: READ_TIMEOUT_MS },
    (win) =>
      new Cypress.Promise<Layers>((resolve) => {
        const doc = win.document;
        const screen = win.innerWidth * win.innerHeight;
        const most = new Map<string, number>();
        const labels = new Map<Element, number>();
        let peak = 0;
        let peakParts: { label: string; screens: number }[] = [];
        let planes = 0;
        let drawn = false;
        const start = win.performance.now();
        const tick = (): void => {
          const root = doc.querySelector(FX_DOM);
          const parts = new Map<string, { el: Element; pseudo: string }>();
          const add = (el: Element, pseudo: string): void => {
            if (root === null || !root.contains(el) || (el instanceof win.SVGElement && el.ownerSVGElement !== null)) return;
            if (!labels.has(el)) labels.set(el, labels.size);
            parts.set(`${String(labels.get(el))}${pseudo}`, { el, pseudo });
          };
          for (const animation of doc.getAnimations()) {
            const effect = animation.effect;
            if (effect instanceof win.KeyframeEffect && effect.target !== null) add(effect.target, effect.pseudoElement ?? "");
          }
          for (const el of root?.querySelectorAll("*") ?? []) if (win.getComputedStyle(el).willChange !== "auto") add(el, "");
          let frame = 0;
          const read: { label: string; screens: number }[] = [];
          for (const [key, { el, pseudo }] of parts) {
            const style = win.getComputedStyle(el, pseudo === "" ? null : pseudo);
            const scale = scaleOf(win, style);
            const covered = (areaOf(win, el, pseudo) * scale * scale) / screen;
            frame += covered;
            const label = `${el.localName}.${[...el.classList].join(".")}${pseudo} #${key}`;
            most.set(label, Math.max(most.get(label) ?? 0, covered));
            read.push({ label, screens: covered });
          }
          const bytes = frame * screen * DPR * DPR * FX_LAYER_BYTES_PER_PX;
          if (bytes > peak) {
            peak = bytes;
            peakParts = read;
          }
          planes = Math.max(planes, root?.querySelectorAll(".fx-takeover-plane").length ?? 0);
          const busy = (root?.childElementCount ?? 0) > 0;
          drawn ||= busy;
          if ((drawn && !busy) || win.performance.now() - start >= SAMPLE_MAX_MS) {
            resolve({ parts: [...most].map(([label, screens]) => ({ label, screens })), peakBytes: peak, peakParts, planes });
            return;
          }
          win.requestAnimationFrame(tick);
        };
        win.requestAnimationFrame(tick);
      }),
  );
}

/** A scene's layers held to the budget: every part within FX_LAYER_MAX_SCREENS, every frame within FX_PHONE_LAYER_BUDGET_MB. */
function holdsLayers(label: string, layers: Layers): void {
  const largest = [...layers.peakParts]
    .sort((a, b) => b.screens - a.screens)
    .slice(0, PEAK_PARTS_SHOWN)
    .map((part) => `${part.label} ${part.screens.toFixed(2)}`)
    .join(", ");
  const line = `${label}: ${(layers.peakBytes / FX_BYTES_PER_MB).toFixed(1)} MB at its fullest frame over ${String(layers.parts.length)} parts (largest, in screens: ${largest})`;
  cy.log(line);
  Cypress.log({ name: "layers", message: line });
  expect(layers.parts.length, `${label}: parts read`).to.be.greaterThan(0);
  for (const part of layers.parts) expect(part.screens, `${label}: ${part.label}`).to.be.at.most(FX_LAYER_MAX_SCREENS);
  expect(layers.peakBytes, line).to.be.at.most(FX_PHONE_LAYER_BUDGET_MB * FX_BYTES_PER_MB);
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

/** A Mythic played and summoned by the viewer. */
function mythic(view: PlayerView, defId: string, lane: number): GameEvent[] {
  return [
    { type: "cardPlayed", player: view.viewer, instanceId: "fx-mythic", defId, costPaid: 2 },
    { type: "summoned", player: view.viewer, instanceId: "fx-mythic", defId, row: "units", lane },
  ];
}

describe("R4493000 R4493001 the phone scenes hold the layer budget at 3x", () => {
  beforeEach(() => {
    cy.viewport(PHONE.width, PHONE.height);
    cy.then(() =>
      Cypress.automation("remote:debugger:protocol", {
        command: "Emulation.setDeviceMetricsOverride",
        params: { width: 0, height: 0, deviceScaleFactor: DPR, mobile: false },
      }),
    );
    cy.window().then((win) => {
      expect(win.devicePixelRatio).to.eq(DPR);
      expect(win.innerWidth).to.eq(PHONE.width);
    });
  });

  // The density goes back for the next spec, and the effects settings a scene may have changed with it.
  afterEach(() => {
    cy.then(() => Cypress.automation("remote:debugger:protocol", { command: "Emulation.clearDeviceMetricsOverride", params: {} }));
    setFxSettings({ ...DEFAULT_FX_SETTINGS });
    window.localStorage.removeItem(FX_SETTINGS_KEY);
  });

  it("R4493001 Twisting Nether holds the layer budget at 3x", () => {
    const view = fullBoardView();
    const me = view.viewer;
    mountScene(view, view, [
      { type: "cardPlayed", player: me, instanceId: "fx-clear", defId: "core-088", costPaid: 4 },
      { type: "hookResolving", sourceId: "fx-clear", defId: "core-088", controller: me, hook: "cry" },
      ...removals(view, "destroyed"),
      { type: "cardResolved", player: me, instanceId: "fx-clear", defId: "core-088", permanent: false, costPaid: 4 },
    ]);
    cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "nether");
    measureLayers().then((layers) => holdsLayers("Twisting Nether", layers));
  });

  it("R4493001 a Mythic takeover holds the layer budget at 3x", () => {
    const view = fullBoardView();
    mountScene(view, view, mythic(view, "meditative-049", 3));
    cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", "ai");
    measureLayers().then((layers) => holdsLayers("Mythic takeover", layers));
  });

  it("R4493000 R4493001 a takeover through Ceaseless Void's diorama draws at most FX_PHONE_STAGE_LAYERS planes and holds the layer budget at 3x", () => {
    const view = fullBoardView();
    mountScene(view, view, mythic(view, "core-100", 2));
    cy.get(`${FX_DOM} [data-fx="takeover"] .fx-takeover-plane[data-slice]`).should("have.length", FX_PHONE_STAGE_LAYERS);
    measureLayers().then((layers) => {
      expect(layers.planes, "the stage's planes").to.be.at.most(FX_PHONE_STAGE_LAYERS);
      holdsLayers("Ceaseless Void's takeover", layers);
    });
  });

  it("R4493001 Ceaseless Void's fold holds the layer budget at 3x", () => {
    const view = fullBoardView();
    const me = view.viewer;
    mountScene(view, view, [
      { type: "cardPlayed", player: me, instanceId: "fx-void", defId: "core-100", costPaid: 0 },
      { type: "hookResolving", sourceId: "fx-void", defId: "core-100", controller: me, hook: "cry" },
      ...removals(view, "exiled"),
      { type: "cardResolved", player: me, instanceId: "fx-void", defId: "core-100", permanent: true, costPaid: 0 },
    ]);
    cy.get(`${FX_DOM} [data-fx="setPiece"]`, { timeout: 4_000 }).should("have.attr", "data-shape", "fold");
    measureLayers().then((layers) => holdsLayers("Ceaseless Void's fold", layers));
  });

  it("R4493001 the Starship launch holds the layer budget at 3x", () => {
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
    cy.get(`${FX_DOM} [data-fx="cinematic"]`, { timeout: 4_000 }).should("exist");
    measureLayers().then((layers) => holdsLayers("Starship launch", layers));
  });

  it("R4493001 the Drone Battleship's arrival holds the layer budget at 3x", () => {
    const CORE_X = "stinkyscape-036";
    const layers = ["stinkyscape-t-meltdown-layer", "stinkyscape-t-propagation-layer", "stinkyscape-t-shield-layer"];
    const arrived = (done: boolean): PlayerView =>
      baseView({
        you: emptySide("p1", {
          units: [done ? unit("p1", { instanceId: "fx-shield", defId: layers[2], attack: 0, armor: 4, buried: 2 }) : null, null, null, null, null],
        }),
        opponent: emptySide("p2"),
      });
    mountScene(arrived(false), arrived(true), [
      { type: "cardPlayed", player: "p1", instanceId: "fx-core", defId: CORE_X, costPaid: 10 },
      ...layers.map((defId, k): GameEvent => ({ type: "summoned", player: "p1", instanceId: `fx-layer-${String(k)}`, defId, row: "units", lane: 1 })),
      { type: "cardResolved", player: "p1", instanceId: "fx-core", defId: CORE_X, permanent: false, costPaid: 10 },
    ]);
    cy.get("[data-animating]", { timeout: 4_000 }).should("exist");
    measureLayers().then((read) => holdsLayers("Drone Battleship arrival", read));
  });

  it("R4493001 the Anomaly cast holds the layer budget at 3x", () => {
    const view = fullBoardView();
    const me = view.viewer;
    const mine = view.you.units.flatMap((u): GameEvent[] => (u === null ? [] : [{ type: "exiled", instanceId: u.instanceId, defId: u.defId, owner: u.owner }]));
    const rocks: GameEvent[] = [1, 2, 3, 4, 5].map((lane) => ({ type: "summoned", player: me, instanceId: `fx-rock-${String(lane)}`, defId: "stinkyscape-169", row: "units", lane }));
    mountScene(view, view, [
      { type: "cardPlayed", player: me, instanceId: "fx-anomaly", defId: "stinkyscape-t-anomaly-found", costPaid: 4 },
      { type: "boardReplaced", player: me, anomaly: "AT" },
      ...mine,
      ...rocks,
      { type: "cardResolved", player: me, instanceId: "fx-anomaly", defId: "stinkyscape-t-anomaly-found", permanent: false, costPaid: 4 },
    ]);
    cy.get(`${FX_DOM} [data-fx="rift"]`, { timeout: 4_000 }).should("exist");
    measureLayers().then((layers) => holdsLayers("Anomaly cast", layers));
  });

  it("R4493001 The Big Crunch holds the layer budget at 3x", () => {
    const view = fullBoardView();
    const entry = FX_CARD_TABLE.get(BIG_CRUNCH);
    if (entry === undefined || entry.kind !== "setPiece") throw new Error("The Big Crunch has no set piece");
    mountScene(view, view, galleryCardEvents(BIG_CRUNCH, { kind: "setPiece", key: entry.key }, view, 1));
    cy.get(`${FX_DOM} [data-fx="setPiece"]`, { timeout: 6_000 }).should("have.attr", "data-shape", "crunch");
    measureLayers().then((layers) => holdsLayers("The Big Crunch", layers));
  });
});
