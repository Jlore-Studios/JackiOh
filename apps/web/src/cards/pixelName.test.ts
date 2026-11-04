// R2942001: the Pixel version's name face (pixelName.ts): a 5×7 bitmap built in, every catalog name drawn in it but
// those with a character it lacks, as squares alone. The faces wearing it are versionLooks3.test.tsx's.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import {
  PIXEL_GLYPHS,
  PIXEL_GLYPH_COLS,
  PIXEL_GLYPH_GAP,
  PIXEL_GLYPH_ROWS,
  PIXEL_NAME_INK,
  PIXEL_NAME_SHADOW,
  PIXEL_NAME_SHADOW_OFFSET,
  pixelNameSvg,
  pixelNameUri,
} from "./pixelName.ts";

/** A CJK ideograph: the pixel face has none. */
const CJK = /[㐀-鿿]/u;

describe("R2942001 the pixel name face", () => {
  it("R2942001 every glyph is 7 rows of 5, and the face holds every letter and digit", () => {
    for (const [char, rows] of Object.entries(PIXEL_GLYPHS)) {
      expect(rows, char).toHaveLength(PIXEL_GLYPH_ROWS);
      for (const row of rows) expect(row, char).toMatch(new RegExp(`^[01]{${String(PIXEL_GLYPH_COLS)}}$`));
    }
    for (const char of "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ") expect(PIXEL_GLYPHS[char], char).toBeDefined();
    // No two characters share a glyph but the space, which lights nothing.
    const drawn = Object.entries(PIXEL_GLYPHS).map(([, rows]) => rows.join("/"));
    expect(new Set(drawn).size).toBe(drawn.length);
    expect(PIXEL_GLYPHS[" "]?.join("")).toMatch(/^0+$/);
  });

  it("R2942001 every catalog name draws but the CJK ones, as squares alone, the same each time", () => {
    const names = [...new Set(Object.values(CATALOG).map((def) => def.name))];
    const plain = names.filter((name) => pixelNameSvg(name) === null);
    // Only the names with a Chinese character fall back to the normal face.
    expect(plain.length).toBeGreaterThan(0);
    for (const name of plain) expect(name, name).toMatch(CJK);
    for (const name of names.filter((name) => !CJK.test(name))) {
      const svg = pixelNameSvg(name);
      expect(svg, name).not.toBeNull();
      expect(svg, name).toBe(pixelNameSvg(name));
      for (const banned of ["<text", "<title", "href", "NaN", "undefined"]) expect(svg, `${name} ${banned}`).not.toContain(banned);
      expect(svg?.match(/<svg[\s>]/g) ?? [], name).toHaveLength(1);
    }
    expect(pixelNameSvg("")).toBeNull();
    expect(pixelNameUri("饕餮")).toBeNull();
  });

  it("R2942001 a name is its capitals' glyphs side by side, white over its black drop shadow", () => {
    const svg = pixelNameSvg("Hi!") ?? "";
    // Three glyphs, a cell's gap between, and the shadow's offset past the last.
    const width = 3 * PIXEL_GLYPH_COLS + 2 * PIXEL_GLYPH_GAP + PIXEL_NAME_SHADOW_OFFSET;
    expect(svg).toContain(`viewBox="0 0 ${String(width)} ${String(PIXEL_GLYPH_ROWS + PIXEL_NAME_SHADOW_OFFSET)}"`);
    expect(svg).toContain('shape-rendering="crispEdges"');
    expect(svg.indexOf(`fill="${PIXEL_NAME_SHADOW}"`)).toBeLessThan(svg.indexOf(`fill="${PIXEL_NAME_INK}"`));
    // Lower case reads as its capitals.
    expect(pixelNameSvg("hi!")).toBe(svg);
    // H's top row: a cell at each side, at the origin and four cells over.
    expect(svg).toContain(`<path fill="${PIXEL_NAME_INK}" d="M0 0h1v1h-1zM4 0h1v1h-1z`);
    expect(pixelNameUri("Hi!")).toBe(`data:image/svg+xml,${encodeURIComponent(svg)}`);
  });
});
