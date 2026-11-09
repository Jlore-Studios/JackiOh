// R3351000: the living game on a phone (issue #3351, part of #3330). A full-board scenario at phone
// size: five art-first Units (two Legendary and one Mythic among them) with tile auras, an Anomaly's
// backdrop, a launched Starship and a hand of ten with one tilted, played at 390x844, 360x780 and
// 844x390 (a phone on its side) under 4x CPU throttling. Through it play a Mythic takeover, a
// Starship launch and broadside, a Drone Battleship arrival, a Terminus set piece, a hand-to-tile
// flight and a riffle, with frame times measured throughout against the one shared budget: the mean
// frame within FX_PHONE_FRAME_BUDGET_MS and the worst within FX_PHONE_FRAME_WORST_MS. A forced shed
// sheds motes first and a set piece's beats never.
// Run: E2E_COMPONENT_PORT=5283 pnpm --dir e2e test:component --spec cypress/component/fx-phone-living-game.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { ANOMALY_FOUND_ID } from "../../../apps/web/src/cards/anomalies.ts";
import { FX_PHONE_FRAME_BUDGET_MS, FX_PHONE_FRAME_WORST_MS, FX_SETTINGS_KEY, FX_SPEED_DEFAULT, FX_SPEED_MIN } from "../../../apps/web/src/fx/constants.ts";
import { planAuras } from "../../../apps/web/src/fx/aura/plan.ts";
import { planTileAuras } from "../../../apps/web/src/fx/aura/tile.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { setShedLevel } from "../../../apps/web/src/fx/shed.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import {
  baseView,
  card,
  dormantStarship,
  emptySide,
  faceDownBackrow,
  faceUpBackrow,
  unit,
  withEvents,
} from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof baseView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const SIZES = [
  { label: "390x844", width: 390, height: 844 },
  { label: "360x780", width: 360, height: 780 },
  { label: "844x390", width: 844, height: 390 },
] as const;
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';
/** Long enough to cross a set piece's hold, short enough to stay mid-flight. */
const SAMPLE_MS = 2_000;
/** How many fresh plays of a moment may be sampled before its budget is called broken (see `holdsLivingBudget`). */
const FRAME_ATTEMPTS = 3;
/** R3344001: a pointer circles over the tilted card, one turn each CIRCLE_TURN_MS, at this share of the card's box from its middle. */
const CIRCLE_TURN_MS = 2_500;
const CIRCLE_REACH = 0.45;
/** 4x CPU throttling, Chrome DevTools Protocol, reset after every test. */
const THROTTLED = 4;
const UNTHROTTLED = 1;

/** rAF deltas over SAMPLE_MS in the browser: what the player feels, as the director's adapt loop sees it. */
function sampleFrames(): Cypress.Chainable<number[]> {
  return cy.window().then(
    (win) =>
      new Cypress.Promise<number[]>((resolve) => {
        const deltas: number[] = [];
        const start = win.performance.now();
        let last = start;
        const tick = (): void => {
          const now = win.performance.now();
          deltas.push(now - last);
          last = now;
          if (now - start < SAMPLE_MS) win.requestAnimationFrame(tick);
          else resolve(deltas);
        };
        win.requestAnimationFrame(tick);
      }),
  );
}

function mean(values: readonly number[]): number {
  return values.reduce((a, b) => a + b, 0) / Math.max(1, values.length);
}

/** A sample's mean frame time (its first delta, which runs from the sample's start, left out) and the line the log carries. */
function summary(label: string, deltas: readonly number[]): { meanMs: number; worstMs: number; line: string } {
  const frames = deltas.slice(1);
  expect(frames.length, `${label}: frames sampled`).to.be.greaterThan(10);
  const sorted = [...frames].sort((a, b) => a - b);
  const dist = `min=${sorted[0]?.toFixed(0)} p50=${sorted[Math.floor(sorted.length / 2)]?.toFixed(0)} p90=${sorted[Math.floor(sorted.length * 0.9)]?.toFixed(0)} max=${sorted[sorted.length - 1]?.toFixed(0)}`;
  return { meanMs: mean(frames), worstMs: sorted[sorted.length - 1] ?? 0, line: `${label}: mean frame ${mean(frames).toFixed(1)}ms over ${String(frames.length)} frames [${dist}]` };
}

