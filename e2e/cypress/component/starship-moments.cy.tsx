// R3334000–R3334003 (issue #3334): the Starship's moments through the real board's effects layer, at the
// slowest effects speed (FX_SPEED_MIN), at desktop size (1280x720) and on a phone (390x844), each with a
// Starship holding 1, 4 and 8 pieces beforehand:
// - R3334000: a fuse of one piece and a fuse of three at once, each piece's module flying from its card to
//   its hardpoint on the Starship and welding on, its hum rising with the count (no card back flies);
// - R3334001: a launch held as the `starship` set piece, its cinematic over the board (a stop for each
//   module, its engines and their sparks, fewer on a phone, and the ship's name in its design's lettering,
//   no countdown) and a tap that skips it at once; Big moments short and off and Reduce Motion show the
//   still title card instead;
// - R3334002: each of the eight broadsides (beam, repeater, cannon, flak and the rush, fruit, jade and book
//   launchers), fired where the ship stands, the cannon kicking back, and the unarmed ship's ram, its lunge
//   turned prow first;
// - R3334003: the wreck, its newest modules blown off, the hull broken in two halves and a scorch on its zone.
// A DOM cue is caught as it mounts (a MutationObserver on the effects layer's DOM root: a weld or a cannon's
// kick stands a fifth of a second), with the boxes of the cards and the zone it plays on at that moment;
// the shots are canvas cues, caught as lit pixels at their target. At rest nothing of any of it is left.
// The launch's cinematic and the wreck's break are captured once at each size, the largest ship's, with
// CSS animations running (e2e/README.md), in Chrome (`CAPTURES`).
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/starship-moments.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { starshipDesignOf, STARSHIP_MODULES_MAX } from "../../../apps/web/src/cards/art/starshipHulls.ts";
import { starshipModuleOf } from "../../../apps/web/src/cards/art/starshipModules.ts";
import { STARSHIP_HULL_PLATES_MAX } from "../../../apps/web/src/cards/constants.ts";
import { FX_MOBILE_WIDTH, FX_SETTINGS_KEY, FX_SHAKE_MAX_PX, FX_SPEED_MIN, FX_STARSHIP_RAM_TURN_DEG } from "../../../apps/web/src/fx/constants.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import {
  BROADSIDE_WEAPONS,
  STARSHIP_CINEMATIC,
  STARSHIP_DOCK,
  STARSHIP_LETTERING,
  STARSHIP_TUNING,
  STARSHIP_WRECK,
  starshipSizeOf,
  type BroadsideFire,
} from "../../../apps/web/src/fx/starship.ts";
import { STARSHIP_SET_PIECE } from "../../../apps/web/src/game/animations.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { __resetSettingsForTests, resetSettings, writeSettings } from "../../../apps/web/src/settings/index.ts";
import { baseView, dormantStarship, emptySide, unit, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof baseView>;
type UnitView = ReturnType<typeof unit>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const VIEWPORTS = [
  { label: "1280x720", width: 1280, height: 720 },
  { label: "390x844", width: 390, height: 844 },
] as const;
type Viewport = (typeof VIEWPORTS)[number];

/** How many pieces the Starship holds before each scene. */
const COUNTS = [1, 4, 8] as const;
/** The one ship each size's captures are of: the largest. */
const SHOT_COUNT = 8;
/**
 * Whether the captures are taken. Electron's headless window draws the page's animation frames at about one
 * a second after any `cy.screenshot` (measured here: one frame in the 1.5 s after a capture, against some
 * fifty in Chrome, in that test and every one after it), which starves every cue played after it; Chrome,
 * which CI's component job runs (ci.yml `e2e component (chrome)`), keeps drawing. A capture checks nothing.
 */
const CAPTURES = Cypress.browser.name !== "electron";

const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';
const SKIP_HINT = '[data-testid="set-piece-skip"]';

const HULL = "stinkyscape-t-dormant-starship";
const NAME = "Ember of Departure";
const SHIP = "fx-ship";
const FOE = "fx-foe";
/** The lane the Starship stands in, its target facing it; the pieces that fuse stand in the lanes before it. */
const LANE = 4;
const cardOf = (instanceId: string): string => `[data-testid="card-${instanceId}"]`;
const SHIP_CARD = cardOf(SHIP);
/** The Starship's art, which its picture fills and its wreck covers. */
const SHIP_ART = `${SHIP_CARD} .cf-art`;
const FOE_CARD = cardOf(FOE);
const ZONE = `[data-testid="zone-you-units-${String(LANE)}"]`;

/** At a quarter speed every entry takes four times its table time; a launch's hold and lift-off run past the default wait. */
const SLOW = { timeout: 12_000 } as const;
/** A skipped launch is over at once: far inside the lift-off and landing (seconds, at this speed) it would still have played. */
const AT_ONCE = { timeout: 1_000 } as const;
/** The canvas watcher's page-time budget, past every entry here and its tail. */
const WATCH_MS = 10_000;
/** How far round a target a shot's lit pixels may fall. */
const NEAR_PX = 48;
/** How far a cue's point may sit from the box it was measured against: the board's shake moves its cards between the two. */
const SLACK_PX = FX_SHAKE_MAX_PX;
/** How far a cue's size may differ from the box it covers (sub-pixel layout). */
const SIZE_PX = 1;

const BEAM = "stinkyscape-050";
const REPEATER = "stinkyscape-051";
const CANNON = "stinkyscape-052";
const FLAK = "stinkyscape-053";
const RUSH = "stinkyscape-140";
const FRUIT = "stinkyscape-141";
const JADE = "stinkyscape-142";
const BOOK = "stinkyscape-143";
const SHIELD = "stinkyscape-038";
const HEALING = "stinkyscape-043";
const DRILL = "stinkyscape-062";
const TRACTOR = "stinkyscape-067";
const AFTERBURNER = "stinkyscape-090";
const NANOBOTS = "stinkyscape-093";
const BOOSTER = "stinkyscape-098";
const OVERDRIVE = "stinkyscape-101";
const PLATING = "stinkyscape-104";

/** The eight broadsides: each weapon's piece and how it fires (R3334002). */
const BROADSIDES: readonly { key: string; defId: string; fire: BroadsideFire }[] = [
  { key: "beam", defId: BEAM, fire: "beam" },
  { key: "repeater", defId: REPEATER, fire: "repeater" },
  { key: "cannon", defId: CANNON, fire: "cannon" },
  { key: "flak", defId: FLAK, fire: "flak" },
  { key: "rush launcher", defId: RUSH, fire: "launcher" },
  { key: "fruit launcher", defId: FRUIT, fire: "launcher" },
  { key: "jade launcher", defId: JADE, fire: "launcher" },
  { key: "book launcher", defId: BOOK, fire: "launcher" },
];
/** Pieces that are no weapon, filling a ship out to its count: they fire nothing. */
const UNARMED = [SHIELD, AFTERBURNER, DRILL, PLATING, HEALING, BOOSTER, TRACTOR, NANOBOTS] as const;
/** A launch's, a fuse's and a wreck's ship: weapons, launchers, two engines and the rest. */
const MIX = [BEAM, AFTERBURNER, RUSH, OVERDRIVE, SHIELD, CANNON, FRUIT, DRILL] as const;
/** The pieces that fuse: one, or three at once. */
const FUSE_ONE = [CANNON] as const;
const FUSE_THREE = [REPEATER, FRUIT, BOOSTER] as const;

const fireOf = (defId: string): BroadsideFire | undefined => BROADSIDE_WEAPONS[starshipModuleOf(defId).kind]?.fire;

// The fixtures stay what the scenes say: each broadside's piece fires as it is named, the others fire nothing.
for (const { key, defId, fire } of BROADSIDES) {
  if (fireOf(defId) !== fire) throw new Error(`${defId} no longer fires as a ${key}`);
}
for (const defId of UNARMED) {
  if (fireOf(defId) !== undefined) throw new Error(`${defId} is a weapon now`);
}

/** The Starship holding `pieces`, oldest first: Dormant, or launched under NAME. */
function starship(pieces: readonly string[], launched: boolean): UnitView {
  return launched
    ? unit("p1", { instanceId: SHIP, defId: HULL, attack: 3, starship: { pieces: pieces.map((defId) => ({ defId })), name: NAME, launched: 1 } })
    : dormantStarship("p1", pieces, { instanceId: SHIP });
}

/** p1's board: `ship` in lane LANE and `others` in the lanes from 1; p2's unit FOE facing the ship. */
function board(ship: UnitView | null, others: readonly UnitView[] = []): PlayerView {
  const units: (UnitView | null)[] = [null, null, null, null, null];
  others.forEach((u, k) => {
    units[k] = u;
  });
  units[LANE - 1] = ship;
  const foes: (UnitView | null)[] = [null, null, null, null, null];
  foes[LANE - 1] = unit("p2", { instanceId: FOE, attack: 2, maxHealth: 6, health: 6 });
  return baseView({ you: emptySide("p1", { units }), opponent: emptySide("p2", { units: foes }) });
}

/** A DOM cue as it mounted, with the boxes of the elements its scene measures (by selector) at that moment. */
type Mounted = { el: HTMLElement; boxes: Readonly<Record<string, DOMRect | null>> };
/** What a scene caught: every DOM cue that mounted, and whether the canvas lit round its target. */
type Caught = { mounted: Mounted[]; lit: boolean; stop: () => void };

/** Whether the canvas holds a lit pixel within `margin` of `box` (fx-layer.cy.tsx's). */
function litNear(canvas: HTMLCanvasElement, box: DOMRect, margin: number): boolean {
  const ctx = canvas.getContext("2d");
  if (ctx === null) return false;
  const rect = canvas.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) return false;
  const sx = canvas.width / rect.width;
  const sy = canvas.height / rect.height;
  const x0 = Math.max(0, Math.floor((box.left - margin - rect.left) * sx));
  const y0 = Math.max(0, Math.floor((box.top - margin - rect.top) * sy));
  const x1 = Math.min(canvas.width, Math.ceil((box.right + margin - rect.left) * sx));
  const y1 = Math.min(canvas.height, Math.ceil((box.bottom + margin - rect.top) * sy));
  if (x1 <= x0 || y1 <= y0) return false;
  const data = ctx.getImageData(x0, y0, x1 - x0, y1 - y0).data;
  for (let i = 3; i < data.length; i += 4) {
    if ((data[i] ?? 0) > 0) return true;
  }
  return false;
}

