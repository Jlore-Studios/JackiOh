// R4773000: on a phone the game board never scrolls the page while no picker or menu is open.
// Dragging the targeting arrow from an attacker — or an aim started from your hero — toward a
// target must not scroll or bounce the page under the finger, and a swipe anywhere on the board
// must leave the page unscrolled with the document's `overscroll-behavior` at `none`. Each menu
// and the log keep scrolling inside themselves while the lock is on.
//
// A finger's pointer events (`pointerType: "touch"`, past the 8 px drag threshold) follow
// `deckbuilder-touch-drag.cy.tsx`. Synthetic events cannot move a real page on their own, so the
// scroll readings pin the behavior while the computed-style assertions (`touch-action: none` on
// the board, `overscroll-behavior: none` on the document, `pan-y`/`contain` on each scroller)
// are what fail without the fix.

import type { ReactElement } from "react";

import type { ActionBody, GameEvent } from "@jackioh/shared";

import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, card, emptySide, heroPower, pendingFor, unit } from "../../../apps/web/src/test/fixtures.ts";

type GameProps = Parameters<typeof Game>[0];
type View = GameProps["view"];
type Legal = GameProps["legal"];

const BOARD = '[data-testid="board"]';
const FIELD = `${BOARD} .field`;

/** R4773000: the attribute a mounted Game holds on the document element (Game.tsx). */
const GAME_SCROLL_LOCK = "data-game-scroll-lock";

/** Past DragLayer's 8 px threshold, so no rounding keeps the lift below it. */
const NUDGE_PX = 24;

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

type Point = { x: number; y: number };

/** Mounts the game and collects every action it sends. */
function mountGame(view: View, legal: Legal = [], controlExtras?: GameProps["controlExtras"]): { acted: ActionBody[] } {
  const acted: ActionBody[] = [];
  cy.mount(
    <div className="app-shell app-shell--wide">
      <Game
        view={view}
        legal={legal}
        onAction={(body) => {
          acted.push(body);
        }}
        controlExtras={controlExtras}
      />
    </div>,
  );
  return { acted };
}

function touch(phase: "down" | "move" | "up", at: Point) {
  return {
    eventConstructor: "PointerEvent",
    pointerType: "touch",
    pointerId: 7,
    isPrimary: true,
    button: 0,
    buttons: phase === "up" ? 0 : 1,
    clientX: at.x,
    clientY: at.y,
    bubbles: true,
    cancelable: true,
  };
}

function centerOf(selector: string): Cypress.Chainable<Point> {
  return cy.get(selector).then(($el) => {
    const box = ($el[0] as HTMLElement).getBoundingClientRect();
    return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
  });
}

/** `window.scrollX/scrollY` sampled after every step of a gesture. */
function snapScroll(trail: Point[]): void {
  cy.window({ log: false }).then((win) => {
    trail.push({ x: win.scrollX, y: win.scrollY });
  });
}

function expectPageUnscrolled(trail: readonly Point[], where: string): void {
  cy.window({ log: false }).should((win) => {
    expect(win.scrollX, `${where}: scrollX stays 0`).to.eq(0);
    expect(win.scrollY, `${where}: scrollY stays 0`).to.eq(0);
  });
  cy.wrap(null, { log: false }).should(() => {
    expect(
      trail.every((point) => point.x === 0 && point.y === 0),
      `${where}: no scroll at any sampled point (${trail.length} samples)`,
    ).to.eq(true);
  });
}

function expectDocumentLocked(where: string): void {
  cy.document({ log: false }).should((doc) => {
    const root = doc.documentElement;
    expect(root.hasAttribute(GAME_SCROLL_LOCK), `${where}: the game holds the scroll lock`).to.eq(true);
    const win = doc.defaultView;
    expect(win, `${where}: a window to read the style from`).to.not.eq(null);
    if (win === null) return;
    expect(win.getComputedStyle(root).overscrollBehavior, `${where}: the document never overscrolls`).to.eq("none");
  });
  cy.get(BOARD, { log: false }).should(($board) => {
    const win = $board[0]?.ownerDocument.defaultView;
    expect(win, `${where}: a window to read the style from`).to.not.eq(null);
    if ($board[0] === undefined || win === null || win === undefined) return;
    expect(win.getComputedStyle($board[0]).touchAction, `${where}: the board takes no pan`).to.eq("none");
  });
}

