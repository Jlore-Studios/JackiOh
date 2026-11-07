// #3329: the face's cost gem is a hexagon (cards.css's `.cf .cost-gem`), so it draws no box shadow of
// its own. board.css's round `.cost-gem` has one (`--shadow-2`); on the face's square, radius-0 box it
// showed as a faint square behind the crystal. The hexagon's `::before` keeps its `drop-shadow`, which
// follows the outline. Vitest stubs CSS imports and jsdom paints nothing, so this reads the sheets as
// text (borders.test.ts does the same). Presentation only: no ruling, so no R number.

import { describe, expect, it } from "vitest";

import { readSheet } from "../test/setup.ts";

/** Every sheet of the face and its marks that could style the gem. */
const FACE_SHEETS = [
  "cards.css",
  "cosmetics.css",
  "glitch.css",
  "stamps.css",
  "cardstate.css",
  "marks.css",
  "keywords.css",
  "dormant.css",
  "tallies.css",
  "setmark.css",
  "inspect/inspect.css",
] as const;

/** A sheet's rules (at any depth) as selector and body, comments stripped so a comment naming a property proves nothing. */
function rules(file: string): { selector: string; body: string }[] {
  const css = readSheet(`src/cards/${file}`).replace(/\/\*[\s\S]*?\*\//g, "");
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({ selector: (match[1] ?? "").trim(), body: match[2] ?? "" }));
}

/** The value of `property` in a rule's body, or null when it declares none. */
function declared(body: string, property: string): string | null {
  const match = new RegExp(`(?:^|[;\\s])${property}\\s*:\\s*([^;]+)`).exec(body);
  return match === null ? null : (match[1] ?? "").trim();
}

describe("#3329 the face's cost gem shadow", () => {
  it("#3329 .cf .cost-gem resets box-shadow, so board.css's round gem shadow is not drawn round its square box", () => {
    const gem = rules("cards.css").filter((rule) => rule.selector === ".cf .cost-gem");
    expect(gem, "the face's one gem rule").toHaveLength(1);
    expect(declared(gem[0]?.body ?? "", "box-shadow")).toBe("none");
  });

  it("#3329 no rule of the face's sheets puts a box shadow back on the gem's box", () => {
    let seen = 0;
    for (const file of FACE_SHEETS) {
      for (const rule of rules(file)) {
        for (const part of rule.selector.split(",")) {
          // The gem's own box: `.cost-gem`, with attribute selectors at most (not its `::before`).
          if (!/\.cost-gem(\[[^\]]*\])*$/.test(part.trim())) continue;
          seen += 1;
          const shadow = declared(rule.body, "box-shadow");
          expect(shadow === null || shadow === "none", `${file}: ${part.trim()} sets box-shadow: ${shadow ?? ""}`).toBe(true);
        }
      }
    }
    expect(seen, "the sweep found the gem's rules").toBeGreaterThanOrEqual(8);
  });

  it("#3329 the hexagon keeps its own drop shadow, which follows its outline", () => {
    const hex = rules("cards.css").filter((rule) => rule.selector === ".cf .cost-gem::before");
    expect(hex, "the hexagon's one rule").toHaveLength(1);
    expect(declared(hex[0]?.body ?? "", "clip-path")).toContain("polygon(");
    expect(declared(hex[0]?.body ?? "", "filter")).toContain("drop-shadow(");
  });
});