type PlayOptions = {
  /** The layer's state once mounted: "off" under Reduce Motion, which draws still cues only. */
  fx?: "on" | "off";
  /** The selectors whose boxes are measured as each cue mounts. */
  measure?: readonly string[];
  /** A selector whose surroundings are watched for lit canvas pixels. */
  litNear?: string;
};

/**
 * Mounts the board as `before`, starts catching what the effects layer mounts, then plays `events` over
 * `after`. What it catches fills in as the scene runs.
 */
function play(before: PlayerView, after: PlayerView, events: GameEvent[], options: PlayOptions = {}): Caught {
  const caught: Caught = { mounted: [], lit: false, stop: () => undefined };
  const lookup = lookupFromDefs(CATALOG);
  const noop = (): void => undefined;
  const scene = (view: PlayerView) => (
    <CatalogContext.Provider value={lookup}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>
    </CatalogContext.Provider>
  );
  cy.mount(scene(before)).then(({ rerender }) => {
    cy.get(LAYER).should("have.attr", "data-fx", options.fx ?? "on");
    cy.get(FX_DOM).then(($root) => {
      const root = $root[0] as HTMLElement;
      const doc = root.ownerDocument;
      const win = doc.defaultView;
      if (win === null) throw new Error("the effects layer has no window");
      const measure = options.measure ?? [];
      const boxOf = (selector: string): DOMRect | null => doc.querySelector(selector)?.getBoundingClientRect() ?? null;
      const observer = new MutationObserver((records) => {
        for (const record of records) {
          for (const node of Array.from(record.addedNodes)) {
            if (node.nodeType !== 1 || !(node as Element).hasAttribute("data-fx")) continue;
            const boxes: Record<string, DOMRect | null> = {};
            for (const selector of measure) boxes[selector] = boxOf(selector);
            caught.mounted.push({ el: node as HTMLElement, boxes });
          }
        }
      });
      observer.observe(root, { childList: true });
      let frame = 0;
      const target = options.litNear;
      if (target !== undefined) {
        const start = win.performance.now();
        const tick = (): void => {
          const canvas = doc.querySelector<HTMLCanvasElement>(CANVAS);
          const box = boxOf(target);
          if (canvas !== null && box !== null && litNear(canvas, box, NEAR_PX)) caught.lit = true;
          if (!caught.lit && win.performance.now() - start < WATCH_MS) frame = win.requestAnimationFrame(tick);
        };
        frame = win.requestAnimationFrame(tick);
      }
      caught.stop = () => {
        observer.disconnect();
        win.cancelAnimationFrame(frame);
      };
    });
    cy.then(() => rerender(scene(withEvents(after, events))));
  });
  return caught;
}