/**
 * R3351000: a moment holds the shared budget when its mean frame time is within
 * FX_PHONE_FRAME_BUDGET_MS and its worst single frame within FX_PHONE_FRAME_WORST_MS in any of
 * FRAME_ATTEMPTS plays. `scene` plays the moment and returns a sample of it; a replay first
 * rerenders the plain board, since the runner only plays events it has not seen. A frame time is a
 * measurement a loaded runner inflates and noise only ever adds time, so the lowest of a few
 * windows is the moment's own cost; a moment that really breaks the budget breaks it in every
 * window. Each sample is logged, as the report cites them.
 */
function holdsLivingBudget(label: string, scene: (attempt: number) => Cypress.Chainable<number[]>, attempt = 1): void {
  scene(attempt).then((deltas) => {
    const { meanMs, worstMs, line } = summary(label, deltas);
    const full = `${line} worst ${worstMs.toFixed(0)}ms`;
    cy.log(`${full} (attempt ${String(attempt)})`);
    cy.then(() => {
      Cypress.log({ name: "frame times", message: `${full} (attempt ${String(attempt)})` });
    });
    if ((meanMs <= FX_PHONE_FRAME_BUDGET_MS && worstMs <= FX_PHONE_FRAME_WORST_MS) || attempt >= FRAME_ATTEMPTS) {
      expect(meanMs, line).to.be.lessThanOrEqual(FX_PHONE_FRAME_BUDGET_MS);
      expect(worstMs, `${label}: worst single frame`).to.be.lessThanOrEqual(FX_PHONE_FRAME_WORST_MS);
      return;
    }
    holdsLivingBudget(label, scene, attempt + 1);
  });
}

const HULL = "stinkyscape-t-dormant-starship";
const FUSED = [HULL, "stinkyscape-140", "stinkyscape-141"];
// A Beam, a Repeater, a Cannon, a Flak gun and the four Launchers: every gun fires.
const GUNS = ["stinkyscape-050", "stinkyscape-051", "stinkyscape-052", "stinkyscape-053", "stinkyscape-140", "stinkyscape-141", "stinkyscape-142", "stinkyscape-143"];
const CORE_X = "stinkyscape-036";
const SHIP_LAYERS = ["stinkyscape-t-meltdown-layer", "stinkyscape-t-propagation-layer", "stinkyscape-t-shield-layer"];
const ROCK = "stinkyscape-169";
/** The tilted hand card: a Mythic, whose Default face has depth (R3344001, R3343001). */
const TILTED = "hand-card-fx-tilt";

/**
 * Two Legendary and three Mythic Units, none wearing an aura of its own, so each wears its tile
 * aura (R3342000): Silly Silas, Felinor Fiender, Tylannt, Paragon-X and Prototype-X2.
 */
const LIVING_UNITS = ["core-052", "core-092", "stinkyscape-006", "stinkyscape-074", "stinkyscape-086"] as const;

