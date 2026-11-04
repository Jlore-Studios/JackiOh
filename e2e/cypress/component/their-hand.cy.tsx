// #2933: the result panel's "Their hand" cards are full 5:7 faces at phone widths (R434).
// On staging (iPhone Safari, portrait, ~308 CSS px) the faces rendered as thin slivers, about
// 11 px wide: the block button's width comes from `aspect-ratio` alone, which WebKit does not
// pass on to the CardFace inside. jsdom cannot measure layout, so this mounts the finished game
// in a browser at 320, 390 and 1280 px and measures every face; `reveal.test.tsx` keeps the R434
// behavior green, this keeps the geometry. Every measurement sits inside `.should()`, so it
// retries while the deal animation settles, and carries controls, because "fits" is satisfied
// by a box that collapsed to nothing.

import { CATALOG } from "@jackioh/cards";

import { FACE_ASPECT } from "../../../apps/web/src/cards/index.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { writeSettings, __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { baseView, card, emptySide } from "../../../apps/web/src/test/fixtures.ts";

/** Fractional-pixel edge slack. */
const EPSILON = 1;

/** A name narrower than this is a sliver's syllable-per-line wrap, not a readable face. */
const NAME_FLOOR_PX = 30;

/** The issue's four cards, one radiant among them. */
const FOUR_DEFS = ["core-002", "core-019", "core-055", "core-004"] as const;

function fourHand(): ReturnType<typeof card>[] {
  return FOUR_DEFS.map((defId, index) => card({ instanceId: `t${String(index)}`, defId }));
}

/** A big hand reuses one def with distinct ids, so the catalog stays out of the count. */
function tenHand(): ReturnType<typeof card>[] {
  return Array.from({ length: 10 }, (_, index) => card({ instanceId: `w${String(index)}`, defId: "core-002" }));
}

function finished(hand: ReturnType<typeof card>[]): ReturnType<typeof baseView> {
  return baseView({
    phase: "over",
    result: { winner: "p2", reason: "hero-death" },
    opponent: emptySide("p2", { hand }),
  });
}

function mountFinished(hand: ReturnType<typeof card>[]): void {
  // Reduced motion makes `resultRevealMs()` 0, so the panel and its deal animation settle at once.
  writeSettings({ reduceMotion: true });
  cy.mount(
    <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
      <div className="app-shell app-shell--wide">
        <Game view={finished(hand)} legal={[]} onAction={() => undefined} />
      </div>
    </CatalogContext.Provider>,
  );
}

/** Every Their-hand card button: the section's own testid carries no trailing dash, so the prefix only matches cards. */
function theirCards(doc: Document): HTMLElement[] {
  return [...doc.querySelectorAll<HTMLElement>('[data-testid^="result-their-hand-"]')];
}

function checkCards(doc: Document, where: string, minWidth: number, expectedHeight: number): void {
  const cards = theirCards(doc);
  expect(cards.length, `Their hand holds its cards at ${where}`).to.be.greaterThan(0);
  for (const el of cards) {
    const label = el.getAttribute("data-testid") ?? "a card";
    const box = el.getBoundingClientRect();
    expect(box.height, `${label} is the ${String(expectedHeight)}px face at ${where}`).to.be.closeTo(expectedHeight, EPSILON);
    expect(box.width, `${label} is a full 5:7 face at ${where}`).to.be.closeTo(box.height * FACE_ASPECT, EPSILON);
    expect(box.width, `${label} is full width, not an 11px sliver, at ${where}`).to.be.at.least(minWidth);
    const face = el.querySelector<HTMLElement>(":scope > .cf");
    expect(face, `${label} draws a face at ${where}`).to.not.eq(null);
    if (face === null) continue;
    const drawn = face.getBoundingClientRect();
    expect(drawn.left, `${label} face fills its button's left at ${where}`).to.be.closeTo(box.left, EPSILON);
    expect(drawn.top, `${label} face fills its button's top at ${where}`).to.be.closeTo(box.top, EPSILON);
    expect(drawn.right, `${label} face fills its button's right at ${where}`).to.be.closeTo(box.right, EPSILON);
    expect(drawn.bottom, `${label} face fills its button's bottom at ${where}`).to.be.closeTo(box.bottom, EPSILON);
    const name = face.querySelector<HTMLElement>(".card-name");
    expect(name, `${label} has a name box at ${where}`).to.not.eq(null);
    if (name === null) continue;
    expect(name.clientWidth, `${label} name is readable at ${where}`).to.be.greaterThan(NAME_FLOOR_PX);
    expect(name.scrollWidth, `${label} name is not clipped at ${where}`).to.be.at.most(name.clientWidth + 1);
  }
}