/** At rest: nothing animating, nothing of the scene left in the effects layer, and the catching stopped. */
function atRest(caught: Caught): void {
  cy.get("[data-animating]", SLOW).should("not.exist");
  cy.get(FX_DOM, SLOW).should(($root) => {
    expect(($root[0] as HTMLElement).childElementCount, "nothing is left in the effects layer").to.eq(0);
  });
  cy.then(() => caught.stop());
}

/** The cues of `kind` the scene caught, in the order they mounted. */
function cuesOf(caught: Caught, kind: string): HTMLElement[] {
  return caught.mounted.filter((m) => m.el.getAttribute("data-fx") === kind).map((m) => m.el);
}

function mountedOf(caught: Caught, kind: string): Mounted[] {
  return caught.mounted.filter((m) => m.el.getAttribute("data-fx") === kind);
}

/** A cue's pixel variable (`--fx-x`, …), as dom.ts writes it. */
function px(el: HTMLElement, name: string): number {
  return Number.parseFloat(el.style.getPropertyValue(name));
}

/** `x, y` lies in `box`, give or take SLACK_PX. */
function expectInside(x: number, y: number, box: DOMRect | null | undefined, what: string): void {
  expect(box ?? null, `${what}: the box it plays on`).to.not.eq(null);
  if (box === null || box === undefined) return;
  expect(x, `${what}: x`).to.be.within(box.left - SLACK_PX, box.right + SLACK_PX);
  expect(y, `${what}: y`).to.be.within(box.top - SLACK_PX, box.bottom + SLACK_PX);
}

