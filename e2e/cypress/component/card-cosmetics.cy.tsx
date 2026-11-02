// R1295000: the card cosmetics in a real browser. apps/web's vitest suite proves which attributes a
// face wears (cards/cosmetics.test.tsx) and what the stylesheet declares; how the worn face draws
// beside the plain one, which only a style engine can say, is measured here. R1953001: a face wears its
// card's own picks, and a worn foil (and a chase finish or border) moves only on the worn face.

import { CATALOG } from "@jackioh/cards";
import { CardFace, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import {
  CARD_SLOT_KINDS,
  MOVING_FINISHES_AND_BORDERS,
  cosmeticsOf,
  resetCosmeticsForTests,
  writeCardPick,
} from "../../../apps/web/src/cards/cosmetics.ts";
import { writeSettings, __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

const DEF_ID = "core-004";

/** A hand-sized full face, 5:7. */
const FACE_BOX = { width: 150, height: 210 } as const;

/** A board minion's box (keyword-visuals.cy.tsx's large size). */
const MINION_BOX = { width: 81, height: 113 } as const;

/** The nine items: every entry of every kind but its default. */
const ITEMS = CARD_SLOT_KINDS.flatMap((kind) =>
  cosmeticsOf(kind)
    .filter((cosmetic) => !cosmetic.default)
    .map((cosmetic) => ({ kind, id: cosmetic.id })),
);

const FOILS = ITEMS.filter((item) => item.kind === "foil");

const BORDERS = ITEMS.filter((item) => item.kind === "border");

function face() {
  return faceModel({ defId: DEF_ID, def: CATALOG[DEF_ID], radiant: false });
}

function Pair({ minion = false }: { minion?: boolean }) {
  const box = minion ? MINION_BOX : FACE_BOX;
  const u = unit("p1", { defId: DEF_ID });
  return (
    <div style={{ display: "flex", gap: 12, padding: 12, background: "#1b1f2b" }}>
      {(["worn", "plain"] as const).map((which) => (
        <div key={which} data-which={which} style={{ ...box, flex: "none", position: "relative" }}>
          {minion ? <MinionFace face={face()} unit={u} worn={which === "worn"} /> : <CardFace face={face()} worn={which === "worn"} />}
        </div>
      ))}
    </div>
  );
}

function faceOf(doc: Document, which: "worn" | "plain"): HTMLElement {
  const found = doc.querySelector<HTMLElement>(`[data-which="${which}"] > .cf`);
  if (found === null) throw new Error(`no ${which} face`);
  return found;
}

/** Everything a cosmetic can change on a face, as one string. */
function look(cf: HTMLElement): string {
  const before = getComputedStyle(cf, "::before");
  const scale = cf.querySelector(".cf-scale");
  const art = cf.querySelector(".cf-art");
  return [
    getComputedStyle(cf).backgroundImage,
    before.backgroundImage,
    before.boxShadow,
    scale === null ? "" : getComputedStyle(scale).backgroundImage,
    art === null ? "" : getComputedStyle(art, "::after").backgroundImage,
  ].join(" | ");
}

function moving(cf: HTMLElement): boolean {
  return cf.getAnimations({ subtree: true }).some((animation) => animation.playState === "running");
}

function clearPicks(): void {
  for (const kind of CARD_SLOT_KINDS) writeCardPick(DEF_ID, kind, null);
}

describe("R1295000 the card cosmetics on a face", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    clearPicks();
    cy.viewport(400, 300);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    clearPicks();
  });

  for (const item of ITEMS) {
    it(`R1295000 ${item.id} draws on the worn face and not on the plain one`, () => {
      writeCardPick(DEF_ID, item.kind, item.id);
      cy.mount(<Pair />);
      cy.document().should((doc) => {
        const worn = faceOf(doc, "worn");
        const plain = faceOf(doc, "plain");
        expect(worn.getBoundingClientRect().width, "the face fills its box").to.be.greaterThan(FACE_BOX.width / 2);
        expect(look(worn), item.id).to.not.equal(look(plain));
        if (item.kind === "foil" || MOVING_FINISHES_AND_BORDERS.has(item.id)) {
          expect(moving(worn), "the worn foil, or chase finish or border, moves").to.equal(true);
          expect(moving(plain), "the plain face holds still").to.equal(false);
        }
      });
    });
  }

  // A border is one object on the full face and on the minion: the face paints it as its edge, and the
  // minion, whose panel covers the edge, as a ring layer (the panel's ::before) painted with that same
  // edge and masked down to a ring, over the panel's inner shadow, which the border leaves as it was.
  it("R1295000 a worn minion wears the border", () => {
    const edges: string[] = [];
    for (const border of BORDERS) {
      let edge = "";
      // A command, so that each border is the pick when its own faces mount, not the last one's.
      cy.then(() => {
        writeCardPick(DEF_ID, "border", border.id);
      });

      cy.mount(<Pair />);
      cy.document().should((doc) => {
        edge = getComputedStyle(faceOf(doc, "worn")).backgroundImage;
        expect(edge, `${border.id}'s edge on the full face`).to.not.equal("none");
      });

      cy.mount(<Pair minion />);
      cy.document().should((doc) => {
        const scaleOf = (which: "worn" | "plain"): Element => {
          const scale = faceOf(doc, which).querySelector(".cf-scale");
          if (scale === null) throw new Error(`no ${which} panel`);
          return scale;
        };
        const ring = getComputedStyle(scaleOf("worn"), "::before");
        expect(ring.backgroundImage, `${border.id}'s ring paints the full face's edge`).to.equal(edge);
        expect(ring.maskComposite, `${border.id}'s ring is masked to a ring`).to.contain("exclude");
        expect(ring.position, `${border.id}'s ring lies over the panel`).to.equal("absolute");
        expect(getComputedStyle(scaleOf("worn")).boxShadow, `${border.id} keeps the panel's inner shadow`).to.equal(
          getComputedStyle(scaleOf("plain")).boxShadow,
        );
        expect(getComputedStyle(scaleOf("plain"), "::before").content, "the plain minion has no ring").to.equal("none");
        edges.push(edge);
      });
    }
    cy.then(() => {
      expect(new Set(edges).size, "each border has an edge of its own").to.equal(BORDERS.length);
    });
  });

  for (const foil of FOILS) {
    it(`R1295000 ${foil.id} holds its still version under Reduce motion`, () => {
      writeSettings({ reduceMotion: true });
      writeCardPick(DEF_ID, "foil", foil.id);
      cy.mount(<Pair />);
      cy.get("html").should("have.attr", "data-reduce-motion", "true");
      cy.document().should((doc) => {
        const worn = faceOf(doc, "worn");
        const art = worn.querySelector(".cf-art");
        expect(art, "the art window").to.not.equal(null);
        const sheen = getComputedStyle(art as Element, "::after");
        expect(sheen.backgroundImage, "the still sheen").to.not.equal("none");
        expect(sheen.animationName).to.equal("none");
        expect(moving(worn)).to.equal(false);
      });
    });
  }
});
