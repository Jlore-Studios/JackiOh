// R2723000: a full-art card in the viewer's own hand, in a real browser. apps/web's vitest suite proves
// which parts a bare face renders (cards/CardFace.test.tsx) and which views keep them
// (cards/fullArtHand.test.tsx); what only a layout engine can say is measured here: a hand of a
// full-art Unit, a full-art Spell and a normal card draws the full-art faces with their cost gem and
// the Unit's attack and health on the art and no visible name, signature, rarity gem, set mark or
// crest, the name staying in the face's words at a rect no bigger than a pixel, and the normal card
// keeping its banner, drawn and inside its face. A screenshot of the hand is kept.

import { CATALOG } from "@jackioh/cards";
import { resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { baseView, card, emptySide, resetIds } from "../../../apps/web/src/test/fixtures.ts";

/** Felinor Flagbearer, a Legendary Unit that prints full art. */
const UNIT = "classicplus-046";
/** League of Losers, a Spell that prints full art. */
const SPELL = "classicplus-019";
/** Bigot, a card that prints none. */
const PLAIN = "core-002";

/** What a full-art face prints on its own and a hand's does not. */
const HIDDEN = ".card-name, .cf-gem, .cf-set, .cf-crest, .cf-signature";

/** Sub-pixel rounding between rects. */
const EPSILON = 0.5;

/** A visually hidden word is a box of one pixel; it gets one pixel and a half of slack for rounding. */
const HIDDEN_BOX_PX = 1 + EPSILON;

function nameOf(id: string): string {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`the catalog has no ${id}`);
  return def.name;
}

function handView() {
  resetIds();
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "fa-unit", defId: UNIT }),
        card({ instanceId: "fa-spell", defId: SPELL }),
        card({ instanceId: "plain", defId: PLAIN }),
      ],
    }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
  });
}

function must<T extends Element>(found: T | null, what: string): T {
  if (found === null) throw new Error(`no ${what}`);
  return found;
}

function clean(): void {
  localStorage.clear();
  __resetSettingsForTests();
  resetCosmeticsForTests();
}

/** The face of a hand card, by its instance id. */
function faceOf(hand: Element, instanceId: string): HTMLElement {
  return must(hand.querySelector<HTMLElement>(`[data-testid="hand-card-${instanceId}"] > .cf`), `face of ${instanceId}`);
}

/** `part` has a box with an area, whole inside `face`. */
function expectDrawnInside(face: Element, part: string, what: string): void {
  const inner = must(face.querySelector(part), `${part} of ${what}`).getBoundingClientRect();
  const outer = face.getBoundingClientRect();
  expect(inner.width, `${what}: ${part} has a width`).to.be.greaterThan(0);
  expect(inner.height, `${what}: ${part} has a height`).to.be.greaterThan(0);
  expect(inner.left, `${what}: ${part} inside the face, left`).to.be.at.least(outer.left - EPSILON);
  expect(inner.right, `${what}: ${part} inside the face, right`).to.be.at.most(outer.right + EPSILON);
  expect(inner.top, `${what}: ${part} inside the face, top`).to.be.at.least(outer.top - EPSILON);
  expect(inner.bottom, `${what}: ${part} inside the face, bottom`).to.be.at.most(outer.bottom + EPSILON);
}

describe("R2723000 a full-art card in the viewer's own hand", () => {
  beforeEach(() => {
    clean();
    cy.viewport(1280, 800);
    cy.mount(
      <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
        <div className="app-shell app-shell--wide">
          <Game view={handView()} legal={[]} onAction={() => undefined} wearCosmetics />
        </div>
      </CatalogContext.Provider>,
    );
  });

  after(clean);

  it("R2723000 a hand of a full-art Unit, a full-art Spell and a normal card: the full-art faces draw their art, cost gem and the Unit's stats and no visible name; the normal card keeps its banner", () => {
    cy.get('[data-testid="hand-you"]').should(($hand) => {
      const hand = $hand.get(0);
      if (hand === undefined) throw new Error("no hand");

      for (const [instanceId, id, stats] of [
        ["fa-unit", UNIT, true],
        ["fa-spell", SPELL, false],
      ] as const) {
        const face = faceOf(hand, instanceId);
        expect(face.getAttribute("data-full-art-hand"), `${id} is bare in hand`).to.eq("true");
        expect(face.querySelectorAll(HIDDEN).length, `${id} draws none of ${HIDDEN}`).to.eq(0);

        const name = must(face.querySelector(".cf-hand-name"), `name of ${id}`);
        expect(name.textContent, `${id}'s name is in its words`).to.eq(nameOf(id));
        const box = name.getBoundingClientRect();
        expect(box.width, `${id}'s name is no wider than a pixel`).to.be.at.most(HIDDEN_BOX_PX);
        expect(box.height, `${id}'s name is no taller than a pixel`).to.be.at.most(HIDDEN_BOX_PX);

        expectDrawnInside(face, ".cost-gem", id);
        if (stats) {
          expectDrawnInside(face, ".cf-atk", id);
          expectDrawnInside(face, ".cf-hp", id);
        } else {
          expect(face.querySelector(".cf-stats"), `${id} has no stats`).to.eq(null);
        }
      }

      const plain = faceOf(hand, "plain");
      expect(plain.hasAttribute("data-full-art-hand"), "the normal card is not bare").to.eq(false);
      const banner = must(plain.querySelector(".card-name"), "name banner of the normal card");
      expect(banner.textContent).to.eq("Bigot");
      expectDrawnInside(plain, ".card-name", PLAIN);
      const box = banner.getBoundingClientRect();
      expect(box.width, "the banner is wider than a pixel").to.be.greaterThan(1);
      expect(box.height, "the banner is taller than a pixel").to.be.greaterThan(1);
    });

    cy.get('[data-testid="hand-you"]').screenshot("R2723000-full-art-hand", { overwrite: true });
  });
});