/** The living board: five art-first Units with tile auras, a launched-when-asked Starship, ten cards in hand. */
function livingBoard(shipLaunched: number): PlayerView {
  const [first, second, third, fourth, fifth] = LIVING_UNITS;
  const ship =
    shipLaunched === 0
      ? dormantStarship("p2", FUSED, { instanceId: "fx-ship" })
      : unit("p2", {
          instanceId: "fx-ship",
          defId: HULL,
          attack: 3,
          starship: { pieces: GUNS.map((defId) => ({ defId })), name: "Ember of Departure", launched: 1 },
        });
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", { instanceId: "fx-unit-1", defId: first }),
        unit("p1", { instanceId: "fx-unit-2", defId: second }),
        unit("p1", { instanceId: "fx-unit-3", defId: third }),
        unit("p1", { instanceId: "fx-unit-4", defId: fourth }),
        unit("p1", { instanceId: "fx-unit-5", defId: fifth }),
      ],
      backrow: [
        faceUpBackrow("p1", { defId: "core-014", type: "Field Spell" }),
        faceUpBackrow("p1", { defId: "classic-042", type: "Field Spell" }),
        faceDownBackrow,
        faceDownBackrow,
        faceUpBackrow("p1", { defId: "core-084", type: "Trap", radiant: true }),
      ],
      hand: [
        card({ instanceId: "fx-tilt", defId: "core-100", cost: 10 }),
        card({ instanceId: "fx-hand-2", defId: "core-019", cost: 3, radiant: true }),
        card({ instanceId: "fx-hand-3", defId: "core-055", cost: 0 }),
        card({ instanceId: "fx-hand-4", defId: "core-077", cost: 6 }),
        card({ instanceId: "fx-fly", defId: "core-002", cost: 1 }),
        card({ instanceId: "fx-hand-6", defId: "core-003", cost: 2 }),
        card({ instanceId: "fx-hand-7", defId: "core-005", cost: 2 }),
        card({ instanceId: "fx-hand-8", defId: "core-007", cost: 4 }),
        card({ instanceId: "fx-hand-9", defId: "core-009", cost: 5 }),
        card({ instanceId: "fx-hand-10", defId: "core-011", cost: 7 }),
      ],
    }),
    opponent: emptySide("p2", {
      units: [
        unit("p2", { instanceId: "fx-opp-1", defId: "core-008", attack: 4, maxHealth: 4, health: 4 }),
        unit("p2", { instanceId: "fx-opp-2", defId: "core-013", attack: 3, maxHealth: 5, health: 2 }),
        ship,
        unit("p2", { instanceId: "fx-opp-4", defId: "core-022", attack: 3, maxHealth: 2, health: 2 }),
        unit("p2", { instanceId: "fx-opp-5", defId: "core-040", attack: 1, maxHealth: 1, health: 1 }),
      ],
      backrow: [faceDownBackrow, faceDownBackrow, faceDownBackrow, faceDownBackrow, faceDownBackrow],
    }),
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

/** Mounts the living board and returns its player: rerendering the plain board resets the runner, replaying events plays them again. */
function mountLiving(initial: PlayerView): { play: (shown: PlayerView, events: GameEvent[]) => void; rerender: (view: PlayerView) => void } {
  const lookup = lookupFromDefs(CATALOG);
  const noop = (): void => undefined;
  const board = (view: PlayerView) => (
    <CatalogContext.Provider value={lookup}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>
    </CatalogContext.Provider>
  );
  let rerender: ((view: PlayerView) => void) | null = null;
  cy.mount(board(initial)).then(({ rerender: r }) => {
    cy.get(LAYER).should("have.attr", "data-fx", "on");
    cy.then(() => {
      rerender = (view: PlayerView) => r(board(view));
    });
  });
  return {
    play: (shown: PlayerView, events: GameEvent[]) => {
      cy.then(() => {
        rerender?.(withEvents(shown, events));
      });
    },
    rerender: (view: PlayerView) => {
      cy.then(() => {
        rerender?.(view);
      });
    },
  };
}

