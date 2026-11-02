// R2721000: Legendary and Mythic faces wear no rarity foil by default, the crest telling them
// apart, in a real browser. apps/web's vitest suite proves which `data-foil` a face wears
// (cards/CardFace.test.tsx) and what the stylesheet declares (cards/foil.test.ts); what only a
// style engine can say is measured here: no sheen layer, no rainbow edge and no Mythic ring at the
// defaults, and with Rarity foil on, today's sheen, edge and ring on the Mythic.

import { CATALOG } from "@jackioh/cards";
import { CardFace, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

const LEGENDARY = "core-052";
const MYTHIC = "core-100";

/** A hand-sized full face, 5:7. */
const FACE_BOX = { width: 150, height: 210 } as const;

/** A board minion's box (card-cosmetics.cy.tsx's). */
const MINION_BOX = { width: 81, height: 113 } as const;

function Faces() {
  return (
    <div style={{ display: "flex", gap: 12, padding: 12, background: "#1b1f2b" }}>
      {[
        { id: LEGENDARY, size: "hand" },
        { id: MYTHIC, size: "hand" },
        { id: LEGENDARY, size: "board" },
        { id: MYTHIC, size: "board" },
      ].map(({ id, size }) => {
        const box = size === "hand" ? FACE_BOX : MINION_BOX;
        const face = faceModel({ defId: id, def: CATALOG[id], radiant: false });
        return (
          <div key={`${id}-${size}`} data-face={`${id}-${size}`} style={{ ...box, flex: "none", position: "relative" }}>
            {size === "hand" ? (
              <CardFace face={face} layout="full" />
            ) : (
              <MinionFace face={face} unit={unit("p1", { defId: id })} />
            )}
          </div>
        );
      })}
    </div>
  );
}

function faceOf(doc: Document, name: string): HTMLElement {
  const found = doc.querySelector<HTMLElement>(`[data-face="${name}"] > .cf`);
  if (found === null) throw new Error(`no ${name} face`);
  return found;
}

const FACES = [`${LEGENDARY}-hand`, `${MYTHIC}-hand`, `${LEGENDARY}-board`, `${MYTHIC}-board`];

describe("R2721000 the rarity marks on a face", () => {
  beforeEach(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    cy.viewport(700, 300);
  });

  after(() => {
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  it("R2721000 with the default settings a Legendary and a Mythic at hand and board sizes wear their crest and no rarity foil", () => {
    cy.mount(<Faces />);
    cy.document().should((doc) => {
      for (const name of FACES) {
        const cf = faceOf(doc, name);
        const box = name.endsWith("hand") ? FACE_BOX : MINION_BOX;
        expect(cf.getBoundingClientRect().width, `${name} fills its box`).to.be.greaterThan(box.width / 2);
        expect(cf.getAttribute("data-foil"), `${name} wears no foil`).to.equal("none");
        expect(getComputedStyle(cf, "::after").content, `${name} has no sheen layer`).to.equal("none");
        expect(getComputedStyle(cf).backgroundImage, `${name} has no rainbow edge`).to.not.contain("conic-gradient");
        const crest = cf.querySelector<HTMLElement>(".cf-crest");
        expect(crest?.getBoundingClientRect().width, `${name} wears its crest`).to.be.greaterThan(0);
      }
      // The Mythic's crest is tinted; the Legendary's is not.
      for (const size of ["hand", "board"]) {
        expect(getComputedStyle(faceOf(doc, `${MYTHIC}-${size}`).querySelector(".cf-crest") as Element).filter, `the Mythic ${size} crest`).to.contain(
          "hue-rotate",
        );
        expect(
          getComputedStyle(faceOf(doc, `${LEGENDARY}-${size}`).querySelector(".cf-crest") as Element).filter,
          `the Legendary ${size} crest`,
        ).to.not.contain("hue-rotate");
      }
      // The gem keeps its rainbow; the board portrait's ring does not.
      expect(
        getComputedStyle(faceOf(doc, `${MYTHIC}-hand`).querySelector(".cf-gem") as Element).backgroundImage,
        "the Mythic gem",
      ).to.contain("conic-gradient");
      expect(getComputedStyle(faceOf(doc, `${MYTHIC}-board`).querySelector(".cf-portrait") as Element).boxShadow, "the board Mythic ring").to.not.contain(
        "148, 208, 255",
      );
    });
  });

  it("R2721000 with Rarity foil on the Mythic wears today's sheen, rainbow edge and ring; the Legendary is unchanged", () => {
    writeCardSettings({ rarityFoil: true });
    cy.mount(<Faces />);
    cy.document().should((doc) => {
      for (const name of FACES) {
        const cf = faceOf(doc, name);
        const box = name.endsWith("hand") ? FACE_BOX : MINION_BOX;
        expect(cf.getBoundingClientRect().width, `${name} fills its box`).to.be.greaterThan(box.width / 2);
      }
      // The hand Mythic wears today's look: the animated sheen and the rainbow edge.
      const hand = faceOf(doc, `${MYTHIC}-hand`);
      expect(hand.getAttribute("data-foil")).to.equal("animated");
      const sheen = getComputedStyle(hand, "::after");
      expect(sheen.content, "the sheen layer").to.not.equal("none");
      expect(sheen.backgroundImage, "the Mythic sheen").to.contain("255, 106, 213");
      expect(getComputedStyle(hand).backgroundImage, "the rainbow edge").to.contain("conic-gradient");
      // The board Mythic's ring is back.
      expect(
        getComputedStyle(faceOf(doc, `${MYTHIC}-board`).querySelector(".cf-portrait") as Element).boxShadow,
        "the board Mythic ring",
      ).to.contain("148, 208, 255");
      // Both Legendary faces are as at the defaults.
      for (const name of [`${LEGENDARY}-hand`, `${LEGENDARY}-board`]) {
        const cf = faceOf(doc, name);
        expect(cf.getAttribute("data-foil"), `${name} wears no foil`).to.equal("none");
        expect(getComputedStyle(cf, "::after").content, `${name} has no sheen layer`).to.equal("none");
        expect(getComputedStyle(cf).backgroundImage, `${name} has no rainbow edge`).to.not.contain("conic-gradient");
      }
    });
  });
});
