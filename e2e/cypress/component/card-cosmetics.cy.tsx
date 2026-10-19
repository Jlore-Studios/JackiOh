// R1295000: the card cosmetics in a real browser. apps/web's vitest suite proves which attributes a
// face wears (cards/cosmetics.test.tsx) and what the stylesheet declares; what only a style engine
// can say is measured here: each of the nine items changes how the worn face draws and leaves the
// plain face beside it as it was, a worn foil moves only on the worn face, the board minion wears
// its border, and under Reduce motion every animated foil holds its still version.
//
// A face's look is what the cosmetics can reach, read off the computed styles: the root's edge,
// the finish layer's background and shadow, the panel and the art's sheen.

import { CATALOG } from "@jackioh/cards";
import { CardFace, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  cosmeticsOf,
  resetCosmeticsForTests,
  writeCosmetics,
  type CosmeticKind,
} from "../../../apps/web/src/cards/cosmetics.ts";
import { writeSettings, __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

const DEF_ID = "core-004";

/** A hand-sized full face, 5:7. */
const FACE_BOX = { width: 150, height: 210 } as const;

/** A board minion's box (keyword-visuals.cy.tsx's large size). */
const MINION_BOX = { width: 81, height: 113 } as const;

const KINDS: readonly CosmeticKind[] = ["foil", "finish", "border"];

/** The nine items: every entry of every kind but its default. */
const ITEMS = KINDS.flatMap((kind) =>
  cosmeticsOf(kind)
    .filter((cosmetic) => !cosmetic.default)
    .map((cosmetic) => ({ kind, id: cosmetic.id })),
);

const FOILS = ITEMS.filter((item) => item.kind === "foil");

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

describe("R1295000 the card cosmetics on a face", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCosmetics({ ...DEFAULT_CARD_COSMETICS });
    cy.viewport(400, 300);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCosmetics({ ...DEFAULT_CARD_COSMETICS });
  });

  for (const item of ITEMS) {
    it(`R1295000 ${item.id} draws on the worn face and not on the plain one`, () => {
      writeCosmetics({ [item.kind]: item.id });
      cy.mount(<Pair />);
      cy.document().should((doc) => {
        const worn = faceOf(doc, "worn");
        const plain = faceOf(doc, "plain");
        expect(worn.getBoundingClientRect().width, "the face fills its box").to.be.greaterThan(FACE_BOX.width / 2);
        expect(look(worn), item.id).to.not.equal(look(plain));
        if (item.kind === "foil") {
          expect(moving(worn), "the worn foil moves").to.equal(true);
          expect(moving(plain), "the plain face holds still").to.equal(false);
        }
      });
    });
  }

  it("R1295000 a worn minion wears the border", () => {
    writeCosmetics({ border: ITEMS.find((item) => item.kind === "border")?.id });
    cy.mount(<Pair minion />);
    cy.document().should((doc) => {
      const shadow = (which: "worn" | "plain"): string => {
        const scale = faceOf(doc, which).querySelector(".cf-scale");
        return scale === null ? "" : getComputedStyle(scale).boxShadow;
      };
      expect(shadow("worn")).to.not.equal(shadow("plain"));
    });
  });

  for (const foil of FOILS) {
    it(`R1295000 ${foil.id} holds its still version under Reduce motion`, () => {
      writeSettings({ reduceMotion: true });
      writeCosmetics({ foil: foil.id });
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