/** Your attacker against their minion, aimed across the field. */
function attackView(): View {
  return baseView({
    you: emptySide("p1", {
      units: [unit("p1", { instanceId: "u1" }), null, null, null, null],
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), null, null, null, null],
    }),
  });
}

const ATTACK_U1_E1: ActionBody = { type: "attack", attackerId: "u1", targetId: "e1" };

const ATTACK_LEGAL: Legal = [
  ATTACK_U1_E1,
  { type: "attack", attackerId: "u1", targetId: "hero-p2" },
  { type: "endTurn" },
  { type: "offerDraw" },
  { type: "concede" },
];

/** Your hero's power, aimed at their minion: the aim that starts on your hero's plate. */
function heroAimView(): View {
  return baseView({
    you: emptySide("p1", {
      hero: { health: 30, armor: 0, powers: [heroPower], power: heroPower },
    }),
    opponent: emptySide("p2", {
      hand: { count: 3 },
      units: [unit("p2", { instanceId: "e1" }), null, null, null, null],
    }),
  });
}

const POWER_E1: ActionBody = {
  type: "activatePower",
  instanceId: "power-1",
  targets: [{ pick: "instance", instanceId: "e1" }],
};

const POWER_LEGAL: Legal = [
  POWER_E1,
  { type: "activatePower", instanceId: "power-1", targets: [{ pick: "hero", player: "p2" }] },
  { type: "endTurn" },
  { type: "offerDraw" },
  { type: "concede" },
];

/**
 * Press `source`, lift with a touch nudge, travel across the field to `target` and release there,
 * sampling the page scroll after every step. Asserts the drag holds `:root[data-dragging]` with
 * no page movement possible once lifted.
 */
function aimAcross(source: string, target: string, kind: "attack" | "activate", trail: Point[], where: string): void {
  let from: Point = { x: 0, y: 0 };
  let to: Point = { x: 0, y: 0 };
  centerOf(source).then((point) => {
    from = point;
  });
  centerOf(target).then((point) => {
    to = point;
  });
  cy.get(source).then(() => {
    cy.get(source).trigger("pointerdown", touch("down", from));
    snapScroll(trail);
    const lifted = { x: from.x, y: from.y - NUDGE_PX };
    cy.get("body").trigger("pointermove", touch("move", lifted));
    snapScroll(trail);
    cy.document().should((doc) => {
      expect(doc.documentElement.getAttribute("data-dragging"), `${where}: the lift holds the drag`).to.eq(kind);
      const win = doc.defaultView;
      expect(win, `${where}: a window to read the style from`).to.not.eq(null);
      if (win === null) return;
      expect(win.getComputedStyle(doc.documentElement).touchAction, `${where}: no page movement during the drag`).to.eq(
        "none",
      );
    });
    const mid = { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 };
    cy.get("body").trigger("pointermove", touch("move", mid));
    snapScroll(trail);
    cy.get("body").trigger("pointermove", touch("move", to));
    snapScroll(trail);
    cy.get(target).trigger("pointerup", touch("up", to));
    snapScroll(trail);
  });
}