/** A cue placed over a box (`--fx-x/y/w/h`, dom.ts `cover`) covers the box `selector` measured, its corner give or take SLACK_PX. */
function expectCovers({ el, boxes }: Mounted, selector: string, what: string): void {
  const box = boxes[selector] ?? null;
  expect(box, `${what}: the box it covers`).to.not.eq(null);
  if (box === null) return;
  expect(px(el, "--fx-x"), `${what}: left`).to.be.closeTo(box.left, SLACK_PX);
  expect(px(el, "--fx-y"), `${what}: top`).to.be.closeTo(box.top, SLACK_PX);
  expect(px(el, "--fx-w"), `${what}: width`).to.be.closeTo(box.width, SIZE_PX);
  expect(px(el, "--fx-h"), `${what}: height`).to.be.closeTo(box.height, SIZE_PX);
}

/** Its engines in the cinematic: one for each stern module, at most `enginesMax`, else one at the stern's middle. */
function enginesOf(pieces: readonly string[]): number {
  const sterns = pieces.slice(0, STARSHIP_MODULES_MAX).filter((defId) => starshipModuleOf(defId).mount === "stern").length;
  return Math.min(STARSHIP_CINEMATIC.enginesMax, Math.max(1, sterns));
}

/** The sparks each engine throws in the cinematic of a ship of `n` pieces: a phone throws its share of them, never none. */
function sparksOf(n: number, viewport: Viewport): number {
  const sparks = STARSHIP_TUNING[starshipSizeOf(n)].sparks;
  return viewport.width < FX_MOBILE_WIDTH ? Math.max(1, Math.round(sparks * STARSHIP_CINEMATIC.phoneShare)) : sparks;
}

/** The font its name stamps in: its launched design's lettering. */
function fontOf(name: string): string {
  const design = starshipDesignOf(true, name);
  if (design === "shipyard") throw new Error("a launched Starship wears a design of its own");
  return STARSHIP_LETTERING[design].font;
}

/** Whether `el` is drawn at all just now (its computed opacity, mid-animation). */
function shows(el: Element): boolean {
  const win = el.ownerDocument.defaultView;
  return win !== null && Number(win.getComputedStyle(el).opacity) > 0;
}

