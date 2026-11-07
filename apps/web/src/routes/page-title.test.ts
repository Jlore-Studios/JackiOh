// R3223000 (#3223): every menu names itself with one PageTitle, and one sheet sizes an `h1`. jsdom ignores the
// cascade, so these read the sources: no screen draws an `h1` of its own, no menu sheet sets a font-size on an
// `h1`, and page-title.css holds the title on one face and one step of the scale. What each route renders (its one
// visible `h1`, the PageTitle) is proven in that route's own test and in page-titles.test.tsx.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

function walk(dir: string, ending: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "node_modules" || name === "wasm" ? [] : walk(path, ending);
    return name.endsWith(ending) ? [relative(SRC, path).replaceAll("\\", "/")] : [];
  });
}

/** A file of src/, its comments cut out, so the words a comment uses to say what it replaced count for nothing. */
function code(file: string, comment: RegExp): string {
  return readFileSync(join(SRC, file), "utf8").replace(comment, " ");
}

const CSS_COMMENT = /\/\*[\s\S]*?\*\//g;
const JSX_COMMENT = /\{\/\*[\s\S]*?\*\/\}|\/\*[\s\S]*?\*\/|(?<![:"'`])\/\/[^\n]*/g;

/** The declarations of the first rule that opens with `selector`. */
function block(css: string, selector: string): string {
  const start = css.indexOf(`${selector} {`);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  return css.slice(css.indexOf("{", start) + 1, css.indexOf("}", start));
}

describe("R3223000 one PageTitle per menu", () => {
  it("R3223000 only PageTitle draws a menu's h1", () => {
    const counts: Record<string, number> = {};
    for (const file of walk(SRC, ".tsx").filter((name) => !name.includes(".test."))) {
      const found = code(file, JSX_COMMENT).match(/<h1[\s>]/g)?.length ?? 0;
      if (found > 0) counts[file] = found;
    }
    // PageTitle's own; then what is not a menu: the shell's logo, the landing's wordmark, the boards' hidden
    // headings, and the developer pages. The settings dialog's title is its Dialog's h2 (R3221000).
    expect(counts).toEqual({
      "routes/PageTitle.tsx": 1,
      "main.tsx": 1,
      "routes/landing.tsx": 1,
      "routes/practice.tsx": 1,
      "dungeon/fight/FightScreen.tsx": 1,
      "duos/DevDuosRoute.tsx": 2,
      "routes/dev/hotseat.tsx": 3,
    });
  });

  it("R3223000 no stylesheet but page-title.css sets a font-size on an h1, and the brand is not an h1", () => {
    // index.css's `.app-shell h1` is the base the board's own pages inherit, and the landing's `.landing h1` is its
    // wordmark; page-title.css outweighs the first on every menu.
    const base = new Set(["routes/page-title.css", "index.css", "routes/landing.css"]);
    const sheets = walk(SRC, ".css");
    expect(sheets.length).toBeGreaterThan(30);
    const bareH1 = /(?<![\w.#-])h1(?![\w-])/;
    for (const file of sheets) {
      const css = code(file, CSS_COMMENT);
      expect(css, `${file} styles the brand as an h1`).not.toMatch(/\.brand\s+h1/);
      if (base.has(file)) continue;
      for (const [, selector = "", declarations = ""] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
        if (!bareH1.test(selector)) continue;
        expect(declarations, `${file}: ${selector.trim()}`).not.toMatch(/(^|[;\s])font(-size)?\s*:/);
      }
    }
  });

  it("R3223000 page-title.css sets the title and the section heading on one face and one step of the scale", () => {
    const css = code("routes/page-title.css", CSS_COMMENT);
    for (const selector of [":is(.app-shell, .screen-nav) .page-title", ":is(.app-shell, .screen-nav) .section-heading"]) {
      const rule = block(css, selector);
      expect(rule, selector).toContain("font-family: var(--font-display);");
      expect(rule, selector).toMatch(/font-size: var\(--type-[a-z0-9]+\);/);
    }
    expect(block(css, ":is(.app-shell, .screen-nav) .page-title")).toContain("font-size: var(--type-3xl);");
    expect(block(css, ":is(.app-shell, .screen-nav) .section-heading")).toContain("font-size: var(--type-2xl);");
  });

  it("R3223000 a board's rail keeps the title beside Back on one row, so its phone height budget holds", () => {
    // The replay player hands its BackLink, with a title, to the board's rail (--board-rail-phone-h: one 44px row).
    // top-bar.css's phone rule would give the title a row under Back; the rail overrides it.
    const css = code("routes/top-bar.css", CSS_COMMENT);
    const line = block(css, ".board-rail .screen-nav__line");
    expect(line).toContain("order: 0;");
    expect(line).toContain("flex: 1 1 0;");
    expect(line).toContain("flex-wrap: nowrap;");
    const title = block(css, ".board-rail .screen-nav .page-title");
    expect(title).toContain("white-space: nowrap;");
    expect(title).toMatch(/font-size: var\(--type-[a-z0-9]+\);/);
  });
});