function checkPage(doc: Document, where: string, viewportWidth: number): void {
  expect(doc.documentElement.scrollWidth, `no page-level horizontal scroll at ${where}`).to.be.at.most(viewportWidth);
}

describe("#2933 Their-hand cards are full 5:7 faces", () => {
  afterEach(() => {
    __resetSettingsForTests();
  });

  const widths = [
    { label: "phone", width: 320, height: 568, minWidth: 46, faceHeight: 66 },
    { label: "phone", width: 390, height: 844, minWidth: 46, faceHeight: 66 },
    { label: "desktop", width: 1280, height: 800, minWidth: 64, faceHeight: 92 },
  ] as const;

  for (const viewport of widths) {
    const where = `${viewport.label} ${String(viewport.width)}x${String(viewport.height)}`;

    it(`their hand cards are full 5:7 faces at ${where}`, () => {
      cy.viewport(viewport.width, viewport.height);
      mountFinished(fourHand());

      cy.get('[data-testid="result-overlay"]').should("have.attr", "data-revealed", "true");
      // Control: the catalog really is in, so the faces carry full names, not def ids.
      cy.get('[data-testid="result-their-hand"] .card-name').should(($names) => {
        const names = $names.toArray().map((el) => el.textContent ?? "");
        expect(names, "catalog names in Their hand").to.include.members(["Speciesist", "Midrange Menace", "Lava Golem"]);
        expect(
          names.filter((name) => /^core-\d/.test(name)),
          "def ids standing in for names",
        ).to.deep.equal([]);
      });

      cy.document({ log: false }).should((doc) => {
        expect(theirCards(doc).length, `four cards at ${where}`).to.eq(4);
        checkCards(doc, where, viewport.minWidth, viewport.faceHeight);
        checkPage(doc, where, viewport.width);
      });
    });
  }

  it("a 10-card hand wraps and scrolls without page overflow", () => {
    // Portrait first: the row wraps under its `max-height: 42vh` cap.
    cy.viewport(320, 568);
    mountFinished(tenHand());
    cy.get('[data-testid="result-overlay"]').should("have.attr", "data-revealed", "true");

    cy.document({ log: false }).should((doc) => {
      const grid = doc.querySelector<HTMLElement>(".their-hand__cards");
      expect(grid, "the row is mounted").to.not.eq(null);
      if (grid === null) return;
      expect(theirCards(doc).length, "ten cards").to.eq(10);
      checkCards(doc, "phone 320x568", 46, 66);
      const rows = new Set(theirCards(doc).map((el) => Math.round(el.getBoundingClientRect().top)));
      expect(rows.size, "the row wraps at phone 320x568").to.be.greaterThan(1);
      expect(grid.clientHeight, "the row stays within max-height: 42vh at phone 320x568").to.be.at.most(568 * 0.42 + 8);
      expect(grid.scrollHeight, "the row scrolls rather than grows at phone 320x568").to.be.at.least(grid.clientHeight);
      checkPage(doc, "phone 320x568", 320);
    });

    // Landscape after: 42vh of 320px is shorter than two rows, so the row genuinely scrolls.
    // This also covers the media query's landscape arm at the phone face size.
    cy.viewport(568, 320);
    cy.document({ log: false }).should((doc) => {
      const grid = doc.querySelector<HTMLElement>(".their-hand__cards");
      expect(grid, "the row is mounted").to.not.eq(null);
      if (grid === null) return;
      checkCards(doc, "landscape 568x320", 46, 66);
      expect(grid.clientHeight, "the row stays within max-height: 42vh at landscape 568x320").to.be.at.most(320 * 0.42 + 8);
      expect(grid.scrollHeight, "a 10-card hand scrolls at landscape 568x320").to.be.greaterThan(grid.clientHeight);
      checkPage(doc, "landscape 568x320", 568);
    });
  });
});