/** R3334000: `fusing` fuse into a Dormant Starship of `n` pieces, one after another in one run. */
function fuse(n: number, fusing: readonly string[]): void {
  const held = MIX.slice(0, n);
  const pieces = fusing.map((defId, k) => unit("p1", { instanceId: `fx-piece-${String(k)}`, defId, attack: 1, maxHealth: 2, health: 2 }));
  const pieceCards = pieces.map((p) => cardOf(p.instanceId));
  const events: GameEvent[] = fusing.flatMap((defId, k): GameEvent[] => [
    { type: "fused", instanceIds: [SHIP, `fx-copy-${String(k)}`], resultInstanceId: SHIP, defId: HULL },
    { type: "starshipFused", player: "p1", instanceId: SHIP, pieceId: `fx-piece-${String(k)}`, defId, pieces: n + k + 1 },
  ]);
  const caught = play(board(starship(held, false), pieces), board(starship([...held, ...fusing], false)), events, { measure: [SHIP_CARD, ...pieceCards] });
  // Mid-flight: the run plays on the Starship, a module (never a card back) flying to it.
  cy.get(SHIP_CARD, SLOW).should("have.attr", "data-animating", "starshipFused");
  cy.get(`${FX_DOM} [data-fx="module"][data-motion="dock"]`).should("exist");
  atRest(caught);
  cy.then(() => {
    const where = `a fuse of ${String(fusing.length)} into ${String(n)}`;
    expect(cuesOf(caught, "ghost"), `${where}: no card back flies`).to.have.length(0);
    // Each piece's module leaves its card, in order, and lands on the Starship.
    const docks = mountedOf(caught, "module");
    expect(docks.map((m) => m.el.getAttribute("data-motion")), `${where}: one docking module per piece`).to.deep.eq(fusing.map(() => "dock"));
    docks.forEach(({ el, boxes }, k) => {
      expect(el.querySelector(".fx-module-img")?.getAttribute("src"), `${where}: module ${String(k)} is the module's picture`).to.match(/^data:image\/svg\+xml/);
      const x = px(el, "--fx-x");
      const y = px(el, "--fx-y");
      expectInside(x, y, boxes[pieceCards[k] ?? ""], `${where}: module ${String(k)} leaves its piece's card`);
      expectInside(x + px(el, "--fx-dx"), y + px(el, "--fx-dy"), boxes[SHIP_CARD], `${where}: module ${String(k)} lands on the Starship`);
    });
    // And each welds on where it lands: two clamps and a flash, humming higher with every piece the ship then holds.
    const welds = mountedOf(caught, "weld");
    expect(welds, `${where}: one weld per piece`).to.have.length(fusing.length);
    welds.forEach(({ el, boxes }, k) => {
      expect(el.querySelectorAll(".fx-weld-clamp"), `${where}: weld ${String(k)}'s clamps`).to.have.length(2);
      expect(el.querySelectorAll(".fx-weld-flash"), `${where}: weld ${String(k)}'s flash`).to.have.length(1);
      expect(el.style.getPropertyValue("--fx-hum"), `${where}: weld ${String(k)}'s hum`).to.eq(Math.min(1, (n + k + 1) / STARSHIP_DOCK.humFull).toFixed(3));
      expectInside(px(el, "--fx-x"), px(el, "--fx-y"), boxes[SHIP_CARD], `${where}: weld ${String(k)} is on the Starship`);
    });
  });
  // The board shows the Starship with a plate for each piece it now holds, as far as its plates go.
  cy.get(`${SHIP_CARD} .cf-starship-plate`).should("have.length", Math.min(n + fusing.length, STARSHIP_HULL_PLATES_MAX));
}

/** R3334001: a Dormant Starship of `n` pieces launches; its cinematic holds the board until a tap skips it. */
function launch(n: number, viewport: Viewport): void {
  const held = MIX.slice(0, n);
  const events: GameEvent[] = [
    { type: "awakened", player: "p1", instanceId: SHIP, defId: HULL },
    { type: "starshipLaunched", player: "p1", instanceId: SHIP, defId: HULL, name: NAME, pieces: [...held] },
  ];
  const caught = play(board(starship(held, false)), board(starship(held, true)), events);
  const where = `a launch of ${String(n)} at ${viewport.label}`;
  // Held: the queue holds it as the Starship's set piece, behind the hint.
  cy.get(GAME, SLOW).should("have.attr", "data-set-piece", STARSHIP_SET_PIECE);
  cy.get(SKIP_HINT).should("exist");
  cy.get(`${FX_DOM} [data-fx="cinematic"]`).should(($film) => {
    const film = $film[0] as HTMLElement;
    expect(film.getAttribute("data-still"), `${where}: a cinematic, not the title card`).to.eq(null);
    expect(film.getAttribute("data-size"), `${where}: its size`).to.eq(starshipSizeOf(n));
    expect(film.querySelector(".fx-cinematic-art")?.getAttribute("src"), `${where}: the ship's own picture`).to.match(/^data:image\/svg\+xml/);
    expect(film.querySelectorAll(".fx-cinematic-stop"), `${where}: a stop for each module it draws`).to.have.length(Math.min(n, STARSHIP_MODULES_MAX));
    const engines = Array.from(film.querySelectorAll(".fx-cinematic-engine"));
    expect(engines, `${where}: its engines`).to.have.length(enginesOf(held));
    for (const engine of engines) expect(engine.querySelectorAll(".fx-cinematic-spark"), `${where}: each engine's sparks`).to.have.length(sparksOf(n, viewport));
    const name = film.querySelector(".fx-cinematic-name");
    expect(name?.getAttribute("data-text"), `${where}: its name`).to.eq(NAME);
    expect(name?.getAttribute("data-font"), `${where}: in its design's lettering`).to.eq(fontOf(NAME));
  });
  // The tile's lift-off plays under it, with no countdown.
  cy.get(`${FX_DOM} [data-fx="launch"]`).should("exist");
  cy.get(`${FX_DOM} .fx-launch-count`).should("not.exist");
  if (CAPTURES && n === SHOT_COUNT) {
    // Mid-cinematic: the camera passing one of its modules.
    cy.get(`${FX_DOM} .fx-cinematic-stop`).should(($stops) => {
      expect(Array.from($stops).some(shows), `${where}: a module's stop lit`).to.eq(true);
    });
    cy.screenshot(`starship-launch-cinematic-${viewport.label}`, { capture: "viewport", overwrite: true, disableTimersAndAnimations: false });
  }
  // A tap skips it: the cinematic and the launch go at once, the hold is over and the board has caught up.
  cy.get(GAME).click({ force: true });
  cy.get(`${FX_DOM} [data-fx="cinematic"]`, AT_ONCE).should("not.exist");
  cy.get(`${FX_DOM} [data-fx="launch"]`).should("not.exist");
  cy.get("[data-animating]", AT_ONCE).should("not.exist");
  cy.get(GAME).should("not.have.attr", "data-set-piece");
  cy.get(SKIP_HINT).should("not.exist");
  cy.get(`${SHIP_CARD} .cf-starship-hull`).should("not.exist");
  atRest(caught);
  cy.then(() => {
    expect(cuesOf(caught, "cinematic"), `${where}: one cinematic`).to.have.length(1);
  });
}