/** Circles the pointer over the tilted card for the whole scenario, so its face tilts under its glare. */
function startTilt(): { stop: () => void } {
  let stop = (): void => undefined;
  cy.get(`[data-testid="${TILTED}"]`).then(($card) => {
    const element = $card[0] as HTMLElement;
    const win = element.ownerDocument.defaultView as Window & typeof globalThis;
    let frame = 0;
    let running = true;
    const circle = (time: number): void => {
      if (!running) return;
      const box = element.getBoundingClientRect();
      const angle = (2 * Math.PI * time) / CIRCLE_TURN_MS;
      element.dispatchEvent(
        new win.PointerEvent("pointermove", {
          bubbles: true,
          pointerType: "mouse",
          clientX: box.left + box.width * (0.5 + CIRCLE_REACH * Math.cos(angle)),
          clientY: box.top + box.height * (0.5 + CIRCLE_REACH * Math.sin(angle)),
        }),
      );
      frame = win.requestAnimationFrame(circle);
    };
    element.dispatchEvent(new win.PointerEvent("pointerenter", { pointerType: "mouse" }));
    frame = win.requestAnimationFrame(circle);
    stop = () => {
      running = false;
      win.cancelAnimationFrame(frame);
    };
  });
  cy.get(`[data-testid="${TILTED}"] .cf`).should("have.attr", "data-tilt", "on");
  return {
    stop: () => {
      cy.then(() => {
        stop();
      });
    },
  };
}

