// Issue #124: the visible behaviour. A Stack pile opens as a Marvel Snap-style wheel (the top
// card forward, the buried cards as backs beneath, shuffled by tap, buttons and keyboard); a
// sweep rolls one fog over the swept side; a whole-pile impact washes one wave over the pile.
//
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/stack-wheel-and-sweeps.cy.tsx

import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView, withEvents } from "../../../apps/web/src/test/fixtures.ts";

type PlayerView = ReturnType<typeof fullBoardView>;
type GameEvent = Parameters<typeof withEvents>[1][number];

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

const BOARD = '[data-testid="board"]';
const FX_DOM = '[data-testid="fx-dom"]';
const SHEET = '[data-testid="stack-sheet"]';
const POSITION = '[data-testid="wheel-position"]';
const NOTE = '[data-testid="wheel-note"]';
const NEXT = '[data-testid="wheel-next"]';
const CLOSE = '[data-testid="inspect-close"]';
const WHEEL = '[data-testid="wheel"]';

function enemyUnitIds(view: PlayerView): string[] {
  const ids = view.opponent.units.map((unit) => unit?.instanceId).filter((id) => id !== undefined);
  if (ids.length < 2) throw new Error("fullBoardView() needs swept enemy units");
  return ids;
}

function graveyardIds(view: PlayerView): string[] {
  const graves = view.you.graveyard;
  if (graves.length < 2) throw new Error("fullBoardView() needs a graveyard of at least two");
  return graves.map((card) => card.instanceId);
}

function expectFits(doc: Document, where: string): void {
  const win = doc.defaultView;
  expect(win, "the document has a window").to.not.eq(null);
  if (win === null) return;
  expect(doc.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(win.innerWidth);
}

describe("Issue #124: the stack wheel, the sweep fog and the whole-pile wave", () => {
  for (const viewport of VIEWPORTS) {
    const where = `${viewport.label} ${viewport.width}x${viewport.height}`;

    it(`the buried badge opens the pile as a wheel, shuffled by tap and keyboard, at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const view = fullBoardView();
      const noop = (): void => undefined;
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={view} legal={[]} onAction={noop} />
        </div>,
      );
      cy.get(BOARD).should("exist");

      // The fixture stacks a pile: the badge counts its buried cards and names none of them.
      cy.get('[data-buried]').should(($badges) => {
        expect($badges.length).to.be.greaterThan(0);
      });
      cy.get("[data-buried]").first().click();
      cy.get(SHEET).should("exist");
      cy.get("[data-buried]")
        .first()
        .then(($badge) => {
          const buried = Number($badge.attr("data-buried"));
          expect(buried, "the pile buries cards").to.be.greaterThan(0);
          // The top card stands forward over its backs: position and place read aloud.
          cy.get(POSITION).should("have.text", `1 of ${buried + 1}`);
          cy.get(NOTE).should("have.text", "Top of pile");
          // Tapping deeper and End reach the buried cards and the bottom one.
          cy.get(NEXT).click();
          cy.get(POSITION).should("have.text", `2 of ${buried + 1}`);
          cy.get(NOTE).should("have.text", "Buried");
          // The wheel is a div with tabIndex 0, not a typeable element, so End reaches it
          // as the keydown CardWheel listens for (as its unit test fires it) rather than type.
          cy.get(WHEEL).trigger("keydown", { key: "End", keyCode: 35, which: 35, bubbles: true });
          cy.get(POSITION).should("have.text", `${buried + 1} of ${buried + 1}`);
          cy.get(NOTE).should("have.text", "Bottom of pile");
        });
      cy.document().should((doc) => expectFits(doc, where));
      cy.get(CLOSE).click();
      cy.get(SHEET).should("not.exist");
    });

    it(`a sweep rolls one fog over the swept side at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const view = fullBoardView();
      const ids = enemyUnitIds(view);
      const events: GameEvent[] = ids.map((targetId) => ({ type: "damage", sourceId: null, targetId, amount: 2, combat: false }));
      const noop = (): void => undefined;
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={view} legal={[]} onAction={noop} />
        </div>,
      ).then(({ rerender }) => {
        cy.then(() =>
          rerender(
            <div className="app-shell app-shell--wide">
              <Game view={withEvents(view, events)} legal={[]} onAction={noop} />
            </div>,
          ),
        );
      });

      // One fog for the whole side, not one effect per hit; then it leaves nothing behind.
      cy.get(`${FX_DOM} [data-fx="fog"]`).should("exist");
      cy.get(`${FX_DOM} [data-fx="fog"]`).should(($fogs) => {
        expect($fogs.length, "a single fog rolls over the swept side").to.eq(1);
      });
      cy.get("[data-animating]").should("not.exist");
      cy.get(FX_DOM).should(($root) => {
        expect(($root[0] as HTMLElement).childElementCount, "fx-dom is empty").to.eq(0);
      });
      cy.document().should((doc) => expectFits(doc, where));
    });

    it(`a whole-graveyard impact washes one wave over the pile at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      const view = fullBoardView();
      const ids = graveyardIds(view);
      const events: GameEvent[] = ids.map((instanceId) => ({
        type: "degraded",
        instanceId,
        defId: "core-003",
        change: { kind: "cost", delta: -1 },
      }));
      const noop = (): void => undefined;
      cy.mount(
        <div className="app-shell app-shell--wide">
          <Game view={view} legal={[]} onAction={noop} />
        </div>,
      ).then(({ rerender }) => {
        cy.then(() =>
          rerender(
            <div className="app-shell app-shell--wide">
              <Game view={withEvents(view, events)} legal={[]} onAction={noop} />
            </div>,
          ),
        );
      });

      cy.get(`${FX_DOM} [data-fx="zone"]`).should("exist");
      cy.get(`${FX_DOM} [data-fx="zone"]`).should(($waves) => {
        expect($waves.length, "a single wave washes over the pile").to.eq(1);
      });
      cy.get("[data-animating]").should("not.exist");
      cy.get(FX_DOM).should(($root) => {
        expect(($root[0] as HTMLElement).childElementCount, "fx-dom is empty").to.eq(0);
      });
    });
  }
});