/** R3334001: the title card a launch of `n` shows under Big moments `mode` or Reduce Motion. */
function titleCard(n: number, mode: "short" | "off" | "reduce"): void {
  const held = MIX.slice(0, n);
  const events: GameEvent[] = [
    { type: "awakened", player: "p1", instanceId: SHIP, defId: HULL },
    { type: "starshipLaunched", player: "p1", instanceId: SHIP, defId: HULL, name: NAME, pieces: [...held] },
  ];
  cy.then(() => {
    if (mode === "reduce") writeSettings({ reduceMotion: true });
    else setFxSettings({ bigMoments: mode });
  });
  const caught = play(board(starship(held, false)), board(starship(held, true)), events, { fx: mode === "reduce" ? "off" : "on" });
  const where = `${mode} with ${String(n)} pieces`;
  cy.get(`${FX_DOM} [data-fx="cinematic"]`, SLOW).should(($card) => {
    const card = $card[0] as HTMLElement;
    expect(card.getAttribute("data-still"), `${where}: the still title card`).to.eq("true");
    expect(card.querySelector(".fx-cinematic-art")?.getAttribute("src"), `${where}: the ship's own picture`).to.match(/^data:image\/svg\+xml/);
    expect(card.querySelector(".fx-cinematic-name")?.getAttribute("data-text"), `${where}: its name`).to.eq(NAME);
    expect(card.querySelectorAll(".fx-cinematic-spark"), `${where}: no engine throws a spark`).to.have.length(0);
  });
  if (mode === "short") {
    // Short holds the board for a short beat, the title card over it.
    cy.get(GAME).should("have.attr", "data-set-piece", STARSHIP_SET_PIECE);
  } else if (mode === "off") {
    // Off holds nothing: the launch plays straight on, the title card through it.
    cy.get(SHIP_CARD).should("have.attr", "data-animating", "starshipLaunched");
    cy.get(GAME).should("not.have.attr", "data-set-piece");
  } else {
    // Reduce Motion: no hold and no motion, the title card the one effect there is.
    cy.get(GAME).should("not.have.attr", "data-set-piece");
    cy.get(FX_DOM).should("have.attr", "data-still", "true");
    cy.get(FX_DOM).should(($root) => {
      expect(($root[0] as HTMLElement).childElementCount, `${where}: the title card alone`).to.eq(1);
    });
  }
  atRest(caught);
  cy.then(() => {
    expect(cuesOf(caught, "cinematic"), `${where}: one title card`).to.have.length(1);
    if (mode === "reduce") resetSettings();
    else setFxSettings({ bigMoments: "full" });
  });
}

/** R3334002: a launched Starship of `held` attacks the unit facing it. */
function attack(held: readonly string[]): Caught {
  const view = board(starship(held, true));
  return play(view, view, [{ type: "attackDeclared", attackerId: SHIP, targetId: FOE, forced: false }], { measure: [SHIP_CARD], litNear: FOE_CARD });
}

