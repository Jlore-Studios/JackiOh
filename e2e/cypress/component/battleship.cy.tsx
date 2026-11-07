// R3336000: the Drone Battleship as one great ship in three decks, played on the board at desktop size and at
// 390x844. The art: a layer's picture is the decks still standing (`data-art-decks` 3, 2, 1 down the pile). The
// beats: the arrival (Drone Core X's three summons, one layer each), a hit redirected onto the Shield (its dome
// flares, the plates glow for what Armor took) and an ordinary hit (a flicker), each Activate of the
// Propagation layer (a bay opens, then the drone leaves it), the Meltdown's tick at two damage levels, a layer
// breaking over the deck beneath, and the reactor's blast. Each scene asserts its cue and ends with nothing
// animating. The planner is battleship.test.ts's and the phone frame budget fx-phone-frame.cy.tsx's.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/battleship.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, emptySide, fullBoardView, unit, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];
type UnitView = NonNullable<PlayerView["you"]["units"][number]>;

const SHIELD = "stinkyscape-t-shield-layer";
const HANGAR = "stinkyscape-t-propagation-layer";
const MELTDOWN = "stinkyscape-t-meltdown-layer";
const INTERCEPTOR = "stinkyscape-t-drone-interceptor";
const CORVETTE = "stinkyscape-t-drone-corvette";
const CORE_X = "stinkyscape-036";

const SIZES = [
  { width: 1280, height: 720 },
  { width: 390, height: 844 },
] as const;

const LAYER = '[data-testid="fx-layer"]';
const CANVAS = '[data-testid="fx-canvas"]';
/** The longest a beat is given to show: an entry, its hold and its tail, with room for a loaded runner. */
const BEAT_MS = 4_000;

/**
 * What the effects layer mounted and the board animated, as it happened: a cue lives for half a second or so, and a
 * loaded runner may sample the DOM between its mount and its removal, so each cue is written down as it is added.
 * A DOM cue reads `kind|move|size|plates=n|lit=n|doors=n|lights=n`; a card the queue marks reads `animating|testid|type`.
 */
function watchFx(): void {
  cy.document().then((doc) => {
    const seen: string[] = [];
    const count = (el: Element, selector: string): number => el.querySelectorAll(selector).length;
    const noteCue = (el: Element): void => {
      const kind = el.getAttribute("data-fx");
      if (kind === null) return;
      const parts = [kind, el.getAttribute("data-move") ?? "", el.getAttribute("data-size") ?? ""];
      parts.push(`plates=${String(count(el, ".fx-ship-plate"))}`, `lit=${String(count(el, '.fx-ship-plate[data-lit="true"]'))}`);
      parts.push(`doors=${String(count(el, ".fx-ship-door"))}`, `lights=${String(count(el, ".fx-ship-light"))}`);
      seen.push(parts.join("|"));
    };
    const noteAnimating = (el: Element): void => {
      const type = el.getAttribute("data-animating");
      if (type !== null) seen.push(`animating|${el.getAttribute("data-testid") ?? ""}|${type}`);
    };
    const view = doc.defaultView;
    if (view === null) throw new Error("the page has no window");
    new view.MutationObserver((records) => {
      for (const record of records) {
        if (record.type === "attributes") {
          noteAnimating(record.target as Element);
          continue;
        }
        record.addedNodes.forEach((node) => {
          if (node.nodeType !== 1) return;
          const el = node as Element;
          noteCue(el);
          el.querySelectorAll("[data-fx]").forEach(noteCue);
          noteAnimating(el);
          el.querySelectorAll("[data-animating]").forEach(noteAnimating);
        });
      }
    }).observe(doc.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-animating"] });
    cy.wrap(seen, { log: false }).as("seen");
  });
}

/** Mounts the board showing `initial`, then rerenders it showing `shown` with `events` to play. */
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
    watchFx();
    cy.then(() => rerender(board(withEvents(shown, events))));
  });
}

/** The scene showed a cue or an animating card `describe`d by `match`, within BEAT_MS. */
function sees(label: string, match: (entry: string) => boolean): void {
  cy.get<string[]>("@seen", { timeout: BEAT_MS }).should((seen) => {
    expect(seen.some(match), `${label}; saw ${seen.join(" ; ")}`).to.eq(true);
  });
}

