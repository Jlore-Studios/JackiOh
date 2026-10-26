// #1954: the glove cursor. Every former `pointer`, `grab` and `grabbing` names one of the
// three custom properties, defined once in index.css under a fine pointer; touch and pen fall
// back to their own keyword.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));
const CURSORS = join(SRC, "..", "public", "cursors");

const GLOVES = [
  { property: "--cursor-glove", file: "glove.svg", x: 4, y: 3, keyword: "auto" },
  { property: "--cursor-glove-point", file: "glove-point.svg", x: 4, y: 3, keyword: "pointer" },
  { property: "--cursor-glove-grab", file: "glove-grab.svg", x: 10, y: 9, keyword: "grabbing" },
] as const;

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

describe("glove cursors", () => {
  it("#1954 index.css defines each glove once, under a fine pointer", () => {
    const css = readFileSync(join(SRC, "index.css"), "utf8");
    // The media block runs up to the `html` rule that follows it.
    const body = css.slice(css.indexOf("@media (pointer: fine)"), css.indexOf("html {"));
    expect(body).not.toBe("");
    for (const glove of GLOVES) {
      const definition = `${glove.property}: url("/cursors/${glove.file}") ${String(glove.x)} ${String(glove.y)}, ${glove.keyword};`;
      expect(body, glove.property).toContain(definition);
      expect(css.split(`${glove.property}:`).length - 1, glove.property).toBe(1);
    }
    expect(css).toContain("html {\n  cursor: var(--cursor-glove, auto);\n}");
  });

  it("#1954 no stylesheet says cursor: pointer, grab or grabbing", () => {
    const files = cssFiles(SRC);
    expect(files.length).toBeGreaterThan(20);
    const bad: string[] = [];
    for (const path of files) {
      readFileSync(path, "utf8")
        .split("\n")
        .forEach((line, index) => {
          if (/\bcursor:\s*(pointer|grab|grabbing)\b/.test(line)) {
            bad.push(`${relative(SRC, path)}:${String(index + 1)}: ${line.trim()}`);
          }
        });
    }
    expect(bad).toEqual([]);
  });

  it("#1954 each glove is an SVG of at most 32x32 with its hotspot inside", () => {
    for (const glove of GLOVES) {
      const svg = readFileSync(join(CURSORS, glove.file), "utf8");
      const width = Number(/width="(\d+)"/.exec(svg)?.[1]);
      const height = Number(/height="(\d+)"/.exec(svg)?.[1]);
      expect(width, `${glove.file} parses a width`).toBeLessThanOrEqual(32);
      expect(height, `${glove.file} parses a height`).toBeLessThanOrEqual(32);
      expect(svg, `${glove.file} draws vectors, not a bitmap`).not.toContain("<image");
      expect(glove.x, `${glove.file} hotspot x`).toBeLessThan(width);
      expect(glove.y, `${glove.file} hotspot y`).toBeLessThan(height);
    }
  });
});