/** R3334002: a Starship of `n` pieces, its one weapon `weapon`'s, fires a broadside where it stands. */
function broadside(n: number, weapon: (typeof BROADSIDES)[number]): void {
  const caught = attack([weapon.defId, ...UNARMED.slice(0, n - 1)]);
  const where = `a ${weapon.key} broadside of ${String(n)}`;
  // It fires where it stands: a still lunge, never turned to ram.
  cy.get(SHIP_CARD, SLOW).should(($card) => {
    const card = $card[0] as HTMLElement;
    expect(card.getAttribute("data-animating"), `${where}: the attack plays`).to.eq("attackDeclared");
    expect(card.style.getPropertyValue("--fx-lunge-x"), `${where}: no lunge across`).to.eq("0px");
    expect(card.style.getPropertyValue("--fx-lunge-y"), `${where}: no lunge up`).to.eq("0px");
    expect(card.style.getPropertyValue("--fx-lunge-turn"), `${where}: no ram's turn`).to.eq("");
  });
  atRest(caught);
  cy.then(() => {
    expect(caught.lit, `${where}: its shots light the canvas at the target`).to.eq(true);
    const modules = mountedOf(caught, "module");
    if (weapon.fire !== "cannon") {
      expect(modules, `${where}: no module moves`).to.have.length(0);
      return;
    }
    // A cannon kicks back where it stands on the Starship.
    expect(modules.map((m) => m.el.getAttribute("data-motion")), `${where}: the cannon's kick`).to.deep.eq(["recoil"]);
    const [kick] = modules;
    if (kick === undefined) return;
    expect(px(kick.el, "--fx-dx"), `${where}: it kicks where it stands`).to.eq(0);
    expect(px(kick.el, "--fx-dy"), `${where}: it kicks where it stands`).to.eq(0);
    expect(kick.el.querySelector(".fx-module-img")?.getAttribute("src"), `${where}: the cannon's picture`).to.match(/^data:image\/svg\+xml/);
    expectInside(px(kick.el, "--fx-x"), px(kick.el, "--fx-y"), kick.boxes[SHIP_CARD], `${where}: the cannon is on the Starship`);
  });
  cy.get(SHIP_CARD).should(($card) => {
    expect(($card[0] as HTMLElement).style.getPropertyValue("--fx-lunge-x"), `${where}: the lunge is undone`).to.eq("");
  });
}

/** R3334002: a launched Starship of `n` pieces and no weapon rams its target prow first. */
function ram(n: number): void {
  const caught = attack(UNARMED.slice(0, n));
  const where = `a ram of ${String(n)}`;
  // It lunges at its target (above it), turned prow first toward it.
  cy.get(SHIP_CARD, SLOW).should(($card) => {
    const card = $card[0] as HTMLElement;
    expect(card.getAttribute("data-animating"), `${where}: the attack plays`).to.eq("attackDeclared");
    expect(card.style.getPropertyValue("--fx-lunge-turn"), `${where}: turned prow first`).to.eq(`${String(-FX_STARSHIP_RAM_TURN_DEG)}deg`);
    expect(px(card, "--fx-lunge-y"), `${where}: it lunges up at its target`).to.be.lessThan(0);
  });
  atRest(caught);
  cy.then(() => {
    expect(caught.lit, `${where}: its contact lights the canvas at the target`).to.eq(true);
    expect(mountedOf(caught, "module"), `${where}: no gun kicks`).to.have.length(0);
  });
  cy.get(SHIP_CARD).should(($card) => {
    expect(($card[0] as HTMLElement).style.getPropertyValue("--fx-lunge-turn"), `${where}: the turn is undone`).to.eq("");
  });
}

