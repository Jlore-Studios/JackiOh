// Polish 1: B28 clamped DPR, B35 board shake, B40 effects draw without blocking the board.
// R2722001: a Mythic's takeover of the board, caught mid-flight and at rest.
// R2785000: a set piece (that takeover) skipped by Space and by a click, and the board caught up at once.
// R2800000: each board clear's set piece (Twisting Nether's vortex, Ceaseless Void's fold, the shared vortex), mid-flight and at rest.
// R2801000: a Starship's fuse run and its launch, mid-flight and at rest; R3334000: each fused piece's module
// docks in place of the card back that flew there, and R3334001: the launch is a cinematic, its countdown gone.
// The watcher must see the splat before synchronous `getImageData`: a later read can miss R200's brief effect.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-layer.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { FX_MAX_DPR } from "../../../apps/web/src/fx/constants.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, dormantStarship, emptySide, fullBoardView, unit, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

const BOARD = '[data-testid="board"]';
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const FX_DOM = '[data-testid="fx-dom"]';
const SPLAT = '[data-fx="splat"]';

const NEAR_PX = 48;

/** Fixture-derived IDs keep the burst valid if the fixture changes. */
function burstFor(view: PlayerView): { events: GameEvent[]; targetId: string } {
  const target = view.opponent.units[4];
  if (target === null || target === undefined) throw new Error("fullBoardView() has no enemy unit in lane 5");
  if (target.health <= 8) throw new Error("the lane-5 enemy must survive 8 damage so its card stays on the board");
  const spell = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
  if (spell === undefined) throw new Error("fullBoardView() has no card in the viewer's hand");
  return {
    targetId: target.instanceId,
    events: [
      { type: "damage", sourceId: spell.instanceId, targetId: target.instanceId, amount: 8, combat: false },
      { type: "summoned", player: view.viewer, instanceId: "fx-summoned", defId: "core-019", row: "units", lane: 3 },
      { type: "turnStarted", player: view.viewer, turn: view.turn + 1 },
    ],
  };
}

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

/** Page-time watcher budget, past every effect tail. */
const WATCH_MS = 6_000;

type Seen = {
  splatOverIdleCard: boolean;
  clickLandsOnCard: boolean;
  litNearCard: boolean;
  lastHit: string;
};

function unseen(): Seen {
  return { splatOverIdleCard: false, clickLandsOnCard: false, litNearCard: false, lastHit: "" };
}

/** Samples frames until done or WATCH_MS; reads pixels only after the splat. */
function watchFrames(win: Window, card: string, seen: Seen): () => void {
  const doc = win.document;
  const start = win.performance.now();
  let handle = 0;
  let stopped = false;
  const tick = (): void => {
    if (stopped) return;
    const target = doc.querySelector<HTMLElement>(card);
    if (target !== null && !seen.clickLandsOnCard) {
      const splat = doc.querySelector(`${FX_DOM} ${SPLAT}`);
      if (splat !== null && !target.hasAttribute("data-animating")) {
        seen.splatOverIdleCard = true;
        const box = target.getBoundingClientRect();
        const hit = doc.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
        const fits = doc.documentElement.scrollWidth <= win.innerWidth;
        seen.lastHit = hit === null ? "nothing" : (hit.getAttribute("data-testid") ?? hit.getAttribute("class") ?? hit.tagName);
        if (hit !== null && target.contains(hit) && fits) seen.clickLandsOnCard = true;
      }
    }
    if (target !== null && seen.splatOverIdleCard && !seen.litNearCard) {
      const canvas = doc.querySelector<HTMLCanvasElement>(CANVAS);
      if (canvas !== null && litNear(canvas, target.getBoundingClientRect(), NEAR_PX)) seen.litNearCard = true;
    }
    const done = seen.clickLandsOnCard && seen.litNearCard;
    if (!done && win.performance.now() - start < WATCH_MS) handle = win.requestAnimationFrame(tick);
  };
  handle = win.requestAnimationFrame(tick);
  return () => {
    stopped = true;
    win.cancelAnimationFrame(handle);
  };
}

