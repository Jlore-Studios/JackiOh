// R3333000: a Starship's picture is built from its pieces, a Dormant one in its shipyard and a launched one in its name's
// hull design. R3333002: a Dormant hull's plates and the pieces list show each piece's module icon. jsdom has no layout,
// so apps/web/src/cards/starship.test.tsx checks the DOM; here Chrome lays the board out, at both of BUILD M5-T1's
// viewports, and every plate and icon must have a size and sit on its card.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/starship-art.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { STARSHIP_DESIGN_NAMES } from "../../../apps/web/src/cards/art/starshipHulls.ts";
import { starshipModuleOf } from "../../../apps/web/src/cards/art/starshipModules.ts";
import { CardDefsProvider, CardFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { STARSHIP_HULL_PLATES_MAX } from "../../../apps/web/src/cards/constants.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, dormantStarship, emptySide, unit } from "../../../apps/web/src/test/fixtures.ts";

type CardDef = (typeof CATALOG)[string];

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

const GAME = '[data-testid="game"]';
const HULL = "stinkyscape-t-dormant-starship";
/** Eight pieces from as many families: a beam, a drill, a shield, a capsule, rounds, an engine, a launcher, the cat. */
const PIECES = [
  "stinkyscape-058",
  "stinkyscape-066",
  "stinkyscape-038",
  "stinkyscape-043",
  "stinkyscape-100",
  "stinkyscape-090",
  "stinkyscape-140",
  "stinkyscape-134",
] as const;
/** The viewer's Dormant Starships, by their count of pieces, in lanes 1, 3 and 5. */
const DOCKED = [1, 4, 8] as const;
/** The other seat's launched Starships, by their count of pieces and the name each took, in lanes 1, 2, 4 and 5. */
const FLOWN = [
  [1, "The Ninth Life"],
  [4, "The Lantern Ferry"],
  [6, "The Jlockeed Skylark"],
  [8, "The Silent Choir"],
] as const;

/** Both seats' Starships on one board. */
function starshipBoard(): ReturnType<typeof baseView> {
  const mine = [null, null, null, null, null] as (ReturnType<typeof unit> | null)[];
  DOCKED.forEach((n, k) => {
    mine[k * 2] = dormantStarship("p1", PIECES.slice(0, n), { instanceId: `dock-${String(n)}` });
  });
  const theirs = [null, null, null, null, null] as (ReturnType<typeof unit> | null)[];
  FLOWN.forEach(([n, name], k) => {
    theirs[[0, 1, 3, 4][k] ?? 0] = unit("p2", {
      instanceId: `fleet-${String(n)}`,
      defId: HULL,
      attack: 0,
      starship: { pieces: PIECES.slice(0, n).map((defId) => ({ defId })), name, launched: k + 1 },
    });
  });
  return baseView({ you: emptySide("p1", { units: mine }), opponent: emptySide("p2", { units: theirs }) });
}

/** Whether `inner` has a size and lies within `outer`, give or take a pixel. */
function within(inner: DOMRect, outer: DOMRect): boolean {
  return inner.width > 0 && inner.height > 0 && inner.left >= outer.left - 1 && inner.right <= outer.right + 1 && inner.top >= outer.top - 1 && inner.bottom <= outer.bottom + 1;
}

describe("R3333000 R3333002 Starships on a real board, Dormant in the shipyard and launched in their names' designs", () => {
  for (const viewport of VIEWPORTS) {
    it(`R3333000 R3333002 Dormant ships of 1, 4 and 8 pieces and launched ships of 1, 4, 6 and 8 at ${viewport.label} size`, () => {
      cy.viewport(viewport.width, viewport.height);
      const lookup = lookupFromDefs(CATALOG);
      const noop = (): void => undefined;
      cy.mount(
        <CatalogContext.Provider value={lookup}>
          <div className="app-shell app-shell--wide">
            <Game view={starshipBoard()} legal={[]} onAction={noop} />
          </div>
        </CatalogContext.Provider>,
      );
      cy.get(GAME).should("exist");
      for (const n of DOCKED) {
        const card = `[data-testid="card-dock-${String(n)}"]`;
        cy.get(`${card} .cf-art`).should("have.attr", "data-art-design", "shipyard").and("have.attr", "style").and("contain", "data:image/svg+xml");
        cy.get(`${card} .cf-starship-plate`).should("have.length", Math.min(n, STARSHIP_HULL_PLATES_MAX));
        cy.get(card).then(($card) => {
          const shown = $card[0] as HTMLElement;
          const box = shown.getBoundingClientRect();
          const plates = [...shown.querySelectorAll<HTMLElement>(".cf-starship-plate")].sort(
            (a, b) => Number(a.dataset.plate) - Number(b.dataset.plate),
          );
          expect(plates.map((plate) => plate.dataset.module), `dock ${String(n)}`).to.deep.equal(PIECES.slice(0, n).map((id) => starshipModuleOf(id).kind));
          for (const plate of plates) {
            expect(within(plate.getBoundingClientRect(), box), `dock ${String(n)} plate ${String(plate.dataset.plate)} on its card`).to.equal(true);
            expect(getComputedStyle(plate).backgroundImage, `dock ${String(n)} plate ${String(plate.dataset.plate)}`).to.contain("data:image/svg+xml");
          }
        });
      }
      for (const [n, name] of FLOWN) {
        const card = `[data-testid="card-fleet-${String(n)}"]`;
        cy.get(`${card} .cf-art`).should("have.attr", "data-art-design", STARSHIP_DESIGN_NAMES[name]).and("have.attr", "style").and("contain", "data:image/svg+xml");
        cy.get(`${card} .cf-starship-plate`).should("not.exist");
      }
      // Seven pictures, every one its own.
      cy.document().then((doc) => {
        const pictures = [...doc.querySelectorAll<HTMLElement>('[data-art-family="starship"]')].map((art) => art.style.backgroundImage);
        expect(pictures).to.have.length(DOCKED.length + FLOWN.length);
        expect(new Set(pictures).size).to.equal(pictures.length);
      });
      cy.screenshot(`starship-art-board-${viewport.label}`, { capture: "viewport", overwrite: true });
    });
  }

  it("R3333002 a launched Starship's full face lists its eight pieces, each its module's icon before its name", () => {
    cy.viewport(1280, 720);
    const list = PIECES.map((defId, k) => ({ defId, radiant: k === 0 }));
    const face = faceModel({ defId: HULL, def: CATALOG[HULL] as CardDef, radiant: false, inPlay: { starship: { pieces: list.length, name: "Ember of Departure", list } } });
    cy.mount(
      <CardDefsProvider defs={CATALOG}>
        <div data-face style={{ width: 270, height: 378, padding: 8 }}>
          <CardFace face={face} layout="full" />
        </div>
      </CardDefsProvider>,
    );
    cy.get('[data-face] .cf-art').should("have.attr", "data-art-design", "ember-of-departure");
    cy.get("[data-face] .cf-starship-icon").should("have.length", PIECES.length);
    cy.get("[data-face] .cf").then(($card) => {
      const shown = $card[0] as HTMLElement;
      const box = shown.getBoundingClientRect();
      const icons = [...shown.querySelectorAll<HTMLElement>(".cf-starship-icon")];
      expect(icons.map((icon) => icon.dataset.module)).to.deep.equal(PIECES.map((id) => starshipModuleOf(id).kind));
      for (const icon of icons) {
        const rect = icon.getBoundingClientRect();
        expect(rect.width, icon.dataset.module).to.be.greaterThan(rect.height);
        expect(within(rect, box), `${String(icon.dataset.module)} on the face`).to.equal(true);
        expect(getComputedStyle(icon).backgroundImage, icon.dataset.module).to.contain("data:image/svg+xml");
      }
    });
    cy.get("[data-face]").screenshot("starship-art-pieces-list", { overwrite: true });
  });
});
