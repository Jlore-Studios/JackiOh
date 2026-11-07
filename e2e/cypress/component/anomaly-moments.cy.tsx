// R3337002, R3337003: discovering an Anomaly over a real board, at desktop size and at the phone's
// 390x844. A Scanning Probe's scan finds an Anomaly Found! (plain or Radiant) or nothing, on its owner's
// seat and on the other seat, which sees the sweep alone (R202); an Anomaly's cast tears its rift, stamps
// its title card and lands its board, for three Anomalies; Big moments "short" and "off" and Reduce
// Motion each play their own form; and the board that stands wears its backdrop.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/anomaly-moments.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { ANOMALIES, ANOMALY_FOUND_ID, type AnomalyCode } from "../../../apps/web/src/cards/anomalies.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const VIEWPORTS = [
  { name: "desktop", width: 1280, height: 720 },
  { name: "phone", width: 390, height: 844 },
] as const;
const LAYER = '[data-testid="fx-layer"]';
const FX_DOM = '[data-testid="fx-dom"]';
const GAME = '[data-testid="game"]';
const PROBE = "stinkyscape-166";
/** A Tiny Asteroid, a card each asteroid board lays. */
const ROCK = "stinkyscape-169";

/**
 * Mounts the board, then plays `events` over `shown`, as the effects layer's own specs do. Every effect the
 * layer mounts from then on is recorded in the list returned, as `kind` and `kind:side`: a find's ghost lives
 * about a tenth of a second, which a query between two others can miss.
 */
function mountScene(initial: PlayerView, shown: PlayerView, events: GameEvent[]): string[] {
  const lookup = lookupFromDefs(CATALOG);
  const noop = (): void => undefined;
  const mounted: string[] = [];
  const board = (view: PlayerView) => (
    <CatalogContext.Provider value={lookup}>
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>
    </CatalogContext.Provider>
  );
  cy.mount(board(initial)).then(({ rerender }) => {
    cy.get(LAYER).should("have.attr", "data-fx", "on");
    cy.get(FX_DOM).then(($dom) => {
      const record = new MutationObserver((records) => {
        for (const added of records.flatMap((change) => [...change.addedNodes])) {
          if (!(added instanceof Element)) continue;
          const kind = added.getAttribute("data-fx") ?? "";
          mounted.push(kind, `${kind}:${added.getAttribute("data-side") ?? ""}`);
        }
      });
      record.observe($dom[0] as Element, { childList: true });
      rerender(board(withEvents(shown, events)));
    });
  });
  return mounted;
}

/** `view` with an Anomaly Found! `fx-found` in its viewer's hand, as the next view after a find shows it. */
function holdingFound(view: PlayerView, radiant: boolean): PlayerView {
  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  return { ...view, you: { ...view.you, hand: [...hand, { instanceId: "fx-found", defId: ANOMALY_FOUND_ID, radiant, cost: 4 }] } };
}

/** A probe's play by `player`, what its scan adds (nothing, or one `addedToHand`), and its end. */
function scan(player: "p1" | "p2", found: GameEvent[]): GameEvent[] {
  return [
    { type: "cardPlayed", player, instanceId: "fx-probe", defId: PROBE, costPaid: 2 },
    ...found,
    { type: "cardResolved", player, instanceId: "fx-probe", defId: PROBE, permanent: false, costPaid: 2 },
  ];
}

/** p1's cast of an Anomaly Found! laying `code`: its side exiled, five asteroids landing in its unit zones. */
function cast(view: PlayerView, code: AnomalyCode): GameEvent[] {
  const me = view.viewer;
  const mine = view.you.units.flatMap((u): GameEvent[] => (u === null ? [] : [{ type: "exiled", instanceId: u.instanceId, defId: u.defId, owner: u.owner }]));
  const rocks: GameEvent[] = [1, 2, 3, 4, 5].map((lane) => ({ type: "summoned", player: me, instanceId: `fx-rock-${String(lane)}`, defId: ROCK, row: "units", lane }));
  return [
    { type: "cardPlayed", player: me, instanceId: "fx-anomaly", defId: ANOMALY_FOUND_ID, costPaid: 4 },
    { type: "boardReplaced", player: me, anomaly: code },
    ...mine,
    ...rocks,
    { type: "cardResolved", player: me, instanceId: "fx-anomaly", defId: ANOMALY_FOUND_ID, permanent: false, costPaid: 4 },
  ];
}