for (const viewport of [
  { width: 390, height: 844 },
  { width: 844, height: 390 },
] as const) {
  const where = `${viewport.width}x${viewport.height}`;

  describe(`R4773000 at ${where} a touch drag-to-aim never moves the page`, () => {
    beforeEach(() => {
      cy.viewport(viewport.width, viewport.height);
    });

    it(`R4773000 at ${where} a touch drag-to-aim from an attacker lands on the enemy minion with scrollX/scrollY at 0 throughout`, () => {
      const { acted } = mountGame(attackView(), ATTACK_LEGAL);
      cy.get(BOARD).should("be.visible");
      expectDocumentLocked(`aim at ${where}`);

      const trail: Point[] = [];
      aimAcross(ts("card-u1"), ts("card-e1"), "attack", trail, `aim at ${where}`);

      cy.wrap(null, { log: false }).should(() => {
        expect(acted, "the aim landed on the enemy minion").to.deep.include(ATTACK_U1_E1);
      });
      cy.document().should((doc) => {
        expect(doc.documentElement.hasAttribute("data-dragging"), "the drag ended").to.eq(false);
      });
      expectPageUnscrolled(trail, `aim at ${where}`);
    });

    it(`R4773000 at ${where} an aim started from your hero lands on the enemy minion with scrollX/scrollY at 0 throughout`, () => {
      const { acted } = mountGame(heroAimView(), POWER_LEGAL);
      cy.get(BOARD).should("be.visible");
      expectDocumentLocked(`hero aim at ${where}`);

      const trail: Point[] = [];
      aimAcross(ts("power"), ts("card-e1"), "activate", trail, `hero aim at ${where}`);

      cy.wrap(null, { log: false }).should(() => {
        expect(acted, "the aim landed on the enemy minion").to.deep.include(POWER_E1);
      });
      cy.document().should((doc) => {
        expect(doc.documentElement.hasAttribute("data-dragging"), "the drag ended").to.eq(false);
      });
      expectPageUnscrolled(trail, `hero aim at ${where}`);
    });

    it(`R4773000 at ${where} a touch swipe across empty field space leaves the page unscrolled`, () => {
      mountGame(attackView(), ATTACK_LEGAL);
      cy.get(FIELD).should("be.visible");
      expectDocumentLocked(`swipe at ${where}`);

      const trail: Point[] = [];
      centerOf(FIELD).then((from) => {
        cy.get(FIELD).trigger("pointerdown", touch("down", from));
        snapScroll(trail);
        for (const dy of [40, 120, 200]) {
          cy.get("body").trigger("pointermove", touch("move", { x: from.x, y: from.y + dy }));
          snapScroll(trail);
        }
        cy.get("body").trigger("pointerup", touch("up", { x: from.x, y: from.y + 200 }));
        snapScroll(trail);
      });

      cy.document().should((doc) => {
        expect(doc.documentElement.hasAttribute("data-dragging"), "no drag started").to.eq(false);
      });
      expectPageUnscrolled(trail, `swipe at ${where}`);
    });
  });
}

const DISCOVER_DEFS = ["core-043", "core-055", "core-066", "core-008", "core-010", "core-019", "core-002", "core-004", "core-001"];

function overflowDiscoverView(): View {
  return baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "ph1", defId: "core-002" })] }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
    pending: pendingFor(
      "discover",
      Array.from({ length: 15 }, (_, at) => ({
        key: `overflow-${String(at)}`,
        label: `Pick ${String(at)}`,
        defId: DISCOVER_DEFS[at % DISCOVER_DEFS.length] as string,
      })),
    ),
  });
}

