// Issue #1308: the pack-opening scene (`packs/PackOpening.tsx`) measured in a real browser, from the fixture
// answer `POST /api/packs/open` gives (`packs/fixtures.ts`): a Mythic, an Epic, a cosmetic, a bar and a
// card. Only a layout engine can say these (jsdom has none; PackOpening.test.tsx holds the rest). The scene
// mounts in a portal on the body, so no page is built around it; the clock is the spec's own (`cy.clock`)
// where the tear is waited on.

import type { OpenedPackResponse } from "../../../apps/web/src/net/api.ts";
import { PackScene } from "../../../apps/web/src/packs/PackOpening.tsx";
import { openedPack } from "../../../apps/web/src/packs/fixtures.ts";
import { PACK_TEAR_MS, packTestid } from "../../../apps/web/src/packs/packs.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

/** A face is 5:7, as every card is: the height over the width is 1.4. */
const FACE_RATIO = 1.4;
/** Subpixel slack for edges the browser rounds. */
const RATIO_TOLERANCE = 0.05;
const EDGE_EPSILON = 0.5;
/** The smallest tap target a phone's thumb finds without hunting, in CSS pixels. */
const TOUCH_PX = 44;
/** #1999: a compact HUD takes at most this share of a phone's height. */
const HUD_MAX_SHARE = 0.1;

const PHONES = [
  { width: 375, height: 667 },
  { width: 360, height: 640 },
] as const;

const VIEWPORTS = [{ width: 1280, height: 720 }, { width: 390, height: 844 }, ...PHONES] as const;

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

function mountScene(opened: OpenedPackResponse | null = openedPack()): void {
  cy.mount(
    <PackScene
      pack="Core"
      opened={opened}
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

function onScreen(rect: DOMRect, win: Window): boolean {
  return rect.left >= -EDGE_EPSILON && rect.top >= -EDGE_EPSILON && rect.right <= win.innerWidth + EDGE_EPSILON && rect.bottom <= win.innerHeight + EDGE_EPSILON;
}

/** #1999: neither the scene nor the page scrolls sideways. */
function expectNoSidewaysScroll(): void {
  cy.get(ts(packTestid.scene)).should(($scene) => {
    const scene = $scene[0]!;
    const win = scene.ownerDocument.defaultView!;
    expect(scene.scrollWidth, "the scene's width").to.be.at.most(scene.clientWidth);
    expect(scene.ownerDocument.documentElement.scrollWidth, "the page's width").to.be.at.most(win.innerWidth);
  });
}

/** #1999: `selector` is a tap target at least `TOUCH_PX` square, wholly on the screen without a scroll. */
function expectInReach(selector: string): void {
  cy.get(selector).should(($target) => {
    const rect = $target[0]!.getBoundingClientRect();
    expect(rect.width, `${selector}'s width`).to.be.at.least(TOUCH_PX);
    expect(rect.height, `${selector}'s height`).to.be.at.least(TOUCH_PX);
    expect(onScreen(rect, $target[0]!.ownerDocument.defaultView!), `${selector} is on the screen`).to.equal(true);
  });
}

/** #1999: the pinned HUD is compact and covers none of the five cards. */
function expectHudClearOfSlots(): void {
  cy.get(ts(packTestid.bars)).should(($hud) => {
    const hud = $hud[0]!.getBoundingClientRect();
    const win = $hud[0]!.ownerDocument.defaultView!;
    expect(hud.height, "the HUD's height").to.be.at.most(win.innerHeight * HUD_MAX_SHARE);
    const slots = Array.from($hud[0]!.ownerDocument.querySelectorAll(".pack-slot"));
    expect(slots, "five cards").to.have.length(5);
    for (const [index, slot] of slots.entries()) {
      expect(overlaps(hud, slot.getBoundingClientRect()), `the HUD covers card ${String(index)}`).to.equal(false);
    }
  });
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
      expect(mythic, "the mythic color is defined").not.to.equal("");
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

describe("#1999 the pack-opening scene on a phone", () => {
  beforeEach(() => {
    __resetSettingsForTests();
  });

  after(() => {
    __resetSettingsForTests();
  });

  for (const { width, height } of PHONES) {
    const size = `${String(width)}x${String(height)}`;

    it(`sealed, the pack and the actions are taps in reach at ${size}`, () => {
      cy.viewport(width, height);
      mountScene(null);
      cy.get(ts(packTestid.pack)).should("have.attr", "data-state", "sealed");
      expectInReach(ts(packTestid.pack));
      expectInReach(ts(packTestid.skipAll));
      expectInReach(ts(packTestid.close));
      expectNoSidewaysScroll();
    });

    it(`mid-reveal, every card is on one screen clear of the HUD at ${size}`, () => {
      cy.viewport(width, height);
      cy.clock();
      mountScene();
      // The tear's timer starts once the answered scene is drawn, so the clock moves only after it.
      cy.get(ts(packTestid.pack)).should("have.attr", "data-state", "tearing");
      cy.tick(PACK_TEAR_MS);
      // The Common and the Rare turn on a tap; the Mythic and the Epic wait for a hold.
      cy.get(ts(packTestid.flip(4))).click({ scrollBehavior: false });
      cy.get(ts(packTestid.flip(3))).click({ scrollBehavior: false });
      cy.get(ts(packTestid.outcome(3))).should("exist");
      cy.get(".pack-slot").should(($slots) => {
        expect($slots, "five cards").to.have.length(5);
        const win = $slots[0]!.ownerDocument.defaultView!;
        for (const [index, slot] of $slots.toArray().entries()) {
          expect(onScreen(slot.getBoundingClientRect(), win), `card ${String(index)} is on the screen`).to.equal(true);
        }
      });
      expectHudClearOfSlots();
      for (const index of [0, 1, 2]) expectInReach(ts(packTestid.flip(index)));
      expectInReach(ts(packTestid.skipAll));
      expectInReach(ts(packTestid.close));
      expectNoSidewaysScroll();
    });

    it(`all revealed, the HUD is clear of every card and Done is in reach at ${size}`, () => {
      cy.viewport(width, height);
      mountScene();
      cy.get(ts(packTestid.skipAll)).click({ scrollBehavior: false });
      cy.get(".pack-slot .cf").should("have.length", 5);
      expectNoSidewaysScroll();
      expectHudClearOfSlots();
      cy.get(ts(packTestid.close)).should("have.text", "Done");
      expectInReach(ts(packTestid.close));
    });
  }
});
