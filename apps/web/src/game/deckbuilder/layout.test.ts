// R3956000: the deck workshop's pool and deck sit side by side from 601 px, the rail shows a deck's name whole,
// the bar's title sits beside Back, and the Cards tab's set names are whole with a row that fades where it scrolls.
// The sheets are read as text, comments cut out; the Cypress specs (46-menus-fit-one-screen.cy.ts and
// deckbuilder-layout.cy.tsx) measure the drawn layout.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

const read = (file: string): string => readFileSync(join(SRC, file), "utf8");
/** The sheet with its block comments cut out. */
const sheet = (file: string): string => read(file).replace(/\/\*[\s\S]*?\*\//g, " ");

/** The body of the first rule whose selector is exactly `selector`, at or after `from`. */
function rule(css: string, selector: string, from = 0): string {
  const start = css.indexOf(`${selector} {`, from);
  expect(start, `a rule for ${selector}`).toBeGreaterThanOrEqual(0);
  return css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
}

describe("R3956000 the workshop's pool and deck side by side", () => {
  it("R3956000 the editor stacks at 600 px and under and sits side by side from 601 px", () => {
    const sheets = ["game/deckbuilder/deckbuilder.css", "game/deckbuilder/workshop.css", "routes/almanac.css", "dungeon/dungeon.css"];
    for (const file of sheets) {
      const css = sheet(file);
      // The old stack and split, at 760 and 761 px, are gone: the pairs now meet at 600 and 601.
      expect(css, file).not.toContain("(max-width: 760px) and (min-height: 501px)");
      expect(css, file).not.toContain("(min-width: 761px) and (min-height: 501px)");
    }

    const deckbuilder = sheet("game/deckbuilder/deckbuilder.css");
    // From 601 to 760 px the sidebar is 232 px beside the pool.
    const mid = deckbuilder.indexOf("@media (min-width: 601px) and (max-width: 760px) {");
    expect(mid, "a block from 601 to 760 px").toBeGreaterThanOrEqual(0);
    expect(rule(deckbuilder, ".db-layout", mid)).toContain("grid-template-columns: minmax(0, 1fr) 232px;");
    // A phone alone stacks the sidebar over the pool.
    const phone = deckbuilder.indexOf("@media (max-width: 600px) {");
    expect(phone, "a block at 600 px and under").toBeGreaterThanOrEqual(0);
    const stacked = rule(deckbuilder, ".db-layout", phone);
    expect(stacked).toContain("grid-template-columns: minmax(0, 1fr);");
    expect(stacked).toContain('"sidebar"');

    // The bar's line dissolves into the bar from 601 to 1024 px, so the title sits beside Back: the workshop sets the
    // bar's custom properties there (routes/top-bar.css places the slots and reads them, with the phone's layout as the default).
    const workshop = sheet("game/deckbuilder/workshop.css");
    const tablet = workshop.indexOf("@media (min-width: 601px) and (max-width: 1024px) {");
    expect(tablet, "a block from 601 to 1024 px").toBeGreaterThanOrEqual(0);
    const root = rule(workshop, ".workshop", tablet);
    expect(root).toContain("--bar-line-display: contents;");
    expect(root).toContain("--bar-extras-order: 2;");
    expect(root).toContain("--bar-extras-flex: 1 0 100%;");
    const bar = sheet("routes/top-bar.css");
    expect(rule(bar, ".screen-nav__line")).toContain("display: var(--bar-line-display, flex);");
    expect(rule(bar, ".screen-nav__extra")).toContain("order: var(--bar-extras-order, 0);");
    expect(rule(bar, ".screen-nav__extra")).toContain("flex: var(--bar-extras-flex, 1 1 auto);");
  });

  it("R3956000 a deck's name in the rail wraps, with no ellipsis", () => {
    const name = rule(sheet("game/deckbuilder/workshop.css"), ".ws-row-name");
    expect(name).toContain("line-clamp: 3;");
    expect(name).toContain("overflow-wrap: anywhere;");
    expect(name).not.toContain("text-overflow");
    expect(name).not.toContain("nowrap");
    // A phone's Back to decks keeps its words for a reader, in a span the name field's room is taken from.
    expect(read("game/deckbuilder/DeckEditor.tsx")).toContain("ws-back-label");
    expect(rule(sheet("game/deckbuilder/workshop.css"), ".ws-editor--deck .ws-back-label")).toContain("clip-path: inset(50%);");
  });
});

describe("R3956000 the Cards tab's set row", () => {
  it("R3956000 the Cards tab's set names are whole and its row fades where it scrolls", () => {
    const css = sheet("collection/browse.css");
    expect(rule(css, ".collection-screen .collection-set__name")).not.toContain("text-overflow");
    const list = rule(css, ".collection-screen .collection-set-list");
    expect(list).toContain("mask-image:");
    expect(list).not.toContain("scrollbar-width: none");
  });
});

describe("R4349000 the deck editor's Compare with picker", () => {
  it("R4349000 from 601 to 760 px on a tall screen the label sits over the select, which takes the body's size", () => {
    const workshop = sheet("game/deckbuilder/workshop.css");
    const narrow = workshop.indexOf("@media (max-width: 760px) {");
    expect(narrow, "the 760 px block").toBeGreaterThanOrEqual(0);
    const tall = workshop.indexOf("@media (min-width: 601px) and (min-height: 501px) and (max-width: 760px) {");
    expect(tall, "a block from 601 to 760 px on a tall screen").toBeGreaterThan(narrow);
    const pick = rule(workshop, ".ws-compare-pick", tall);
    expect(pick).toContain("flex-direction: column;");
    expect(pick).toContain("align-items: stretch;");
    expect(rule(workshop, ".ws-compare-pick .ws-select", tall)).toContain("font-size: var(--type-md);");
    // The phone's row and its 16 px stay in the 760 px block.
    expect(rule(workshop, ".ws-compare-pick", narrow)).toContain("flex-direction: row;");
    expect(rule(workshop, ".ws-compare-pick .ws-select", narrow)).toContain("flex: 1 1 auto;");
  });
});