function expectNoHorizontalOverflow(doc: Document, where: string): void {
  const win = doc.defaultView;
  expect(win, "the document has a window").to.not.eq(null);
  if (win === null) return;
  expect(doc.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(win.innerWidth);
}

describe("Polish 1: the effects layer over a real board", () => {
  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`B40 B35 B28: a spell hit lights the canvas, shakes the board, blocks no click and leaves nothing behind at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);

      const view = fullBoardView();
      const { events, targetId } = burstFor(view);
      const card = `[data-testid="card-${targetId}"]`;
      const noop = (): void => undefined;

      const record = { translate: [] as string[], rotate: [] as string[] };
      const prior = { translate: "", rotate: "" };
      let observer: MutationObserver | null = null;
      // `should` retries the mutable watcher result; an assertion after `.then()` would not.
      const seen = unseen();
      let stopWatching: (() => void) | null = null;

      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={view} legal={[]} onAction={noop} />
        </div>,
      ).then(({ rerender }) => {
        cy.get(LAYER).should("have.attr", "data-fx", "on");
        cy.get(CANVAS).should("exist");
        cy.get(FX_DOM).should("exist");
        cy.document().should((doc) => expectNoHorizontalOverflow(doc, where));

        cy.get(BOARD).then(($board) => {
          const board = $board[0] as HTMLElement;
          prior.translate = board.style.translate;
          prior.rotate = board.style.rotate;
          observer = new MutationObserver(() => {
            record.translate.push(board.style.translate);
            record.rotate.push(board.style.rotate);
          });
          observer.observe(board, { attributes: true, attributeFilter: ["style"] });
          const win = board.ownerDocument.defaultView;
          if (win === null) throw new Error("the board has no window");
          stopWatching = watchFrames(win, card, seen);
        });

        cy.then(() => rerender(
          <div className="app-shell app-shell--wide">
            <Game view={withEvents(view, events)} legal={[]} onAction={noop} />
          </div>,
        ));
      });

      // B40.
      cy.wrap(seen).should((saw: Seen) => {
        expect(saw.splatOverIdleCard, "its damage splat is on screen after the card's own damage motion has finished").to.eq(true);
        expect(saw.clickLandsOnCard, `elementFromPoint at the card's center is inside the card, and the board fits (last hit: ${saw.lastHit})`).to.eq(true);
        expect(saw.litNearCard, "a lit canvas pixel near the damaged card").to.eq(true);
      });

      // B28.
      cy.window().then((win) => {
        const dpr = Math.min(Math.max(win.devicePixelRatio || 1, 1), FX_MAX_DPR);
        cy.get(CANVAS).should(($canvas) => {
          const canvas = $canvas[0] as HTMLCanvasElement;
          const rect = canvas.getBoundingClientRect();
          expect(rect.width, "the canvas has a CSS width").to.be.greaterThan(0);
          expect(canvas.width, "backing width = CSS width x DPR").to.be.closeTo(rect.width * dpr, 1);
          expect(canvas.height, "backing height = CSS height x DPR").to.be.closeTo(rect.height * dpr, 1);
        });
      });

      cy.get("[data-animating]").should("not.exist");

      cy.get(FX_DOM).should(($root) => {
        expect(($root[0] as HTMLElement).childElementCount, "fx-dom is empty").to.eq(0);
      });

      // B35.
      cy.get(BOARD).should(($board) => {
        const board = $board[0] as HTMLElement;
        expect(
          record.translate.some((value) => value !== "" && value !== prior.translate),
          "the board carried an inline translate while it shook",
        ).to.eq(true);
        expect(board.style.translate, "translate restored").to.eq(prior.translate);
        expect(board.style.rotate, "rotate restored").to.eq(prior.rotate);
      });

      cy.document().should((doc) => expectNoHorizontalOverflow(doc, where));
      cy.then(() => {
        observer?.disconnect();
        stopWatching?.();
      });
    });
  }
});