describe("R3351000 the living game holds the phone budget", () => {
  // A scene that changes the effects settings (the broadside's speed) has them put back after it,
  // a failed one included; throttling is reset too, and the shed stage with it.
  afterEach(() => {
    setFxSettings({ ...DEFAULT_FX_SETTINGS });
    window.localStorage.removeItem(FX_SETTINGS_KEY);
    setShedLevel(0);
    cy.then(() =>
      Cypress.automation("remote:debugger:protocol", { command: "Emulation.setCPUThrottlingRate", params: { rate: UNTHROTTLED } }),
    );
  });

  for (const size of SIZES) {
    it(`R3351000 the living game holds the phone budget at ${size.label} under 4x throttling`, () => {
      cy.viewport(size.width, size.height);
      cy.then(() =>
        Cypress.automation("remote:debugger:protocol", { command: "Emulation.setCPUThrottlingRate", params: { rate: THROTTLED } }),
      );
      const dormant = livingBoard(0);
      const launched = livingBoard(1);
      // The five Units wear tile auras and the backrow wears field auras.
      expect(planTileAuras(dormant).length).to.be.greaterThan(4);
      expect(planAuras(dormant, new Map()).length).to.be.greaterThan(0);
      const { play, rerender } = mountLiving(dormant);
      const tilt = startTilt();
      cy.get(".tile-aura", { timeout: 4_000 }).should("have.length.greaterThan", 4);
      cy.get(CANVAS).should("exist");
      // The scenario starts from a settled board: the mount's opening deal, if any, is over.
      cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");

      /**
       * Plays the moment's events over `shown` and samples its flight. A retry resets the board
       * first, since the runner only plays events it has not seen: replaying the same events over
       * the same board would sample an idle board and pass for nothing.
       */
      const moment = (label: string, shown: PlayerView, events: GameEvent[], flight: () => void): void => {
        holdsLivingBudget(label, (attempt) => {
          if (attempt > 1) {
            rerender(shown);
            cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
          }
          play(shown, events);
          flight();
          return sampleFrames();
        });
        cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
      };

      // The Anomaly's cast lays the backdrop the board stands on from here.
      const castMine: GameEvent[] = dormant.opponent.units.flatMap((u): GameEvent[] =>
        u === null ? [] : [{ type: "exiled", instanceId: u.instanceId, defId: u.defId, owner: u.owner }],
      );
      const castRocks: GameEvent[] = [1, 2, 3, 4, 5].map((lane) => ({
        type: "summoned",
        player: "p2",
        instanceId: `fx-rock-${String(lane)}`,
        defId: ROCK,
        row: "units",
        lane,
      }));
      moment("Anomaly cast", dormant, [
        { type: "cardPlayed", player: "p2", instanceId: "fx-anomaly", defId: ANOMALY_FOUND_ID, costPaid: 4 },
        { type: "boardReplaced", player: "p2", anomaly: "AT" },
        ...castMine,
        ...castRocks,
        { type: "cardResolved", player: "p2", instanceId: "fx-anomaly", defId: ANOMALY_FOUND_ID, permanent: false, costPaid: 4 },
      ], () => {
        cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "anomaly");
        cy.get(`${FX_DOM} [data-fx="rift"]`).should("exist");
      });

      moment("Mythic takeover", dormant, [
        { type: "cardPlayed", player: dormant.viewer, instanceId: "fx-yile", defId: "meditative-049", costPaid: 2 },
        { type: "summoned", player: dormant.viewer, instanceId: "fx-yile", defId: "meditative-049", row: "units", lane: 3 },
      ], () => {
        cy.get(GAME).should("have.attr", "data-takeover", "mythic");
        cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", "ai");
        cy.get(`${FX_DOM} [data-fx="takeover"] .fx-takeover-planes`).should("exist");
        cy.get(CANVAS).should("exist");
      });

      moment("Starship launch", launched, [
        { type: "awakened", player: "p2", instanceId: "fx-ship", defId: HULL },
        { type: "starshipLaunched", player: "p2", instanceId: "fx-ship", defId: HULL, name: "Ember of Departure", pieces: FUSED },
      ], () => {
        cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "starship");
        cy.get(`${FX_DOM} [data-fx="cinematic"]`).should("exist");
      });

      // At the slowest effects speed the broadside's eight guns stretch over the whole sample.
      cy.then(() => {
        setFxSettings({ speed: FX_SPEED_MIN });
      });
      moment("8-gun broadside", launched, [{ type: "attackDeclared", attackerId: "fx-ship", targetId: "fx-unit-1", forced: false }], () => {
        cy.get('[data-testid="card-fx-ship"]', { timeout: 4_000 }).should("have.attr", "data-animating", "attackDeclared");
        cy.get(CANVAS).should("exist");
      });
      // Full speed again: the broadside alone wants the guns stretched over the sample.
      cy.then(() => {
        setFxSettings({ speed: FX_SPEED_DEFAULT });
      });

      moment("Drone Battleship arrival", launched, [
        { type: "cardPlayed", player: "p1", instanceId: "fx-core", defId: CORE_X, costPaid: 10 },
        ...SHIP_LAYERS.map(
          (defId, k): GameEvent => ({ type: "summoned", player: "p1", instanceId: `fx-layer-${String(k)}`, defId, row: "units", lane: 1 }),
        ),
        { type: "cardResolved", player: "p1", instanceId: "fx-core", defId: CORE_X, permanent: false, costPaid: 10 },
      ], () => {
        // The sample opens with the first layer's beat and runs through the other two.
        cy.get("[data-animating]", { timeout: 4_000 }).should("exist");
        cy.get(CANVAS).should("exist");
      });

      moment("Ceaseless Void fold", launched, [
        { type: "cardPlayed", player: launched.viewer, instanceId: "fx-void", defId: "core-100", costPaid: 0 },
        { type: "hookResolving", sourceId: "fx-void", defId: "core-100", controller: launched.viewer, hook: "cry" },
        ...removals(launched, "exiled"),
        { type: "cardResolved", player: launched.viewer, instanceId: "fx-void", defId: "core-100", permanent: true, costPaid: 0 },
      ], () => {
        cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "voidFold");
        cy.get(`${FX_DOM} [data-fx="setPiece"]`).should("have.attr", "data-shape", "fold");
        cy.get(CANVAS).should("exist");
      });

      moment("Hand-to-tile flight", launched, [
        { type: "cardPlayed", player: "p1", instanceId: "fx-fly", defId: "core-002", costPaid: 1 },
        { type: "summoned", player: "p1", instanceId: "fx-fly", defId: "core-002", row: "units", lane: 5 },
      ], () => {
        cy.get(`${FX_DOM} [data-fx="hold"]`, { timeout: 4_000 }).should("exist");
      });

      moment("Riffle", launched, [
        { type: "traded", player: "p1" },
        { type: "shuffledIn", player: "p1", instanceId: "s1", defId: "core-070", position: 1 },
        { type: "drawn", player: "p1", instanceId: "cY", defId: "core-055" },
      ], () => {
        cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-riffle", "you");
      });

      tilt.stop();
      // Still tilted by the last pose until the pointer leaves, then back to level.
      cy.get(`[data-testid="${TILTED}"]`).then(($card) => {
        $card[0]?.dispatchEvent(new Event("pointerleave"));
      });
      cy.get(`[data-testid="${TILTED}"] .cf`).should("not.have.attr", "data-tilt");
    });
  }

  it("R3351000 a forced shed sheds motes first and a set piece's beats never", () => {
    // The trip from slow windows to these same stages is the director's unit-pinned half; here the
    // stages are forced through the store the director publishes, so the living board's shedding
    // reads deterministically: unthrottled, one viewport.
    cy.viewport(390, 844);
    const living = livingBoard(1);
    const { play } = mountLiving(living);
    cy.get(".tile-aura", { timeout: 4_000 }).should("have.length.greaterThan", 4);
    // A Mythic takeover plays its beats through the stages below.
    const takeover = (instanceId: string, lane: number): GameEvent[] => [
      { type: "cardPlayed", player: living.viewer, instanceId, defId: "meditative-049", costPaid: 2 },
      { type: "summoned", player: living.viewer, instanceId, defId: "meditative-049", row: "units", lane },
    ];
    play(living, takeover("fx-yile", 3));
    cy.get(`${FX_DOM} [data-fx="takeover"]`).should("exist");
    const auraMoteCount = (): Cypress.Chainable<number> =>
      cy.get('[data-testid="aura-layer"]').then(($layer) => Number($layer.attr("data-aura-motes")));
    let tileMotes = 0;
    let auraMotes = 0;
    let movingFaces = 0;
    cy.get(".tile-aura-mote").its("length").then((n) => {
      tileMotes = n;
    });
    auraMoteCount().then((n) => {
      auraMotes = n;
    });
    cy.get('[data-art-moving="true"]').its("length").then((n) => {
      movingFaces = n;
    });
    cy.then(() => {
      expect(tileMotes, "tile motes at full").to.be.greaterThan(0);
      expect(auraMotes, "aura motes at full").to.be.greaterThan(0);
      expect(movingFaces, "moving faces at full").to.be.greaterThan(0);
    });
    // Stage 1 sheds mote emission in the director alone: every DOM face, aura and tile stands as it was.
    // (Callback assertions: the baselines are assigned by an earlier command, so only a callback
    // reads them after that command ran.)
    cy.then(() => {
      setShedLevel(1);
    });
    cy.get(".tile-aura-mote").its("length").should((n) => {
      expect(n).to.eq(tileMotes);
    });
    auraMoteCount().should((n) => {
      expect(n).to.eq(auraMotes);
    });
    cy.get('[data-art-moving="true"]').its("length").should((n) => {
      expect(n).to.eq(movingFaces);
    });
    // Stage 2 halves the far auras' and tiles' motes — no pointer moves here, so every aura counts
    // as far — while the faces still move.
    cy.then(() => {
      setShedLevel(2);
    });
    cy.get(".tile-aura-mote").its("length").should((n) => {
      expect(n).to.be.lessThan(tileMotes);
    });
    auraMoteCount().should((n) => {
      expect(n).to.be.lessThan(auraMotes);
    });
    cy.get('[data-art-moving="true"]').its("length").should((n) => {
      expect(n).to.eq(movingFaces);
    });
    // The takeover ends before the deepest stage, so its beats can mount under it.
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
    cy.get(`${FX_DOM} [data-fx="takeover"]`).should("not.exist");
    // Stage 3 stills every face past the pointer's own — no hover in this run, so all of them —
    // while a fresh takeover's beats still mount.
    cy.then(() => {
      setShedLevel(3);
    });
    play(living, takeover("fx-yile-2", 4));
    cy.get(`${FX_DOM} [data-fx="takeover"]`).should("exist");
    cy.get('[data-art-moving="true"]').should("have.length", 0);
    cy.get(".tile-aura-mote").its("length").should((n) => {
      expect(n).to.be.lessThan(tileMotes);
    });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
  });
});
