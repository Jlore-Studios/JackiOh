// R1295000: a border is one object on the full face and on the board minion. The full face draws it
// as its edge, `--cf-edge` (cards.css paints the root with it, cosmetics.css sets it per border: a
// gradient, Wood's grain, Circuit's traces). The minion's panel covers the root, so its border is a
// ring layer over the panel painted with that same `--cf-edge`, not a flat rim in one trim colour.
// The test reads the sheets' text, since Vitest stubs CSS imports and jsdom paints nothing; the style
// engine's side of it, the ring's computed background against the face's, is measured in
// e2e/cypress/component/card-cosmetics.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));

/** A stylesheet as text, comments stripped, so a comment naming a property proves nothing. */
function sheet(name: string): string {
  return readFileSync(join(HERE, name), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

const cosmeticsCss = sheet("cosmetics.css");
const cardsCss = sheet("cards.css");

/** The balanced `{ … }` block of `css` opened at `open`, inner text without the braces. */
function blockFrom(css: string, open: number): string {
  let depth = 0;
  for (let i = open; i < css.length; i += 1) {
    if (css[i] === "{") depth += 1;
    else if (css[i] === "}") {
      depth -= 1;
      if (depth === 0) return css.slice(open + 1, i);
    }
  }
  throw new Error("unbalanced braces");
}

/** Every rule of `css` (at any depth) as its selector and body. */
function rules(css: string): { selector: string; body: string }[] {
  const out: { selector: string; body: string }[] = [];
  let start = 0;
  for (let i = 0; i < css.length; i += 1) {
    if (css[i] === "{") {
      out.push({ selector: css.slice(start, i).trim(), body: blockFrom(css, i) });
      start = i + 1;
    } else if (css[i] === "}" || css[i] === ";") start = i + 1;
  }
  return out;
}

/** The value of `property` in a rule's body, or null when it declares none. */
function declared(body: string, property: string): string | null {
  const match = new RegExp(`(?:^|[;\\s])${property}\\s*:\\s*([^;]+)`).exec(body);
  return match === null ? null : (match[1] ?? "").trim();
}

/** The cosmetics' rules for a minion wearing a border, whichever of its parts they draw. */
const minionBorder = rules(cosmeticsCss).filter((rule) => rule.selector.includes('[data-layout="minion"][data-frame]'));

describe("R1295000 the board minion's border", () => {
  it("R1295000 the minion's border rule paints the face's edge, var(--cf-edge)", () => {
    expect(minionBorder.length).toBeGreaterThan(0);
    const painted = minionBorder.filter((rule) => (declared(rule.body, "background-image") ?? "") === "var(--cf-edge)");
    expect(painted.map((rule) => rule.selector), "a rule painting the edge").toHaveLength(1);

    // The ring has the edge's colours and nothing else's: it names no trim colour and sets no edge.
    const ring = painted[0]?.body ?? "";
    expect(ring).not.toContain("--trim-");
    expect(ring).not.toContain("--cf-edge:");
    // It is a ring over the panel, so a mask cuts the middle out of the layer.
    expect(declared(ring, "mask")).not.toBeNull();
    expect(declared(ring, "mask-composite")).toBe("exclude");
    expect(declared(ring, "pointer-events")).toBe("none");
  });

  it("R1295000 no minion border rule draws a single-colour inset rim in a trim colour", () => {
    for (const rule of minionBorder) {
      for (const property of ["box-shadow", "outline", "border-color"]) {
        const value = declared(rule.body, property) ?? "";
        expect(value, `${rule.selector}'s ${property}`).not.toMatch(/var\(--trim-\d\)/);
      }
      expect(rule.body, rule.selector).not.toMatch(/inset[^;]*var\(--trim-/);
    }
  });

  it("R1295000 the full face and every border share one edge: the root paints it, the sheet sets it once per border", () => {
    const root = rules(cardsCss).find((rule) => rule.selector === ".cf");
    expect(declared(root?.body ?? "", "background")).toBe("var(--cf-edge)");

    const edges = rules(cosmeticsCss).filter((rule) => declared(rule.body, "--cf-edge") !== null);
    expect(edges.length).toBeGreaterThan(0);
    for (const rule of edges) expect(rule.selector, "an edge applies to the face's root").toMatch(/^\.cf\[data-frame/);
  });
});