/** The scene showed no cue that `match` describes (asserted once the scene has settled). */
function neverSees(label: string, match: (entry: string) => boolean): void {
  cy.get<string[]>("@seen").then((seen) => {
    expect(seen.some(match), `${label}; saw ${seen.join(" ; ")}`).to.eq(false);
  });
}

const cue = (kind: string, move = "", size = ""): ((entry: string) => boolean) => (entry) => {
  const [k, m, z] = entry.split("|");
  return k === kind && (move === "" || m === move) && (size === "" || z === size);
};
const animating = (testid: string, type: string): ((entry: string) => boolean) => (entry) => entry === `animating|${testid}|${type}`;

/** p1's board with `layers` in lanes 1 on, p2's empty. */
function board(...layers: (UnitView | null)[]): PlayerView {
  const units: (UnitView | null)[] = [null, null, null, null, null];
  layers.forEach((layer, lane) => {
    units[lane] = layer;
  });
  return baseView({ you: emptySide("p1", { units }), opponent: emptySide("p2") });
}

const activations = [
  { ability: "interceptor", label: "Activate 2", usesLeft: 2, usable: true },
  { ability: "corvette", label: "Activate", usesLeft: 1, usable: true },
];

const shield = (over: Partial<UnitView> = {}): UnitView => unit("p1", { instanceId: "fx-shield", defId: SHIELD, attack: 0, armor: 4, health: 12, maxHealth: 12, buried: 2, ...over });
const hangar = (over: Partial<UnitView> = {}): UnitView => unit("p1", { instanceId: "fx-hangar", defId: HANGAR, attack: 0, health: 12, maxHealth: 12, buried: 1, activations, ...over });
const meltdown = (over: Partial<UnitView> = {}): UnitView => unit("p1", { instanceId: "fx-keel", defId: MELTDOWN, attack: 6, health: 24, maxHealth: 24, buried: 0, ...over });

const destroyed = (instanceId: string, defId: string): GameEvent => ({ type: "destroyed", instanceId, defId, owner: "p1", controller: "p1", attack: 0, maxHealth: 12, killerId: null });
const summoned = (instanceId: string, defId: string, lane: number): GameEvent => ({ type: "summoned", player: "p1", instanceId, defId, row: "units", lane });
const marker = (): GameEvent => ({ type: "hookResolving", sourceId: "fx-hangar", defId: HANGAR, controller: "p1", hook: "activate" });

/** The scene is over: nothing is animating any more. */
function settles(): void {
  cy.get("[data-animating]", { timeout: 10_000 }).should("not.exist");
}