/** R3334003: a launched Starship of `n` pieces is destroyed: a wreck. */
function wreck(n: number, viewport: Viewport): void {
  const held = MIX.slice(0, n);
  const events: GameEvent[] = [{ type: "destroyed", instanceId: SHIP, defId: HULL, owner: "p1", controller: "p1", attack: 3, maxHealth: 4, killerId: null }];
  const caught = play(board(starship(held, true)), board(null), events, { measure: [SHIP_CARD, SHIP_ART, ZONE] });
  const where = `a wreck of ${String(n)} at ${viewport.label}`;
  // Mid-break: the hull over its art, in two halves, fore and aft, as its card is hidden.
  cy.get(`${FX_DOM} [data-fx="wreck"]`, SLOW).should(($wreck) => {
    const halves = Array.from(($wreck[0] as HTMLElement).querySelectorAll(".fx-wreck-half"));
    expect(halves.map((half) => half.getAttribute("data-half")), `${where}: two halves`).to.deep.eq(["fore", "aft"]);
    for (const half of halves) expect(half.getAttribute("src"), `${where}: each half the ship's picture`).to.match(/^data:image\/svg\+xml/);
  });
  cy.get(SHIP_CARD).should("have.attr", "data-fx-concealed", "after");
  if (CAPTURES && n === SHOT_COUNT) {
    cy.get(`${FX_DOM} [data-fx="wreck"] .fx-wreck-half`).should(($halves) => {
      expect(Array.from($halves).some(shows), `${where}: the hull broken`).to.eq(true);
    });
    cy.screenshot(`starship-wreck-break-${viewport.label}`, { capture: "viewport", overwrite: true, disableTimersAndAnimations: false });
  }
  atRest(caught);
  cy.then(() => {
    // Its newest modules blew off the hull, one at a time.
    const off = mountedOf(caught, "module");
    expect(off.map((m) => m.el.getAttribute("data-motion")), `${where}: its newest modules blow off`).to.deep.eq(
      Array.from({ length: Math.min(n, STARSHIP_WRECK.modulesMax) }, () => "blowOff"),
    );
    off.forEach(({ el, boxes }, k) => {
      expectInside(px(el, "--fx-x"), px(el, "--fx-y"), boxes[SHIP_CARD], `${where}: module ${String(k)} leaves the Starship`);
    });
    // The hull broke in two over its art.
    const [hull, ...more] = mountedOf(caught, "wreck");
    expect(more, `${where}: one wreck`).to.have.length(0);
    if (hull === undefined) throw new Error(`${where}: no wreck`);
    expect(hull.el.style.getPropertyValue("--fx-turn"), `${where}: each half turns away`).to.eq(`${String(STARSHIP_WRECK.halfTurnDeg)}deg`);
    expectCovers(hull, SHIP_ART, `${where}: the wreck covers the Starship's art`);
    // A scorch settled over its zone.
    const scorches = mountedOf(caught, "scorch");
    expect(scorches, `${where}: one scorch`).to.have.length(1);
    const [scorch] = scorches;
    if (scorch === undefined) return;
    expectCovers(scorch, ZONE, `${where}: the scorch covers its zone`);
  });
  cy.get(SHIP_CARD).should("not.exist");
}

describe("R3334000 R3334001 R3334002 R3334003 the Starship's moments on a real board", () => {
  // The effects at their slowest, so every moment outlasts the command queue; every setting put back after each test, a failed one included.
  beforeEach(() => {
    setFxSettings({ speed: FX_SPEED_MIN });
  });
  afterEach(() => {
    setFxSettings({ ...DEFAULT_FX_SETTINGS });
    window.localStorage.removeItem(FX_SETTINGS_KEY);
    resetSettings();
    __resetSettingsForTests();
  });

  for (const viewport of VIEWPORTS) {
    const ships = `a Starship of ${COUNTS.join(", ")} pieces`;
    const at = `at ${viewport.label}`;

    describe(`R3334000 a fused piece's module docks and welds ${at}`, () => {
      it(`R3334000 a fuse of one piece into ${ships} ${at}`, () => {
        cy.viewport(viewport.width, viewport.height);
        for (const n of COUNTS) fuse(n, FUSE_ONE);
      });

      it(`R3334000 a fuse of three at once into ${ships} ${at}`, () => {
        cy.viewport(viewport.width, viewport.height);
        for (const n of COUNTS) fuse(n, FUSE_THREE);
      });
    });

    describe(`R3334001 the launch ${at}`, () => {
      it(`R3334001 the cinematic of ${ships}, skipped by a tap, ${at}`, () => {
        cy.viewport(viewport.width, viewport.height);
        for (const n of COUNTS) launch(n, viewport);
      });

      it(`R3334001 Big moments short and off and Reduce Motion show the still title card ${at}`, () => {
        cy.viewport(viewport.width, viewport.height);
        titleCard(1, "short");
        titleCard(4, "off");
        titleCard(8, "reduce");
      });
    });

    describe(`R3334002 the broadside ${at}`, () => {
      for (const weapon of BROADSIDES) {
        it(`R3334002 a ${weapon.key} broadside from ${ships} ${at}`, () => {
          cy.viewport(viewport.width, viewport.height);
          for (const n of COUNTS) broadside(n, weapon);
        });
      }

      it(`R3334002 ${ships} and no weapon rams prow first ${at}`, () => {
        cy.viewport(viewport.width, viewport.height);
        for (const n of COUNTS) ram(n);
      });
    });

    describe(`R3334003 the wreck ${at}`, () => {
      it(`R3334003 the wreck of ${ships} ${at}`, () => {
        cy.viewport(viewport.width, viewport.height);
        for (const n of COUNTS) wreck(n, viewport);
      });
    });
  }
});
