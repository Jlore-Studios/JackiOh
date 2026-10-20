// Issue #1308: the pack-opening scene (`packs/PackOpening.tsx`) measured in a real browser, from the
// fixture answer `POST /api/packs/open` gives (`packs/fixtures.ts`): a Mythic, an Epic, a cosmetic, a
// bar and a card. Only a layout engine can say these (jsdom has none; PackOpening.test.tsx holds the
// rest):
//   - the sealed pack glows in the best rarity's colour the moment the server has answered, and once
//     it has torn the five cards are laid out,
//   - skip-all lays out five whole faces, 5:7 each, on the screen and apart, on a desktop and on a phone,
//   - under Reduce motion the results show at once and nothing in the scene animates.
//
// The scene mounts in a portal on the body, over whatever is there, so no page is built around it. The
// clock is the spec's own (`cy.clock`) where the tear is waited on, since a spec waits on no real one.

import { PackScene } from "../../../apps/web/src/packs/PackOpening.tsx";
import { openedPack } from "../../../apps/web/src/packs/fixtures.ts";
import { PACK_TEAR_MS, packTestid } from "../../../apps/web/src/packs/packs.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

/** A face is 5:7, as every card is: the height over the width is 1.4. */
const FACE_RATIO = 1.4;
/** Subpixel slack for edges the browser rounds. */
const RATIO_TOLERANCE = 0.05;
const EDGE_EPSILON = 0.5;

const VIEWPORTS = [
  { width: 1280, height: 720 },
  { width: 390, height: 844 },
] as const;

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

function mountScene(): void {
  cy.mount(
    <PackScene
      pack="Core"
      opened={openedPack()}
      busy={false}
      error={null}
      left={0}
      onTear={() => undefined}
      onNext={() => undefined}
      onClose={() => undefined}
    />,
  );
}

function overlaps(a: DOMRect, b: DOMRect): boolean {
  return a.left < b.right - EDGE_EPSILON && b.left < a.right - EDGE_EPSILON && a.top < b.bottom - EDGE_EPSILON && b.top < a.bottom - EDGE_EPSILON;
}

describe("#1308 the pack-opening scene", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    cy.viewport(1280, 720);
  });

  after(() => {
    __resetSettingsForTests();
  });

  it("the pack glows in its best rarity before the reveal", () => {
    cy.clock();
    mountScene();
    cy.get(ts(packTestid.pack)).should(($pack) => {
      const doc = $pack[0]!.ownerDocument;
      const win = doc.defaultView!;
      const glow = win.getComputedStyle($pack[0]!).getPropertyValue("--pack-glow").trim();
      const mythic = win.getComputedStyle(doc.documentElement).getPropertyValue("--rarity-mythic").trim();
      expect(mythic, "the mythic colour is defined").not.to.equal("");
      expect(glow).to.equal(mythic);
    });
    cy.get(ts(packTestid.slots)).should("not.exist");

    cy.tick(PACK_TEAR_MS);
    cy.get(ts(packTestid.pack)).should("not.exist");
    cy.get(".pack-slot").should("have.length", 5);
    cy.get(ts(packTestid.slot(0))).should("have.attr", "data-rarity", "Mythic");
  });

  for (const { width, height } of VIEWPORTS) {
    it(`skip-all lays out five whole faces, apart and on screen, at ${String(width)}x${String(height)}`, () => {
      cy.viewport(width, height);
      mountScene();
      cy.get(ts(packTestid.skipAll)).click();
      cy.get(".pack-slot .cf").should(($faces) => {
        expect($faces, "five faces").to.have.length(5);
        const win = $faces[0]!.ownerDocument.defaultView!;
        const rects = $faces.toArray().map((face) => face.getBoundingClientRect());
        for (const [index, rect] of rects.entries()) {
          expect(rect.width, `face ${String(index)} has a width`).to.be.greaterThan(0);
          expect(rect.height / rect.width, `face ${String(index)}'s height over its width`).to.be.closeTo(FACE_RATIO, RATIO_TOLERANCE);
          expect(rect.left, `face ${String(index)}'s left edge`).to.be.at.least(-EDGE_EPSILON);
          expect(rect.right, `face ${String(index)}'s right edge`).to.be.at.most(win.innerWidth + EDGE_EPSILON);
        }
        for (let a = 0; a < rects.length; a += 1) {
          for (let b = a + 1; b < rects.length; b += 1) {
            expect(overlaps(rects[a]!, rects[b]!), `faces ${String(a)} and ${String(b)} overlap`).to.equal(false);
          }
        }
      });
      cy.get(ts(packTestid.close)).should("have.text", "Done");
    });
  }

  it("under Reduce motion the results show at once with no animation", () => {
    writeSettings({ reduceMotion: true });
    mountScene();
    cy.get(ts(packTestid.scene)).should("have.attr", "data-motion", "reduce");
    cy.get(ts(packTestid.pack)).should("not.exist");
    cy.get(ts(packTestid.fx)).should("not.exist");
    cy.get(".pack-slot .cf").should("have.length", 5);
    cy.get(ts(packTestid.scene)).should(($scene) => {
      const win = $scene[0]!.ownerDocument.defaultView!;
      const everything = [$scene[0]!, ...Array.from($scene[0]!.querySelectorAll("*"))];
      for (const element of everything) {
        for (const pseudo of [null, "::before", "::after"]) {
          expect(win.getComputedStyle(element, pseudo).animationName, `${element.tagName}.${element.className.toString()} ${pseudo ?? ""}`).to.equal("none");
        }
      }
    });
  });
});
