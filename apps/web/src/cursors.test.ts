// #1954: the glove cursor: every `pointer`, `grab` and `grabbing` is one of three custom properties
// defined once in index.css under a fine pointer; touch and pen keep their own keyword.

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
  { property: "--cursor-glove-press", file: "glove-press.svg", x: 4, y: 3, keyword: "pointer" },
] as const;

const PRESS = ":root:active {\n    --cursor-glove-point: var(--cursor-glove-press);\n  }";

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

function keyframes(css: string): string[] {
  const blocks: string[] = [];
  for (let at = css.indexOf("@keyframes"); at !== -1; at = css.indexOf("@keyframes", at + 1)) {
    let depth = 0;
    let end = css.indexOf("{", at);
    for (; end < css.length; end += 1) {
      if (css[end] === "{") depth += 1;
      if (css[end] === "}") depth -= 1;
      if (depth === 0) break;
    }
    blocks.push(css.slice(at, end + 1));
  }
  return blocks;
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
      // Set once each, but for the press swap (#2103), which the next tests pin.
      expect(css.replace(PRESS, "").split(`${glove.property}:`).length - 1, glove.property).toBe(1);
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

  it("#2103 while a button is held the pointing glove is the pressed one, under a fine pointer only", () => {
    const css = readFileSync(join(SRC, "index.css"), "utf8");
    const body = css.slice(css.indexOf("@media (pointer: fine)"), css.indexOf("html {"));
    expect(body).toContain(PRESS);
    expect(css.split(PRESS).length - 1).toBe(1);
    // The press reaches a stylesheet only through the swap: nothing else names it.
    const named = cssFiles(SRC).flatMap((path) => {
      const count = readFileSync(path, "utf8").split("--cursor-glove-press").length - 1;
      return count === 0 ? [] : [`${relative(SRC, path)}: ${String(count)}`];
    });
    expect(named).toEqual(["index.css: 2"]);
  });

  it("#2103 the pressed glove's fingertip is the pointing glove's hotspot", () => {
    const [, point, , press] = GLOVES;
    expect(press.file).toBe("glove-press.svg");
    expect([press.x, press.y]).toEqual([point.x, point.y]);
    const pointSvg = readFileSync(join(CURSORS, point.file), "utf8");
    const pressSvg = readFileSync(join(CURSORS, press.file), "utf8");
    // The fingertip is the top of the finger's arc, (14.4, 1.6) before the glove's turn. The press
    // turns the hand the same way and shrinks it about that point, so the tip keeps its pixel.
    const TURN = 'transform="translate(-2.5 -2) rotate(-32 16 17)';
    const TIP = "V4a2.4,2.4,0,0,1,4.8,0";
    expect(pointSvg).toContain(`${TURN}"`);
    expect(pressSvg).toContain(`${TURN} translate(14.4 1.6) scale(0.92) translate(-14.4 -1.6)"`);
    expect(pointSvg).toContain(`M12,15.5${TIP}V15.5"`);
    expect(pressSvg).toContain(`M12,13${TIP}V13"`);
  });

  it("#2103 the press is one still frame: no cursor file goes unchecked and no keyframes name a cursor", () => {
    expect(new Set(readdirSync(CURSORS))).toEqual(new Set(GLOVES.map((glove) => glove.file)));
    const animated: string[] = [];
    for (const path of cssFiles(SRC)) {
      for (const block of keyframes(readFileSync(path, "utf8"))) {
        if (block.includes("cursor")) animated.push(`${relative(SRC, path)}: ${block.slice(0, block.indexOf("{")).trim()}`);
      }
    }
    expect(animated).toEqual([]);
  });
});
