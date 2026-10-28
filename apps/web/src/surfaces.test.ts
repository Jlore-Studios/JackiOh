// #2109: the board's corners, stacking and neutral shadows come from one named surface scale,
// surfaces.css, and no stylesheet gains a radius, a shadow or a layer of its own unless this file
// says so. Gradients stay only where they mean something: card faces (src/cards/), the main
// button (--primary-face*), the table (game/table.css) and the named texture keeps below.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

/** The surface scale's sizes: a new radius, shadow or layer raises its number in the same change. */
const SURFACE_RADII = 7;
const SURFACE_SHADOWS = 4;
const SURFACE_LAYERS = 21;

function files(dir: string, ending: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : files(path, ending);
    return name.endsWith(ending) ? [path] : [];
  });
}

const SHEETS = files(SRC, ".css").map((path) => relative(SRC, path).replaceAll("\\", "/")).sort();
const read = (sheet: string): string => readFileSync(join(SRC, sheet), "utf8");
const codeOf = (sheet: string): string => read(sheet).replace(/\/\*[\s\S]*?\*\//g, " ");

/** The board's stylesheets, as palette.test.ts has them: index.css and game/'s, but the deck builder's and table.css. */
const BOARD_SHEETS = SHEETS.filter(
  (sheet) => sheet === "index.css" || (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

const RADII = [...read("surfaces.css").matchAll(/^\s*--radius-([\w-]+):\s*([\d.]+px|[\d.]+%);$/gm)].map((match) => [
  match[1] ?? "",
  match[2] ?? "",
]);
const SHADOWS = [...read("surfaces.css").matchAll(/^\s*--shadow-([\w-]+):\s*([^;]+);$/gm)].map((match) => [
  match[1] ?? "",
  (match[2] ?? "").trim().replace(/\s+/g, " "),
]);
const LAYERS = [...read("surfaces.css").matchAll(/^\s*--z-([a-z-]+):\s*(-?\d+);$/gm)].map((match) => [
  match[1] ?? "",
  Number(match[2]),
]);

/** A background gradient the board keeps: the locked/facedown hatch, the countered badge's strike
 * and bubbles, the drag reticle and the cell-pick crosshair. Everything else goes flat. */
const GRADIENT_KEEP = [
  /repeating-(linear|radial|conic)-gradient/, // the locked-zone and facedown hatch weaves
  /var\(--countered-bubble\)/, // R667's rising bubbles
  /var\(--drag-reticle-colour\)/, // the drop reticle and its tick bars
  /\/\s*(\d+px\s+100%|100%\s+\d+px)\s+no-repeat/, // R514's cell-pick crosshair bars
  /calc\(50% [-+] 1\.5px\)/, // the countered badge's struck-circle strike
];

/** A box-shadow layer that is a glow or a selection ring rather than a neutral shadow. */
const GLOW =
  /^(inset\s+)?0\s+0(\s+(\d+px|0)){1,2}\s+(var\(--(selected|legal|locked|mana|countered-rim|tap-tint|glow-ready|glow-condition|drag-reticle-colour)\)|color-mix\(|rgba?\()/;
/** The fuse spark's thrown particles (clock.css): offset dots in ember, not a shadow with a blur. */
const SPARK_PARTICLE = /^-?[\d.]+px -?[\d.]+px 0( -?[\d.]+(px)?)? var\(--clock-ember/;

/** Every `box-shadow` layer `css` states off the tokens, outside its comments. */
function offShadow(css: string): string[] {
  const found: string[] = [];
  for (const match of css.matchAll(/(?<![\w-])box-shadow\s*:\s*([^;]+)/g)) {
    const value = (match[1] ?? "").trim().replace(/\s+/g, " ");
    let depth = 0;
    let layer = "";
    const layers: string[] = [];
    for (const char of value) {
      if (char === "(") depth += 1;
      if (char === ")") depth -= 1;
      if (char === "," && depth === 0) {
        layers.push(layer);
        layer = "";
      } else {
        layer += char;
      }
    }
    layers.push(layer);
    for (const raw of layers) {
      const trimmed = raw.trim();
      if (trimmed === "" || trimmed === "none" || trimmed === "inherit") continue;
      if (trimmed.includes("var(--shadow-") || trimmed.includes("var(--primary-shadow")) continue;
      if (GLOW.test(trimmed) || SPARK_PARTICLE.test(trimmed)) continue;
      found.push(`box-shadow: ${trimmed}`);
    }
  }
  return found;
}

/** Every `border-radius`, `z-index` and `background` gradient `css` states off the tokens. */
function offSurface(css: string): string[] {
  const found: string[] = [];
  for (const match of css.matchAll(/(?<![\w-])(?:border(?:-top|-bottom)?-(?:left|right)-radius|border-radius)\s*:\s*([^;}]+)/g)) {
    const value = (match[1] ?? "").trim();
    const rest = value.replace(/var\(--radius-[\w-]+\)/g, "").trim();
    if (rest !== "inherit" && !/^[\s0/]*$/.test(rest)) found.push(`border-radius: ${value.replace(/\s+/g, " ")}`);
  }
  for (const match of css.matchAll(/(?<![\w-])z-index\s*:\s*([^;}]+)/g)) {
    const value = (match[1] ?? "").trim();
    if (value !== "auto" && !/^var\(--z-[\w-]+\)$/.test(value)) found.push(`z-index: ${value}`);
  }
  for (const match of css.matchAll(/(?<![\w-])background(?:-image)?\s*:\s*([^;]+)/g)) {
    const value = (match[1] ?? "").trim();
    if (value.includes("gradient(") && !GRADIENT_KEEP.some((keep) => keep.test(value)))
      found.push(`background: ${value.replace(/\s+/g, " ").slice(0, 120)}`);
  }
  return found;
}

describe("#2109 the surface scale", () => {
  it("#2109 surfaces.css names 7 radii, 4 shadows and 21 layers in order, and nothing else", () => {
    expect(RADII).toEqual([
      ["xs", "4px"],
      ["sm", "8px"],
      ["md", "12px"],
      ["lg", "16px"],
      ["xl", "22px"],
      ["pill", "999px"],
      ["round", "50%"],
    ]);
    expect(RADII.length).toBe(SURFACE_RADII);
    expect(SHADOWS.map((shadow) => shadow[0])).toEqual(["1", "2", "3", "inset-top"]);
    expect(SHADOWS.length).toBe(SURFACE_SHADOWS);
    const blurs = SHADOWS.slice(0, 3).map((shadow) => Number(/^\s*0 (\d+)px/.exec(shadow[1] ?? "")?.[1]));
    expect(blurs).toEqual([...blurs].sort((a, b) => a - b));
    expect(LAYERS).toEqual([
      ["below", -1],
      ["base", 0],
      ["content", 1],
      ["raised", 2],
      ["overlay", 3],
      ["hover", 4],
      ["popover", 5],
      ["overflow", 18],
      ["strip", 20],
      ["veil", 30],
      ["banner", 35],
      ["market", 36],
      ["clock", 37],
      ["showcase", 39],
      ["modal", 40],
      ["result", 45],
      ["spotlight", 50],
      ["aim", 59],
      ["drag", 60],
      ["toast", 70],
      ["toast-top", 72],
    ]);
    expect(LAYERS.length).toBe(SURFACE_LAYERS);
    const text = read("surfaces.css").replace(/\/\*[\s\S]*?\*\//g, "");
    const rest = text
      .replace(/^\s*--radius-[\w-]+:\s*[\d.]+(?:px|%);$/gm, "")
      .replace(/^\s*--shadow-[\w-]+:\s*[^;]+;$/gm, "")
      .replace(/^\s*--z-[a-z-]+:\s*-?\d+;$/gm, "");
    expect(rest.replace(/\s+/g, "")).toBe(":root{}");
  });

  it("#2109 index.css imports surfaces.css fourth and freezes the old aliases", () => {
    expect(read("index.css")).toMatch(
      /^@import "\.\/fonts\.css";\n@import "\.\/palette\.css";\n@import "\.\/typescale\.css";\n@import "\.\/surfaces\.css";\n/,
    );
    expect(read("index.css")).toMatch(/--radius: 7px;/);
    expect(read("game/board.css")).toMatch(/--seat-radius: var\(--radius-md\);/);
  });

  it("#2109 every var(--radius-*/--shadow-*/--z-*) names a token, and only surfaces.css declares one", () => {
    const names = new Set([
      ...RADII.map((radius) => `--radius-${radius[0]}`),
      ...SHADOWS.map((shadow) => `--shadow-${shadow[0]}`),
      ...LAYERS.map((layer) => `--z-${layer[0]}`),
    ]);
    // Outside board scope, and frozen there: the inspect overlay (src/cards/, which this
    // change must not touch) named its own two layers before the scale existed. No sheet
    // gains a name of its own: a sheet that loses its names empties its entry here.
    const FOREIGN: Record<string, string[]> = {
      "cards/inspect/inspect.css": ["--z-inspect-hover", "--z-inspect-modal"],
    };
    for (const sheet of SHEETS) {
      const css = codeOf(sheet);
      const allowed = new Set([...names, ...(FOREIGN[sheet] ?? [])]);
      for (const match of css.matchAll(/var\((--(?:radius|shadow|z)-[\w-]+)\)/g))
        expect(allowed.has(match[1] ?? ""), `${sheet}: ${match[0]}`).toBe(true);
      const declared = [...css.matchAll(/--(?:radius|shadow|z)-[\w-]+\s*:/g)].map((match) => match[0].replace(/:\s*$/, ""));
      if (sheet === "surfaces.css") {
        expect(new Set(declared)).toEqual(names);
      } else if (sheet in FOREIGN) {
        expect(declared.sort()).toEqual([...(FOREIGN[sheet] ?? [])].sort());
      } else {
        expect(declared, sheet).toEqual([]);
      }
    }
  });
});

describe("#2109 no one-off surfaces", () => {
  it("#2109 the board's stylesheets state no radius, shadow, layer or background gradient of their own", () => {
    expect(BOARD_SHEETS.length).toBeGreaterThan(20);
    const found = BOARD_SHEETS.flatMap((sheet) =>
      [...offSurface(codeOf(sheet)), ...offShadow(codeOf(sheet))].map((declaration) => `${sheet}: ${declaration}`),
    );
    expect(found, "name a radius, a shadow and a layer (surfaces.css) instead").toEqual([]);
    for (const sheet of BOARD_SHEETS) {
      expect(codeOf(sheet), sheet).not.toMatch(/var\(--radius\)/);
      expect(codeOf(sheet), sheet).not.toMatch(/var\(--seat-radius\)/);
    }
  });

  it("#2109 the board's components set none inline", () => {
    const components = files(join(SRC, "game"), ".tsx").filter((path) => !path.endsWith(".test.tsx"));
    expect(components.length).toBeGreaterThan(20);
    for (const path of components)
      expect(readFileSync(path, "utf8"), relative(SRC, path)).not.toMatch(/\b(?:borderRadius|boxShadow|zIndex)\b|gradient\(/);
  });
});