describe("R2722001 a Mythic takes over a real board", () => {
  it("R2722001 a Mythic's takeover mid-flight and at rest", () => {
    cy.viewport(1280, 720);
    const view = fullBoardView();
    const lookup = lookupFromDefs(CATALOG);
    const noop = (): void => undefined;
    // Meditative #49 YileGPT, a Mythic Unit, played into the viewer's empty lane 3.
    const events: GameEvent[] = [
      { type: "cardPlayed", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", costPaid: 2 },
      { type: "summoned", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", row: "units", lane: 3 },
    ];
    const board = (shown: PlayerView) => (
      <CatalogContext.Provider value={lookup}>
        <div className="app-shell app-shell--wide">
          <Game view={shown} legal={[]} onAction={noop} />
        </div>
      </CatalogContext.Provider>
    );
    cy.mount(board(view)).then(({ rerender }) => {
      cy.get(LAYER).should("have.attr", "data-fx", "on");
      cy.then(() => rerender(board(withEvents(view, events))));
    });
    // Mid-flight: the queue holds the entry for the takeover, and its layers are over the board.
    cy.get('[data-testid="game"]').should("have.attr", "data-takeover", "mythic");
    cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", "ai");
    cy.get(`${FX_DOM} .fx-takeover-sweep`).should("exist");
    cy.screenshot("takeover-mid", { capture: "viewport", overwrite: true });
    // At rest: nothing animating, the hold over, and every layer of the takeover gone.
    cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
    cy.get('[data-testid="game"]').should("not.have.attr", "data-takeover");
    cy.get(`${FX_DOM} [data-fx="takeover"]`, { timeout: 4_000 }).should("not.exist");
    cy.get('[data-testid="takeover-still"]').should("not.exist");
    cy.screenshot("takeover-rest", { capture: "viewport", overwrite: true });
  });
});

describe("R2785000 a set piece is skipped on a real board", () => {
  const GAME = '[data-testid="game"]';
  const skips: readonly (readonly [string, () => void])[] = [
    ["Space", () => cy.document().trigger("keydown", { key: " " })],
    ["a click", () => cy.get(GAME).click({ force: true })],
  ];
  for (const [how, skip] of skips) {
    it(`R2785000 ${how} skips a Mythic's takeover at once`, () => {
      cy.viewport(1280, 720);
      const view = fullBoardView();
      const lookup = lookupFromDefs(CATALOG);
      const noop = (): void => undefined;
      // Meditative #49 YileGPT, a Mythic Unit, played into the viewer's empty lane 3: the takeover set piece.
      const events: GameEvent[] = [
        { type: "cardPlayed", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", costPaid: 2 },
        { type: "summoned", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", row: "units", lane: 3 },
      ];
      const board = (shown: PlayerView) => (
        <CatalogContext.Provider value={lookup}>
          <div className="app-shell app-shell--wide">
            <Game view={shown} legal={[]} onAction={noop} />
          </div>
        </CatalogContext.Provider>
      );
      cy.mount(board(view)).then(({ rerender }) => {
        cy.get(LAYER).should("have.attr", "data-fx", "on");
        cy.then(() => rerender(board(withEvents(view, events))));
      });
      cy.get(GAME).should("have.attr", "data-set-piece", "takeover");
      cy.get('[data-testid="set-piece-skip"]').should("contain.text", "Tap to skip");
      cy.then(skip);
      // At once: far inside what is left of the hold, the board has caught up and the takeover is gone.
      cy.get("[data-animating]", { timeout: 300 }).should("not.exist");
      cy.get(GAME).should("not.have.attr", "data-set-piece");
      cy.get('[data-testid="set-piece-skip"]').should("not.exist");
      cy.get(`${FX_DOM} [data-fx="takeover"]`, { timeout: 300 }).should("not.exist");
    });
  }
});

describe("R2800000 a board clear's set piece on a real board", () => {
  const GAME = '[data-testid="game"]';
  type Clear = { key: string; shape: string; defId: string; view: PlayerView; events: GameEvent[] };

  /** Every Unit on the board, as the removals of `type` a wipe streams. */
  function removals(view: PlayerView, type: "destroyed" | "exiled"): GameEvent[] {
    return [...view.you.units, ...view.opponent.units].flatMap((u): GameEvent[] => {
      if (u === null) return [];
      return type === "destroyed"
        ? [{ type, instanceId: u.instanceId, defId: u.defId, owner: u.owner, controller: u.controller, attack: u.attack, maxHealth: u.maxHealth, killerId: null }]
        : [{ type, instanceId: u.instanceId, defId: u.defId, owner: u.owner }];
    });
  }

  /** A Spell `defId` the viewer casts whose hook destroys every Unit. */
  function spellClear(key: string, defId: string): Clear {
    const view = fullBoardView();
    const me = view.viewer;
    return {
      key,
      shape: "vortex",
      defId,
      view,
      events: [
        { type: "cardPlayed", player: me, instanceId: "fx-clear", defId, costPaid: 4 },
        { type: "hookResolving", sourceId: "fx-clear", defId, controller: me, hook: "cry" },
        ...removals(view, "destroyed"),
        { type: "cardResolved", player: me, instanceId: "fx-clear", defId, permanent: false, costPaid: 4 },
      ],
    };
  }

  /** #100 Ceaseless Void played into the viewer's emptied lane 3, its Cry exiling every other Unit. */
  function voidClear(): Clear {
    const full = fullBoardView();
    const view: PlayerView = { ...full, you: { ...full.you, units: full.you.units.map((u, i) => (i === 2 ? null : u)) } };
    const me = view.viewer;
    return {
      key: "voidFold",
      shape: "fold",
      defId: "core-100",
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

  // #88 Twisting Nether, #100 Ceaseless Void and Classic #43 Plague Nuke (the shared vortex).
  const clears: readonly (() => Clear)[] = [() => spellClear("nether", "core-088"), voidClear, () => spellClear("vortex", "classic-043")];
  for (const make of clears) {
    const { key, shape } = make();
    it(`R2800000 ${key} mid-flight and at rest`, () => {
      cy.viewport(1280, 720);
      const { view, events } = make();
      const lookup = lookupFromDefs(CATALOG);
      const noop = (): void => undefined;
      const board = (shown: PlayerView) => (
        <CatalogContext.Provider value={lookup}>
          <div className="app-shell app-shell--wide">
            <Game view={shown} legal={[]} onAction={noop} />
          </div>
        </CatalogContext.Provider>
      );
      cy.mount(board(view)).then(({ rerender }) => {
        cy.get(LAYER).should("have.attr", "data-fx", "on");
        cy.then(() => rerender(board(withEvents(view, events))));
      });
      // Mid-flight: the queue holds the wipe, the board twists or folds, and the cards it takes are drawn in.
      cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", key);
      cy.get(GAME).should("have.attr", "data-set-piece-shape", shape);
      cy.get(GAME).should("have.attr", "data-set-piece-sides", "you opponent");
      cy.get(`${FX_DOM} [data-fx="setPiece"]`).should("have.attr", "data-shape", shape);
      cy.get(`${BOARD} [data-fx-pulled]`).should("have.length.greaterThan", 0);
      cy.screenshot(`set-piece-${key}-mid`, { capture: "viewport", overwrite: true });
      // At rest: nothing animating, the hold over, and every layer of the set piece gone.
      cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
      cy.get(GAME).should("not.have.attr", "data-set-piece");
      cy.get(GAME).should("not.have.attr", "data-set-piece-shape");
      cy.get(`${FX_DOM} [data-fx="setPiece"]`, { timeout: 4_000 }).should("not.exist");
      cy.get(`${FX_DOM} [data-fx="takeover"]`).should("not.exist");
      cy.screenshot(`set-piece-${key}-rest`, { capture: "viewport", overwrite: true });
    });
  }
});

describe("R2801000 a Starship on a real board", () => {
  const GAME = '[data-testid="game"]';
  const HULL = "stinkyscape-t-dormant-starship";
  const LAUNCHERS = ["stinkyscape-140", "stinkyscape-141"];

  /** The viewer's board: `pieces` in lanes 1 and on, and their Starship in lane 5 (Dormant unless `launched`). */
  function starshipBoard(pieces: readonly string[], fused: readonly string[], launched = false): PlayerView {
    const units = [null, null, null, null, null] as (ReturnType<typeof unit> | null)[];
    pieces.forEach((defId, k) => {
      units[k] = unit("p1", { instanceId: `fx-piece-${String(k)}`, defId, attack: 0 });
    });
    units[4] = launched
      ? unit("p1", { instanceId: "fx-ship", defId: HULL, attack: 0, starship: { pieces: fused.map((defId) => ({ defId })), name: "Ember of Departure", launched: 1 } })
      : dormantStarship("p1", fused, { instanceId: "fx-ship" });
    return baseView({ you: emptySide("p1", { units }), opponent: emptySide("p2") });
  }

  function mountWith(before: PlayerView, after: PlayerView, events: GameEvent[]): void {
    const lookup = lookupFromDefs(CATALOG);
    const noop = (): void => undefined;
    const board = (shown: PlayerView) => (
      <CatalogContext.Provider value={lookup}>
        <div className="app-shell app-shell--wide">
          <Game view={shown} legal={[]} onAction={noop} />
        </div>
      </CatalogContext.Provider>
    );
    cy.mount(board(before)).then(({ rerender }) => {
      cy.get(LAYER).should("have.attr", "data-fx", "on");
      cy.then(() => rerender(board(withEvents(after, events))));
    });
  }

  it("R2801000 R3334000 a fuse run of two pieces mid-flight and at rest", () => {
    cy.viewport(1280, 720);
    const before = starshipBoard(LAUNCHERS, [HULL]);
    const after = starshipBoard([], [HULL, ...LAUNCHERS]);
    const events: GameEvent[] = LAUNCHERS.flatMap((defId, k): GameEvent[] => [
      { type: "destroyed", instanceId: `fx-piece-${String(k)}`, defId, owner: "p1", controller: "p1", attack: 0, maxHealth: 4, killerId: null },
    ]);
    LAUNCHERS.forEach((defId, k) => {
      events.push(
        { type: "fused", instanceIds: ["fx-ship", `fx-copy-${String(k)}`], resultInstanceId: "fx-ship", defId: HULL },
        { type: "starshipFused", player: "p1", instanceId: "fx-ship", pieceId: `fx-piece-${String(k)}`, defId, pieces: k + 2 },
      );
    });
    mountWith(before, after, events);
    // Mid-flight: a dead piece's module (no card back, R3334000) flies to its hardpoint on the Starship.
    cy.get('[data-testid="card-fx-ship"][data-animating="starshipFused"]', { timeout: 4_000 }).should("exist");
    cy.get(`${FX_DOM} [data-fx="module"]`).should("exist");
    cy.get(`${FX_DOM} [data-fx="ghost"]`).should("not.exist");
    cy.screenshot("starship-fuse-mid", { capture: "viewport", overwrite: true });
    // At rest: the board shows the Starship with a plate for each of its three pieces.
    cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
    cy.get('[data-testid="card-fx-ship"] .cf-starship-plate').should("have.length", 3);
    cy.get(`${FX_DOM} [data-fx="module"]`, { timeout: 4_000 }).should("not.exist");
    cy.get(`${FX_DOM} [data-fx="weld"]`).should("not.exist");
    cy.screenshot("starship-fuse-rest", { capture: "viewport", overwrite: true });
    cy.get('[data-testid="card-fx-ship"]').screenshot("starship-hull-rest", { overwrite: true });
  });

  it("R2801000 R3334001 a launch mid-flight and at rest", () => {
    cy.viewport(1280, 720);
    const fused = [HULL, ...LAUNCHERS];
    const before = starshipBoard([], fused);
    const after = starshipBoard([], fused, true);
    const events: GameEvent[] = [
      { type: "awakened", player: "p1", instanceId: "fx-ship", defId: HULL },
      { type: "starshipLaunched", player: "p1", instanceId: "fx-ship", defId: HULL, name: "Ember of Departure", pieces: fused },
    ];
    mountWith(before, after, events);
    // Mid-flight: the queue holds the launch, its cinematic over the board (R3334001) and its engines over
    // the Starship, with no countdown.
    cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "starship");
    cy.get(`${FX_DOM} [data-fx="cinematic"]`).should("exist");
    cy.get(`${FX_DOM} [data-fx="launch"]`).should("exist");
    cy.get(`${FX_DOM} .fx-launch-count`).should("not.exist");
    cy.screenshot("starship-launch-mid", { capture: "viewport", overwrite: true });
    // At rest: nothing animating, the hold over, and the launch gone; the Starship wears no hull plates.
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
    cy.get(GAME).should("not.have.attr", "data-set-piece");
    cy.get(`${FX_DOM} [data-fx="launch"]`, { timeout: 4_000 }).should("not.exist");
    cy.get(`${FX_DOM} [data-fx="cinematic"]`).should("not.exist");
    cy.get('[data-testid="card-fx-ship"] .cf-starship-hull').should("not.exist");
    cy.screenshot("starship-launch-rest", { capture: "viewport", overwrite: true });
  });
});
