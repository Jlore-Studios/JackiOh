// R2803000: the audit's phone frame-time spec. At the phone particle cap with a phone viewport,
// every scene holds its mean frame time within FX_PHONE_FRAME_BUDGET_MS: the cap check, Twisting
// Nether (scaled back for the budget: fewer pull/implode particles, a softer shake, and a smaller
// vortex spiral with its mask retuned to the same visible arms), the Starship launch, the Drone
// Battleship's arrival (R3336000), a Mythic takeover (fewer Mythic-burst particles, a softer shake, and takeover rays trimmed to the same
// visible disc), a board with 4 auras, and R3337003's Anomaly cast (its rift, flare, title card and landing).
// R3332001: Felinor Fiender's signature, the first, over the gallery's board with its three Felinors.
// R3338000: Ceaseless Void's fold and Dice of Doom's 67.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/fx-phone-frame.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { planAuras } from "../../../apps/web/src/fx/aura/plan.ts";
import { FX_CARD_TABLE } from "../../../apps/web/src/fx/cards/tables.ts";
import { FX_PARTICLE_CAP_MOBILE, FX_PHONE_FRAME_BUDGET_MS } from "../../../apps/web/src/fx/constants.ts";
import { capacityFor } from "../../../apps/web/src/fx/director.ts";
import { doomFixture, galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayOf } from "../../../apps/web/src/fx/motifs/gallery.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import {
  baseView,
  dormantStarship,
  emptySide,
  faceUpBackrow,
  fullBoardView,
  unit,
  withEvents,
} from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const PHONE = { width: 390, height: 844 };
const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';
/** Long enough to cross a set piece's hold, short enough to stay mid-flight. */
const SAMPLE_MS = 2_000;
/** How many fresh mounts of a scene may be sampled before its budget is called broken (see `holdsBudget`). */
const FRAME_ATTEMPTS = 3;

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
function summary(label: string, deltas: readonly number[]): { meanMs: number; line: string } {
  const frames = deltas.slice(1);
  expect(frames.length, `${label}: frames sampled`).to.be.greaterThan(10);
  const sorted = [...frames].sort((a, b) => a - b);
  const dist = `min=${sorted[0]?.toFixed(0)} p50=${sorted[Math.floor(sorted.length / 2)]?.toFixed(0)} p90=${sorted[Math.floor(sorted.length * 0.9)]?.toFixed(0)} max=${sorted[sorted.length - 1]?.toFixed(0)}`;
  return { meanMs: mean(frames), line: `${label}: mean frame ${mean(frames).toFixed(1)}ms over ${String(frames.length)} frames [${dist}]` };
}

/**
 * R2803000: a scene holds its budget when its mean frame time is within FX_PHONE_FRAME_BUDGET_MS in any of
 * FRAME_ATTEMPTS fresh mounts. `scene` mounts the scene, waits for it to be in flight and returns a sample of it.
 * A frame time is a measurement a loaded runner inflates (a neighbour's burst, a stalled page: measured at a
 * mean of 73 ms with one 2,029 ms frame among 43, and at 75 ms with p50 43 ms, where the same scene runs at
 * about 17 ms alone), and noise only ever adds time, so the lowest of a few windows is the scene's own cost; a
 * scene that really breaks the budget breaks it in every window. Each sample is logged, as the report cites them.
 */
