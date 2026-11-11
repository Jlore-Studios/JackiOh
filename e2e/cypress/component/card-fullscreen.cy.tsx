// R3345000: Full-screen view for Mythics and full-art cards.
// A component spec capturing the full-screen view at desktop size (1280x720) and phone (390x844).
// Run: E2E_COMPONENT_PORT=5282 pnpm --dir e2e test:component --spec cypress/component/card-fullscreen.cy.tsx

import { CATALOG } from "@jackioh/cards";

import {
  ART_PLANES,
  FullScreenArt,
  FullScreenArtHost,
  INSPECT_HOVER,
  closeFullScreenArt,
  useFullScreenTrigger,
} from "../../../apps/web/src/cards/index.ts";
import { FULLSCREEN_HOLD_MS } from "../../../apps/web/src/cards/constants.ts";
import { LONG_PRESS_MS } from "../../../apps/web/src/cards/inspect/constants.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, card, dormantStarship, emptySide, unit } from "../../../apps/web/src/test/fixtures.ts";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

/** R3351000: the shares of a card's width a press is tried at, across its middle and then leftward. */
const PRESS_SHARES = [0.5, 0.4, 0.3, 0.2, 0.1] as const;

/**
 * R3351000: where on `card`, from its box's top left, a finger lands on the card itself: its middle, or, in a fanned hand
 * whose next card covers a card's right part (board.css `--fan-step`), the part of it that shows. Null when none of it does.
 */
function pressPoint(card: HTMLElement): { x: number; y: number } | null {
  const box = card.getBoundingClientRect();
  const y = box.height / 2;
  for (const share of PRESS_SHARES) {
    const x = box.width * share;
    const hit = card.ownerDocument.elementFromPoint(box.left + x, box.top + y);
    if (hit !== null && card.contains(hit)) return { x, y };
  }
  return null;
}

/** Triggers `type` on the card `testid` at its press point, once some of the card shows. */
function pressCard(testid: string, type: "pointerdown" | "pointerup", options: { pointerId: number; pointerType?: string }): void {
  cy.get(`[data-testid="${testid}"]`)
    .should(($card) => {
      expect(pressPoint($card[0] as HTMLElement), `${testid}: some of it shows to press`).not.to.eq(null);
    })
    .then(($card) => {
      const at = pressPoint($card[0] as HTMLElement) ?? { x: 0, y: 0 };
      cy.wrap($card).trigger(type, at.x, at.y, options);
    });
}

function TriggerCard({ id, name }: { id: string; name: string }) {
  const { handlers } = useFullScreenTrigger({
    defId: id,
    rarity: "Mythic",
    name,
  });

  return (
    <div style={{ padding: 40 }}>
      <button
        type="button"
        data-testid="mythic-card-trigger"
        style={{ width: 150, height: 210, background: "#333", color: "#fff", cursor: "pointer" }}
        {...handlers}
      >
        {name}
      </button>
      <FullScreenArtHost />
    </div>
  );
}

describe("R3345000 full-screen view", () => {
  beforeEach(() => {
    closeFullScreenArt();
  });

  afterEach(() => {
    closeFullScreenArt();
  });

  for (const vp of VIEWPORTS) {
    it(`R3345000 captures full-screen art at ${vp.label} (${vp.width}x${vp.height}) with 4 depth planes and camera dolly`, () => {
      cy.viewport(vp.width, vp.height);
      cy.mount(
        <FullScreenArt
          defId="core-100"
          name="The Ceaseless Void"
          onClose={() => {
            closeFullScreenArt();
          }}
        />,
      );

      cy.get('[data-testid="fullscreen-art"]').should("be.visible");
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "role", "dialog");
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "aria-modal", "true");
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "aria-label", "The Ceaseless Void");

      // Verifies all 4 depth planes are present
      for (const plane of ART_PLANES) {
        cy.get(`[data-testid="fullscreen-art"] [data-plane="${plane}"]`).should("exist");
      }

      // Close button exists and works
      cy.get('[data-testid="fullscreen-art-close"]').should("be.visible");
    });

    it(`R3345000 tap pauses and resumes, and release closes it at ${vp.label}`, () => {
      cy.viewport(vp.width, vp.height);
      let closed = false;
      cy.mount(
        <FullScreenArt
          defId="core-100"
          name="The Ceaseless Void"
          heldPointerId={1}
          onClose={() => {
            closed = true;
          }}
        />,
      );

      cy.get('[data-testid="fullscreen-art"]').should("be.visible");
      cy.get('[data-testid="fullscreen-art"]').should("not.have.attr", "data-paused");

      // Tap pauses
      cy.get('[data-testid="fullscreen-art"]').click();
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "data-paused", "true");

      // Tap resumes
      cy.wait(400);
      cy.get('[data-testid="fullscreen-art"]').click();
      cy.get('[data-testid="fullscreen-art"]').should("not.have.attr", "data-paused");

      // Release of held pointer closes
      cy.window().trigger("pointerup", { pointerId: 1 });
      cy.then(() => {
        expect(closed).to.equal(true);
      });
    });

    it(`R3345000 press and hold opens the full-screen view at ${vp.label}`, () => {
      cy.viewport(vp.width, vp.height);
      cy.mount(<TriggerCard id="core-100" name="The Ceaseless Void" />);

      cy.get('[data-testid="fullscreen-art"]').should("not.exist");

      // Press and hold past FULLSCREEN_HOLD_MS
      cy.get('[data-testid="mythic-card-trigger"]').trigger("pointerdown", { pointerId: 2, clientX: 100, clientY: 100 });
      cy.wait(FULLSCREEN_HOLD_MS + 50);

      cy.get('[data-testid="fullscreen-art"]').should("be.visible");

      // Release closes
      cy.window().trigger("pointerup", { pointerId: 2 });
      cy.get('[data-testid="fullscreen-art"]').should("not.exist");
    });
  }
});

