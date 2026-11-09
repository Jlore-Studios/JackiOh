// R3335000: a launched Starship wears its seat's Starship cosmetics, a hull design, a livery, an engine trail and a
// piece skin: an online board's opponent wears the loadout its seat froze (R1834004), here three Fleet track looks
// (R3335001) and the Jlorious one (R3335003). jsdom draws no picture, so apps/web/src/cards/starship.test.tsx checks
// the attributes; here Chrome lays the board out at both of BUILD M5-T1's viewports, and each look's art must carry
// its hull's design, have a size on its card, and differ from the other three.
// Run: E2E_COMPONENT_PORT=5281 pnpm --dir e2e test:component --spec cypress/component/starship-cosmetics.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { HULL_DESIGNS, type StarshipLook } from "../../../apps/web/src/cards/art/starshipLooks.ts";
import { DEFAULT_CARD_COSMETICS, resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import type { SeatLoadouts } from "../../../apps/web/src/cards/loadouts.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, emptySide, unit } from "../../../apps/web/src/test/fixtures.ts";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

const GAME = '[data-testid="game"]';
const SHIP = '[data-testid="card-fleet"]';
const HULL = "stinkyscape-t-dormant-starship";
/** Six pieces from as many families: a beam, a drill, a shield, a capsule, rounds, a repeater. */
const PIECES = ["stinkyscape-058", "stinkyscape-066", "stinkyscape-038", "stinkyscape-043", "stinkyscape-100", "stinkyscape-051"] as const;

/** Three Fleet track looks, the capstone hull among them, and the Jlorious hull over the standard rest. */
const LOOKS: readonly { label: string; look: StarshipLook }[] = [
  {
    label: "fleet-scout",
    look: {
      starshipHull: "starship-hull-scout",
      starshipLivery: "starship-livery-hazard",
      starshipTrail: "starship-trail-ion",
      starshipPieceSkin: "starship-piece-skin-gunmetal",
    },
  },
  {
    label: "fleet-corsair",
    look: {
      starshipHull: "starship-hull-corsair",
      starshipLivery: "starship-livery-midnight",
      starshipTrail: "starship-trail-aurora",
      starshipPieceSkin: "starship-piece-skin-neon",
    },
  },
  {
    label: "fleet-stinkyscape",
    look: {
      starshipHull: "starship-hull-stinkyscape",
      starshipLivery: "starship-livery-flagship",
      starshipTrail: "starship-trail-singularity",
      starshipPieceSkin: "starship-piece-skin-chrome",
    },
  },
  {
    label: "jlorious",
    look: {
      starshipHull: "starship-hull-jlorious",
      starshipLivery: DEFAULT_CARD_COSMETICS.starshipLivery,
      starshipTrail: DEFAULT_CARD_COSMETICS.starshipTrail,
      starshipPieceSkin: DEFAULT_CARD_COSMETICS.starshipPieceSkin,
    },
  },
];

/** The opponent's launched Starship of six pieces in lane 3, under the name whose design a standard hull keeps. */
function fleetBoard(): ReturnType<typeof baseView> {
  const theirs = [null, null, null, null, null] as (ReturnType<typeof unit> | null)[];
  theirs[2] = unit("p2", {
    instanceId: "fleet",
    defId: HULL,
    attack: 0,
    starship: { pieces: PIECES.map((defId) => ({ defId })), name: "The Ninth Life", launched: 1 },
  });
  return baseView({ opponent: emptySide("p2", { hand: { count: 4 }, units: theirs }) });
}

/** Both seats' frozen loadouts: the viewer's the defaults, the opponent's `look`. */
function loadoutsWearing(look: StarshipLook): SeatLoadouts {
  return {
    p1: { seat: { ...DEFAULT_CARD_COSMETICS }, cards: {} },
    p2: { seat: { ...DEFAULT_CARD_COSMETICS, ...look }, cards: {} },
  };
}

describe("R3335000 an opponent's launched Starship wears its seat's Starship cosmetics", () => {
  beforeEach(() => {
    localStorage.clear();
    resetCosmeticsForTests();
  });

  for (const viewport of VIEWPORTS) {
    it(`R3335000 an opponent's launched Starship in three Fleet looks and the Jlorious one at ${viewport.label} size`, () => {
      cy.viewport(viewport.width, viewport.height);
      const lookup = lookupFromDefs(CATALOG);
      const noop = (): void => undefined;
      const pictures: string[] = [];
      for (const { label, look } of LOOKS) {
        cy.mount(
          <CatalogContext.Provider value={lookup}>
            <div className="app-shell app-shell--wide">
              <Game view={fleetBoard()} legal={[]} onAction={noop} wearCosmetics loadouts={loadoutsWearing(look)} />
            </div>
          </CatalogContext.Provider>,
        );
        cy.get(GAME).should("exist");
        cy.get(`${SHIP} .cf-art`)
          .should("have.attr", "data-art-design", HULL_DESIGNS[look.starshipHull] ?? "")
          .and("have.attr", "style")
          .and("contain", "data:image/svg+xml");
        cy.get(SHIP).then(($card) => {
          const card = $card[0] as HTMLElement;
          const art = card.querySelector<HTMLElement>(".cf-art");
          if (art === null) throw new Error(`${label}: no art`);
          const box = art.getBoundingClientRect();
          const frame = card.getBoundingClientRect();
          expect(box.width, `${label}: the art has a size`).to.be.greaterThan(0);
          expect(box.height, `${label}: the art has a size`).to.be.greaterThan(0);
          expect(box.left >= frame.left - 1 && box.right <= frame.right + 1, `${label}: the art sits on its card`).to.equal(true);
          pictures.push(art.style.backgroundImage);
        });
        cy.screenshot(`starship-cosmetics-${label}-${viewport.label}`, { capture: "viewport", overwrite: true });
      }
      // Four looks, four pictures, every one its own.
      cy.then(() => {
        expect(pictures).to.have.length(LOOKS.length);
        expect(new Set(pictures).size).to.equal(LOOKS.length);
      });
    });
  }
});