function holdsBudget(label: string, scene: () => Cypress.Chainable<number[]>, attempt = 1): void {
  scene().then((deltas) => {
    const { meanMs, line } = summary(label, deltas);
    cy.log(`${line} (attempt ${String(attempt)})`);
    cy.then(() => {
      Cypress.log({ name: "frame times", message: `${line} (attempt ${String(attempt)})` });
    });
    if (meanMs <= FX_PHONE_FRAME_BUDGET_MS || attempt >= FRAME_ATTEMPTS) {
      expect(meanMs, line).to.be.lessThanOrEqual(FX_PHONE_FRAME_BUDGET_MS);
      return;
    }
    holdsBudget(label, scene, attempt + 1);
  });
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

describe("R2803000 the phone scenes hold the frame budget", () => {
  it("R2803000 the phone viewport draws at the phone particle cap", () => {
    expect(capacityFor(PHONE.width)).to.eq(FX_PARTICLE_CAP_MOBILE);
    cy.viewport(PHONE.width, PHONE.height);
    cy.window().then((win) => {
      expect(win.innerWidth).to.eq(PHONE.width);
    });
  });

  it("R2803000 Twisting Nether holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    const me = view.viewer;
    holdsBudget("Twisting Nether", () => {
      mountScene(view, view, [
        { type: "cardPlayed", player: me, instanceId: "fx-clear", defId: "core-088", costPaid: 4 },
        { type: "hookResolving", sourceId: "fx-clear", defId: "core-088", controller: me, hook: "cry" },
        ...removals(view, "destroyed"),
        { type: "cardResolved", player: me, instanceId: "fx-clear", defId: "core-088", permanent: false, costPaid: 4 },
      ]);
      cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "nether");
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
    cy.get(GAME).should("not.have.attr", "data-set-piece");
  });

  it("R2803000 the Starship launch holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
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
    holdsBudget("Starship launch", () => {
      mountScene(ship(0), ship(1), [
        { type: "awakened", player: "p1", instanceId: "fx-ship", defId: HULL },
        { type: "starshipLaunched", player: "p1", instanceId: "fx-ship", defId: HULL, name: "Ember of Departure", pieces: fused },
      ]);
      cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "starship");
      cy.get(`${FX_DOM} [data-fx="launch"] .fx-launch-count`).should("have.length", 3);
      return sampleFrames();
    });
  });

  it("R3336000 the Drone Battleship's arrival holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const CORE_X = "stinkyscape-036";
    const layers = ["stinkyscape-t-meltdown-layer", "stinkyscape-t-propagation-layer", "stinkyscape-t-shield-layer"];
    const arrived = (done: boolean): PlayerView =>
      baseView({
        you: emptySide("p1", {
          units: [done ? unit("p1", { instanceId: "fx-shield", defId: layers[2], attack: 0, armor: 4, buried: 2 }) : null, null, null, null, null],
        }),
        opponent: emptySide("p2"),
      });
    holdsBudget("Drone Battleship arrival", () => {
      mountScene(arrived(false), arrived(true), [
        { type: "cardPlayed", player: "p1", instanceId: "fx-core", defId: CORE_X, costPaid: 10 },
        ...layers.map((defId, k): GameEvent => ({ type: "summoned", player: "p1", instanceId: `fx-layer-${String(k)}`, defId, row: "units", lane: 1 })),
        { type: "cardResolved", player: "p1", instanceId: "fx-core", defId: CORE_X, permanent: false, costPaid: 10 },
      ]);
      // The sample opens with the first layer's beat and runs through the other two.
      cy.get("[data-animating]", { timeout: 4_000 }).should("exist");
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
    cy.get(`${FX_DOM} [data-fx="shield"]`).should("not.exist");
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
  });

  it("R2803000 a Mythic takeover holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    holdsBudget("Mythic takeover", () => {
      mountScene(view, view, [
        { type: "cardPlayed", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", costPaid: 2 },
        { type: "summoned", player: view.viewer, instanceId: "fx-yile", defId: "meditative-049", row: "units", lane: 3 },
      ]);
      cy.get(GAME).should("have.attr", "data-takeover", "mythic");
      cy.get(`${FX_DOM} [data-fx="takeover"]`).should("have.attr", "data-look", "ai");
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
  });

  it("R3337003 an Anomaly's cast holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    const me = view.viewer;
    const mine = view.you.units.flatMap((u): GameEvent[] => (u === null ? [] : [{ type: "exiled", instanceId: u.instanceId, defId: u.defId, owner: u.owner }]));
    const rocks: GameEvent[] = [1, 2, 3, 4, 5].map((lane) => ({ type: "summoned", player: me, instanceId: `fx-rock-${String(lane)}`, defId: "stinkyscape-169", row: "units", lane }));
    holdsBudget("Anomaly cast", () => {
      mountScene(view, view, [
        { type: "cardPlayed", player: me, instanceId: "fx-anomaly", defId: "stinkyscape-t-anomaly-found", costPaid: 4 },
        { type: "boardReplaced", player: me, anomaly: "AT" },
        ...mine,
        ...rocks,
        { type: "cardResolved", player: me, instanceId: "fx-anomaly", defId: "stinkyscape-t-anomaly-found", permanent: false, costPaid: 4 },
      ]);
      cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "anomaly");
      cy.get(`${FX_DOM} [data-fx="rift"]`).should("exist");
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
  });

  it("R3332001 Felinor Fiender's signature holds the phone frame budget mid-flight", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const FIENDER = "core-092";
    const entry = FX_CARD_TABLE.get(FIENDER);
    const play = entry === undefined ? null : galleryPlayOf(FIENDER, entry);
    if (play === null) throw new Error("Felinor Fiender has no gallery play");
    const board = galleryBoardOf(FIENDER, fullBoardView());
    holdsBudget("Felinor Fiender's signature", () => {
      mountScene(board, galleryFieldView(FIENDER, board), galleryCardEvents(FIENDER, play, board, 1));
      cy.get(GAME).find("[data-animating]").should("exist");
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
  });

  it("R3338000 Ceaseless Void holds the phone frame budget", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    const me = view.viewer;
    holdsBudget("Ceaseless Void", () => {
      mountScene(view, view, [
        { type: "cardPlayed", player: me, instanceId: "fx-void", defId: "core-100", costPaid: 0 },
        { type: "hookResolving", sourceId: "fx-void", defId: "core-100", controller: me, hook: "cry" },
        ...removals(view, "exiled"),
        { type: "cardResolved", player: me, instanceId: "fx-void", defId: "core-100", permanent: true, costPaid: 0 },
      ]);
      cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "voidFold");
      cy.get(`${FX_DOM} [data-fx="setPiece"]`).should("have.attr", "data-shape", "fold");
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
    cy.get(GAME).should("not.have.attr", "data-set-piece");
  });

  it("R3338000 Dice of Doom's 67 holds the phone frame budget", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    const me = view.viewer;
    holdsBudget("Dice of Doom's 67", () => {
      mountScene(view, view, [
        { type: "cardPlayed", player: me, instanceId: "fx-doom", defId: "stinkyscape-154", costPaid: 0 },
        ...doomFixture(view, "fx-doom", me, "jackpot"),
        { type: "cardResolved", player: me, instanceId: "fx-doom", defId: "stinkyscape-154", permanent: false, costPaid: 0 },
      ]);
      cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "diceOfDoom");
      cy.get(`${FX_DOM} [data-fx="roll"]`).should("have.attr", "data-look", "doom");
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
    cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
    cy.get(GAME).should("not.have.attr", "data-set-piece");
  });

  it("R2803000 a board with 4 auras holds the phone frame budget", () => {
    cy.viewport(PHONE.width, PHONE.height);
    const view = fullBoardView();
    view.you.backrow = [
      faceUpBackrow("p1", { defId: "core-014", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "classic-042", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "classic-075", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "meditative-040", type: "Field Spell" }),
      faceUpBackrow("p1", { defId: "core-084", type: "Trap", radiant: true }),
    ];
    expect(planAuras(view, new Map()).map((aura) => aura.defId)).to.have.length(4);
    holdsBudget("Four auras", () => {
      mountScene(view, view, []);
      cy.get(`[data-aura-look]`, { timeout: 4_000 }).should("have.length.greaterThan", 0);
      cy.get(CANVAS).should("exist");
      return sampleFrames();
    });
  });
});