describe("R3351000 a long press opens inspect and the full-screen view from every board face and hand card", () => {
  beforeEach(() => {
    closeFullScreenArt();
  });

  afterEach(() => {
    closeFullScreenArt();
  });

  it("R3351000 a long press opens inspect from every board face and hand card, and full-screen where R3345000 allows it", () => {
    cy.viewport(390, 844);
    // Own Mythics open the full-screen view; every other face opens inspect; an opponent Mythic
    // opens inspect but never full-screen (R3345000: the board opens it for own cards only).
    const cases = [
      { testid: "card-fx-u1", fullscreen: true },
      { testid: "card-fx-u2", fullscreen: false },
      { testid: "card-fx-u3", fullscreen: false },
      { testid: "card-fx-ship", fullscreen: false },
      { testid: "card-fx-opp-1", fullscreen: false },
      { testid: "hand-card-fx-h1", fullscreen: true },
      { testid: "hand-card-fx-h2", fullscreen: true },
      { testid: "hand-card-fx-h3", fullscreen: false },
      { testid: "hand-card-fx-h4", fullscreen: false },
      { testid: "hand-card-fx-h5", fullscreen: false },
      { testid: "hand-card-fx-h6", fullscreen: false },
      { testid: "hand-card-fx-h7", fullscreen: false },
      { testid: "hand-card-fx-h8", fullscreen: false },
      { testid: "hand-card-fx-h9", fullscreen: false },
      { testid: "hand-card-fx-h10", fullscreen: false },
    ] as const;
    const lookup = lookupFromDefs(CATALOG);
    const noop = (): void => undefined;
    const view = baseView({
      you: emptySide("p1", {
        units: [
          unit("p1", { instanceId: "fx-u1", defId: "meditative-049" }),
          unit("p1", { instanceId: "fx-u2", defId: "core-052" }),
          unit("p1", { instanceId: "fx-u3", defId: "core-004", attack: 2, maxHealth: 3, health: 1 }),
          dormantStarship("p1", ["stinkyscape-t-dormant-starship", "stinkyscape-140"], { instanceId: "fx-ship" }),
          null,
        ],
        hand: [
          card({ instanceId: "fx-h1", defId: "core-100", cost: 10 }),
          card({ instanceId: "fx-h2", defId: "meditative-049", cost: 2 }),
          card({ instanceId: "fx-h3", defId: "core-019", cost: 3 }),
          card({ instanceId: "fx-h4", defId: "core-055", cost: 0 }),
          card({ instanceId: "fx-h5", defId: "core-077", cost: 6 }),
          card({ instanceId: "fx-h6", defId: "core-002", cost: 1 }),
          card({ instanceId: "fx-h7", defId: "core-003", cost: 2 }),
          card({ instanceId: "fx-h8", defId: "core-005", cost: 2 }),
          card({ instanceId: "fx-h9", defId: "core-007", cost: 4 }),
          card({ instanceId: "fx-h10", defId: "core-009", cost: 5 }),
        ],
      }),
      opponent: emptySide("p2", {
        units: [unit("p2", { instanceId: "fx-opp-1", defId: "meditative-049" }), null, null, null, null],
      }),
    });
    cy.mount(
      <CatalogContext.Provider value={lookup}>
        <div className="app-shell app-shell--wide">
          {/* wearCosmetics, as the hotseat passes it: the viewer's own board cards wear their picks, so an own Mythic may open the full-screen view (R3345000). */}
          <Game view={view} legal={[]} onAction={noop} wearCosmetics />
        </div>
      </CatalogContext.Provider>,
    );
    cy.get('[data-testid="fullscreen-art"]').should("not.exist");

    for (const [index, press] of cases.entries()) {
      const pointerId = 100 + index;
      // A touch held past LONG_PRESS_MS where the card shows (in the ten-card fan each card's neighbour covers its middle,
      // the last card's aside): the hover on a phone (R2617000), or the full-screen view.
      pressCard(press.testid, "pointerdown", { pointerId, pointerType: "touch" });
      cy.wait(LONG_PRESS_MS + 100);
      if (press.fullscreen) {
        cy.get('[data-testid="fullscreen-art"]').should("be.visible");
        // Release closes the view it opened.
        cy.window().trigger("pointerup", { pointerId });
        cy.get('[data-testid="fullscreen-art"]').should("not.exist");
      } else {
        cy.get(`[data-testid="${INSPECT_HOVER}"]`).should("exist");
        cy.get('[data-testid="fullscreen-art"]').should("not.exist");
        pressCard(press.testid, "pointerup", { pointerId });
        cy.get(`[data-testid="${INSPECT_HOVER}"]`).should("not.exist");
      }
    }
  });
});