for (const size of SIZES) {
  describe(`R3336000 the Drone Battleship at ${String(size.width)}x${String(size.height)}`, () => {
    beforeEach(() => {
      cy.viewport(size.width, size.height);
    });

    it("R3336000 the art draws the decks still standing: three, then two, then one down the pile", () => {
      const view = board(shield({ instanceId: "fx-a", buried: 2 }), hangar({ instanceId: "fx-b", buried: 1 }), meltdown({ instanceId: "fx-c", buried: 0 }));
      mountScene(view, view, []);
      cy.get('[data-testid="card-fx-a"] [data-art-decks]').should("have.attr", "data-art-decks", "3");
      cy.get('[data-testid="card-fx-b"] [data-art-decks]').should("have.attr", "data-art-decks", "2");
      cy.get('[data-testid="card-fx-c"] [data-art-decks]').should("have.attr", "data-art-decks", "1");
    });

    it("R3336000 the arrival drops the keel, locks the hangar onto it and lands the superstructure, one summon each", () => {
      const initial = board();
      const shown = board(shield());
      mountScene(initial, shown, [
        { type: "cardPlayed", player: "p1", instanceId: "fx-core", defId: CORE_X, costPaid: 10 },
        summoned("fx-keel", MELTDOWN, 1),
        summoned("fx-hangar", HANGAR, 1),
        summoned("fx-shield", SHIELD, 1),
        { type: "cardResolved", player: "p1", instanceId: "fx-core", defId: CORE_X, permanent: false, costPaid: 10 },
      ]);
      // The third beat's dome flares over the zone the three share.
      sees("the superstructure's dome flares", cue("shield", "", "full"));
      cy.get(CANVAS).should("exist");
      settles();
    });

    it("R3336000 a hit redirected onto the Shield beams to the dome, and the plates glow for what Armor took", () => {
      const view = board(shield({ armor: 4 }));
      mountScene(view, view, [
        { type: "redirected", what: "damage", fromId: "hero-p1", toId: "fx-shield", byInstanceId: "fx-shield" },
        { type: "damageAbsorbed", sourceId: null, targetId: "fx-shield", absorbed: 3, combat: false },
      ]);
      sees("four plates, three lit", (entry) => entry.startsWith("ship|plates|") && entry.includes("plates=4") && entry.includes("lit=3"));
      sees("the dome flares", cue("shield", "", "full"));
      settles();
    });

    it("R3336000 an ordinary hit on the Shield is a dome flicker, with no plates", () => {
      const view = board(shield());
      mountScene(view, view, [{ type: "damage", sourceId: "fx-foe", targetId: "fx-shield", amount: 3, combat: true }]);
      sees("a sheen", cue("sheen"));
      sees("a small shield", cue("shield", "", "small"));
      settles();
      neverSees("no plates", cue("ship"));
    });

    it("R3336000 the Propagation layer's Activate 2 opens a bay and shoots an Interceptor out to its tile", () => {
      const view = board(hangar());
      mountScene(view, view, [
        marker(),
        { type: "activated", player: "p1", instanceId: "fx-hangar", defId: HANGAR, ability: "interceptor" },
        summoned("fx-drone", INTERCEPTOR, 4),
      ]);
      // Two doors, and a light for each bay with a use left after this one.
      sees("a bay with two doors and two lights", (entry) => entry.startsWith("ship|bay|") && entry.includes("doors=2") && entry.includes("lights=2"));
      sees("the drone's zone animates its summon", animating("zone-you-units-4", "summoned"));
      cy.get(CANVAS).should("exist");
      settles();
    });

    it("R3336000 the Propagation layer's Activate rolls a Corvette out heavy, a light left on for the Interceptor's bay", () => {
      const view = board(hangar());
      mountScene(view, view, [
        marker(),
        { type: "activated", player: "p1", instanceId: "fx-hangar", defId: HANGAR, ability: "corvette" },
        summoned("fx-drone", CORVETTE, 4),
      ]);
      // The Corvette's single use is spent, so one light stays on.
      sees("a bay with two doors and one light", (entry) => entry.startsWith("ship|bay|") && entry.includes("doors=2") && entry.includes("lights=1"));
      sees("the drone's zone animates its summon", animating("zone-you-units-4", "summoned"));
      cy.get(CANVAS).should("exist");
      settles();
    });

    for (const [label, health, amount] of [
      ["a first tick", 24, 3],
      ["a tick near the end", 6, 6],
    ] as const) {
      it(`R3336000 the Meltdown's end of turn pulses its core: ${label}`, () => {
        const view = board(meltdown({ health }));
        mountScene(view, view, [{ type: "damage", sourceId: "fx-keel", targetId: "fx-keel", amount, combat: false }]);
        // The pulse is a fire ring, an ember burst and a shake, all on the canvas: the card takes the hit.
        sees("the keel takes its own damage", animating("card-fx-keel", "damage"));
        cy.get(CANVAS).should("exist");
        settles();
        neverSees("no shield, plates or bay", (entry) => cue("ship")(entry) || cue("shield")(entry));
      });
    }

    it("R3336000 the Shield breaking shears its deck off and lights the Propagation layer beneath", () => {
      const initial = board(shield());
      const shown = board(hangar({ buried: 1 }));
      mountScene(initial, shown, [destroyed("fx-shield", SHIELD)]);
      sees("the Shield is destroyed", animating("card-fx-shield", "destroyed"));
      // The next deck's lights come up over the zone.
      sees("a sheen over the zone", cue("sheen"));
      settles();
    });

    it("R3336000 the Propagation layer breaking shears its deck off and lights the Meltdown beneath", () => {
      const initial = board(hangar({ buried: 1 }));
      const shown = board(meltdown());
      mountScene(initial, shown, [destroyed("fx-hangar", HANGAR)]);
      sees("the Propagation layer is destroyed", animating("card-fx-hangar", "destroyed"));
      sees("a sheen over the zone", cue("sheen"));
      settles();
    });

    it("R3336000 the Meltdown's death is a reactor blast", () => {
      const initial = board(meltdown());
      const shown = board();
      mountScene(initial, shown, [destroyed("fx-keel", MELTDOWN)]);
      sees("rays over the reactor", cue("rays"));
      settles();
      neverSees("no deck shears off the keel", cue("sheen"));
    });
  });
}