describe("R4773000 menus and the log scroll inside themselves while the page is locked", () => {
  beforeEach(() => {
    cy.viewport(390, 844);
  });

  it("R4773000 an overflowing Discover still scrolls to its last option while the page stays put", () => {
    mountGame(overflowDiscoverView());
    cy.get(ts("prompt-modal")).should("be.visible");
    expectDocumentLocked("discover");

    cy.get(ts("prompt-modal")).should(($modal) => {
      const panel = $modal[0] as HTMLElement;
      expect(panel.scrollHeight, "the Discover overflows its panel").to.be.greaterThan(panel.clientHeight);
    });
    cy.get(ts("prompt-modal")).then(($modal) => {
      const panel = $modal[0] as HTMLElement;
      panel.scrollTop = panel.scrollHeight;
    });
    cy.get(ts("prompt-modal")).should(($modal) => {
      const panel = $modal[0] as HTMLElement;
      expect(panel.scrollTop, "the panel scrolled").to.be.greaterThan(0);
      const last = panel.querySelector(ts("prompt-option-overflow-14"));
      expect(last, "the last option is rendered").to.not.eq(null);
      if (last === null) return;
      const panelBox = panel.getBoundingClientRect();
      const lastBox = last.getBoundingClientRect();
      expect(lastBox.bottom, "the last option scrolled into view").to.be.at.most(panelBox.bottom + 1);
    });
    cy.get(ts("prompt-modal")).should(($modal) => {
      const win = $modal[0]?.ownerDocument.defaultView;
      expect(win, "a window to read the style from").to.not.eq(null);
      if ($modal[0] === undefined || win === null || win === undefined) return;
      expect(win.getComputedStyle($modal[0]).touchAction, "the panel takes a vertical pan").to.eq("pan-y");
      expect(win.getComputedStyle($modal[0]).overscrollBehavior, "the panel contains its overscroll").to.eq("contain");
    });
    cy.window().should((win) => {
      expect(win.scrollX, "the page stays put").to.eq(0);
      expect(win.scrollY, "the page stays put").to.eq(0);
    });
  });

  it("R4773000 the match menu still scrolls while the page stays put", () => {
    const extras = (count: number): ReactElement => (
      <>
        {Array.from({ length: count }, (_, at) => (
          <button key={at} type="button" className="control" data-testid={`x-extra-${String(at)}`}>
            {`Extra ${String(at)}`}
          </button>
        ))}
      </>
    );
    mountGame(attackView(), ATTACK_LEGAL, extras(12));
    cy.get(ts("match-menu")).should("be.visible").click();
    cy.get(ts("match-menu-panel")).should("be.visible");
    expectDocumentLocked("match menu");

    cy.get(ts("match-menu-panel")).should(($menu) => {
      const panel = $menu[0] as HTMLElement;
      expect(panel.scrollHeight, "the match menu overflows").to.be.greaterThan(panel.clientHeight);
    });
    cy.get(ts("match-menu-panel")).then(($menu) => {
      const panel = $menu[0] as HTMLElement;
      panel.scrollTop = panel.scrollHeight;
    });
    cy.get(ts("match-menu-panel")).should(($menu) => {
      const panel = $menu[0] as HTMLElement;
      expect(panel.scrollTop, "the menu scrolled").to.be.greaterThan(0);
      const last = panel.querySelector(ts("x-extra-11"));
      expect(last, "the last row is rendered").to.not.eq(null);
      if (last === null) return;
      const panelBox = panel.getBoundingClientRect();
      const lastBox = last.getBoundingClientRect();
      expect(lastBox.bottom, "the last row scrolled into view").to.be.at.most(panelBox.bottom + 1);
    });
    cy.window().should((win) => {
      expect(win.scrollX, "the page stays put").to.eq(0);
      expect(win.scrollY, "the page stays put").to.eq(0);
    });
  });

  it("R4773000 the log still scrolls inside itself while the page stays put", () => {
    const turns: GameEvent[] = Array.from(
      { length: 40 },
      (_, at) => ({ type: "turnStarted", player: at % 2 === 0 ? "p1" : "p2", turn: at + 1 }) as GameEvent,
    );
    mountGame({ ...attackView(), events: turns }, ATTACK_LEGAL);
    cy.get(ts("log-toggle")).should("be.visible").click();
    cy.get(ts("log")).should("be.visible");
    expectDocumentLocked("log");

    cy.get(ts("log"), { timeout: 30_000 }).should(($log) => {
      const list = $log[0] as HTMLElement;
      expect(list.scrollHeight, "the log overflows its box").to.be.greaterThan(list.clientHeight);
      const win = list.ownerDocument.defaultView;
      expect(win, "a window to read the style from").to.not.eq(null);
      if (win === null || win === undefined) return;
      expect(win.getComputedStyle(list).touchAction, "the log takes a vertical pan").to.eq("pan-y");
      expect(win.getComputedStyle(list).overscrollBehavior, "the log contains its overscroll").to.eq("contain");
    });
    cy.get(ts("log")).then(($log) => {
      const list = $log[0] as HTMLElement;
      list.scrollTop = list.scrollHeight;
    });
    cy.get(ts("log")).should(($log) => {
      expect(($log[0] as HTMLElement).scrollTop, "the log scrolled").to.be.greaterThan(0);
    });
    cy.window().should((win) => {
      expect(win.scrollX, "the page stays put").to.eq(0);
      expect(win.scrollY, "the page stays put").to.eq(0);
    });
  });
});