/**
 * The viewport mid-effect, the page running on through the capture as the other effect specs' captures do. A
 * capture stalls the page for a moment, which can drop a cue of a tenth of a second in the scene it is taken
 * in or the next, so the scans' captures are taken after every scan's checks (`SCANS`).
 */
function shot(name: string): void {
  cy.screenshot(name, { capture: "viewport", overwrite: true, disableTimersAndAnimations: false });
}

/** The title card the effects layer stamps for `code`: its name (a `data-text`, no text node). */
function title(code: AnomalyCode): string {
  return `${FX_DOM} [data-fx="anomalyTitle"] .fx-anomalyTitle-name[data-text="${ANOMALIES[code].name}"]`;
}

/** A find of a plain or a Radiant Anomaly Found!, on the owner's seat. */
function found(view: PlayerView, radiant: boolean): { shown: PlayerView; events: GameEvent[] } {
  return { shown: holdingFound(view, radiant), events: scan("p1", [{ type: "addedToHand", player: "p1", instanceId: "fx-found", defId: ANOMALY_FOUND_ID }]) };
}

/** The opponent's find: the other seat's `addedToHand` names R97's sentinel. */
function opponentFind(view: PlayerView): { shown: PlayerView; events: GameEvent[] } {
  return { shown: view, events: scan("p2", [{ type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" }]) };
}

/** Each scan's scene and the effect its capture waits for. */
const SCANS = [
  { name: "found", scene: (view: PlayerView) => found(view, false), shows: '[data-fx="radar"][data-side="you"]' },
  { name: "radiant", scene: (view: PlayerView) => found(view, true), shows: '[data-fx="rays"]' },
  { name: "nothing", scene: (view: PlayerView) => ({ shown: view, events: scan("p1", []) }), shows: '[data-fx="radar"][data-side="you"]' },
  { name: "opponent", scene: opponentFind, shows: '[data-fx="radar"][data-side="opponent"]' },
] as const;

describe("R3337003 discovering an Anomaly over a real board", () => {
  afterEach(() => {
    setFxSettings(DEFAULT_FX_SETTINGS);
  });

  // R3337002: the scans' checks, at both sizes, before any capture (`shot`).
  for (const vp of VIEWPORTS) {
    describe(`R3337002 a Scanning Probe's scan at ${vp.name} size (${String(vp.width)}x${String(vp.height)})`, () => {
      beforeEach(() => {
        cy.viewport(vp.width, vp.height);
      });

      it("R3337002 the owner's scan that finds an Anomaly sweeps, pings and flies the card into the hand", () => {
        const view = fullBoardView();
        const { shown, events } = found(view, false);
        const mounted = mountScene(view, shown, events);
        cy.get(`${FX_DOM} [data-fx="radar"][data-side="you"]`, { timeout: 4_000 }).should("exist");
        cy.wrap(mounted, { timeout: 4_000 }).should("include", "ghost");
        cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
        cy.get(`${FX_DOM} [data-fx="radar"]`, { timeout: 4_000 }).should("not.exist");
        cy.wrap(mounted).should("not.include", "rays");
      });

      it("R3337002 the owner's scan that finds a Radiant Anomaly flashes as the card lands", () => {
        const view = fullBoardView();
        const { shown, events } = found(view, true);
        const mounted = mountScene(view, shown, events);
        cy.get(`${FX_DOM} [data-fx="radar"]`, { timeout: 4_000 }).should("exist");
        cy.wrap(mounted, { timeout: 4_000 }).should("include", "ghost").and("include", "rays");
        cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
      });

      it("R3337002 a scan that finds nothing sweeps and fades", () => {
        const view = fullBoardView();
        const mounted = mountScene(view, view, scan("p1", []));
        cy.get(`${FX_DOM} [data-fx="radar"]`, { timeout: 4_000 }).should("exist");
        cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
        cy.get(`${FX_DOM} [data-fx="radar"]`, { timeout: 4_000 }).should("not.exist");
        cy.wrap(mounted).should("not.include", "ghost").and("not.include", "rays");
      });

      it("R3337002 R202 the opponent's find draws the sweep alone, with no ping and no card", () => {
        const view = fullBoardView();
        const { shown, events } = opponentFind(view);
        const mounted = mountScene(view, shown, events);
        cy.get(`${FX_DOM} [data-fx="radar"][data-side="opponent"]`, { timeout: 4_000 }).should("exist");
        cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
        cy.wrap(mounted).should("include", "radar:opponent").and("not.include", "radar:you").and("not.include", "ghost").and("not.include", "rays");
      });
    });
  }

  for (const vp of VIEWPORTS) {
    describe(`at ${vp.name} size (${String(vp.width)}x${String(vp.height)})`, () => {
      beforeEach(() => {
        cy.viewport(vp.width, vp.height);
      });

      for (const code of ["WH", "AL", "FO"] as const) {
        it(`R3337003 the cast of ${ANOMALIES[code].name} tears its rift, stamps its title card and lands its board`, () => {
          const view = fullBoardView();
          mountScene(view, view, cast(view, code));
          cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "anomaly");
          cy.get(`${FX_DOM} [data-fx="rift"][data-side="you"]`).should("exist");
          cy.get(title(code), { timeout: 4_000 }).should("exist");
          shot(`anomaly-cast-${code}-${vp.name}`);
          cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
          cy.get(GAME).should("not.have.attr", "data-set-piece");
          cy.get(`${FX_DOM} [data-fx="anomalyTitle"]`, { timeout: 4_000 }).should("not.exist");
        });
      }

      it("R3337003 Big moments short shows the title card and the landing, with no rift", () => {
        setFxSettings({ bigMoments: "short" });
        const view = fullBoardView();
        mountScene(view, view, cast(view, "WH"));
        cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "anomaly");
        cy.get(title("WH"), { timeout: 4_000 }).should("exist");
        cy.get(`${FX_DOM} [data-fx="rift"]`).should("not.exist");
        shot(`anomaly-short-${vp.name}`);
        cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
      });

      it("R3337003 Big moments off shows the title card alone, holding nothing", () => {
        setFxSettings({ bigMoments: "off" });
        const view = fullBoardView();
        mountScene(view, view, cast(view, "WH"));
        cy.get(title("WH"), { timeout: 4_000 }).should("exist");
        cy.get(GAME).should("not.have.attr", "data-set-piece");
        cy.get(`${FX_DOM} [data-fx="rift"]`).should("not.exist");
        shot(`anomaly-off-${vp.name}`);
        cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
      });

      it("R3337003 Reduce Motion stands one still title card", () => {
        setFxSettings({ motion: "reduce" });
        const view = fullBoardView();
        const lookup = lookupFromDefs(CATALOG);
        const noop = (): void => undefined;
        const board = (shown: PlayerView) => (
          <CatalogContext.Provider value={lookup}>
            <div className="app-shell app-shell--wide">
              <Game view={shown} legal={[]} onAction={noop} />
            </div>
          </CatalogContext.Provider>
        );
        // A still layer has no canvas: its DOM is the one sign it is up.
        cy.mount(board(view)).then(({ rerender }) => {
          cy.get(FX_DOM).should("have.attr", "data-still", "true");
          cy.then(() => rerender(board(withEvents(view, cast(view, "FO")))));
        });
        cy.get(`${FX_DOM} [data-fx="anomalyTitle"][data-still="true"]`, { timeout: 4_000 }).should("be.visible");
        cy.get(`${FX_DOM} [data-fx="rift"]`).should("not.exist");
        cy.get(GAME).should("not.have.attr", "data-set-piece");
        shot(`anomaly-still-${vp.name}`);
      });

      it("R3337003 the board that stands wears its Anomaly's backdrop", () => {
        const view = fullBoardView();
        mountScene(view, view, cast(view, "WH"));
        // The cast is under way before it is waited out: the board shows the laid view once it settles.
        cy.get(GAME, { timeout: 4_000 }).should("have.attr", "data-set-piece", "anomaly");
        cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
        cy.get('[data-testid="aura-anomaly-p1"]', { timeout: 4_000 }).should("have.attr", "data-anomaly", "WH").and("have.attr", "data-aura-scope", "side");
        shot(`anomaly-backdrop-${vp.name}`);
      });

      it("R3337002 the scans, captured", () => {
        for (const scanned of SCANS) {
          const view = fullBoardView();
          const { shown, events } = scanned.scene(view);
          mountScene(view, shown, events);
          cy.get(`${FX_DOM} ${scanned.shows}`, { timeout: 4_000 }).should("exist");
          shot(`anomaly-scan-${scanned.name}-${vp.name}`);
          cy.get("[data-animating]", { timeout: 8_000 }).should("not.exist");
        }
      });
    });
  }
});
