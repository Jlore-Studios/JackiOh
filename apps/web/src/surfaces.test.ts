// #2109: the board's corners, stacking and neutral shadows come from one named surface scale, surfaces.css;
// no stylesheet gains a radius, a shadow or a layer of its own unless this file says so. Gradients stay only
// on card faces (src/cards/), the main button (--primary-face*), the table (game/table.css and board.css's
// `.field`) and the named texture keeps below. Every other stylesheet, the menus' included, is held to its
// count of them in UNTOKENED_SURFACES (#3224).

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

/** The board's stylesheets, as palette.test.ts has them. */
const BOARD_SHEETS = SHEETS.filter(
  (sheet) =>
    sheet === "index.css" ||
    sheet === "buttons.css" ||
    (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

/** Every other sheet's own radii, shadows, layers and gradients, counted as the board's test finds them (card faces, the table and fx keep their gradients by design): one that moves values onto surfaces.css lowers its number, and none gains one. */
const UNTOKENED_SURFACES: Record<string, number> = {
  "audio/audio.css": 4,
  "auth/auth.css": 1,
  "auth/code-field.css": 23,
  "auth/consent.css": 3,
  "auth/tavern.css": 27,
  "auth/two-step.css": 1,
  "cards/art/art.css": 7,
  "cards/cards.css": 162,
  "cards/cardstate.css": 24,
  "cards/cosmetics.css": 38,
  "cards/dormant.css": 7,
  "cards/glitch.css": 9,
  "cards/inspect/inspect.css": 36,
  "cards/inspect/list-play.css": 2,
  "cards/keywords.css": 19,
  "cards/marks.css": 7,
  "cards/setmark.css": 6,
  "cards/stamps.css": 1,
  "cards/wheel/wheel.css": 6,
  "collection/collection.css": 27,
  "collection/cosmeticLook.css": 1,
  "collection/tokenOffer.css": 1,
  "duos/duos.css": 24,
  "economy/economy.css": 8,
  "economy/jlore-pill.css": 4,
  "economy/spend.css": 1,
  "emotes/emotes.css": 17,
  "emotes/portrait.css": 20,
  "empty/empty.css": 1,
  "fx/aura/aura.css": 3,
  "fx/fx.css": 106,
  "game/deckbuilder/deckbuilder.css": 66,
  "game/deckbuilder/workshop.css": 30,
  "game/table.css": 44,
  "notfound/notfound.css": 2,
  "packs/pack-object.css": 12,
  "packs/packs.css": 7,
  "patches/patches.css": 25,
  "practice/practice.css": 64,
  "practice/sandbox.css": 7,
  "rank/rankCrest.css": 1,
  "routes/account.css": 1,
  "routes/command-menu.css": 3,
  "routes/dev-panel.css": 5,
  "routes/entry-card.css": 3,
  "routes/game-banner.css": 7,
  "routes/landing.css": 61,
  "routes/lobby.css": 37,
  "routes/replays.css": 1,
  "routes/site-footer.css": 1,
  "routes/stats.css": 15,
  "routes/top-bar.css": 3,
  "settings/changeEmail.css": 2,
  "settings/settings.css": 25,
  "shop/shop.css": 2,
  "skeleton/skeleton.css": 4,
  "stats/tier.css": 1,
  "together/together.css": 16,
  "tutorial/path-visibility.css": 2,
  "tutorial/tutorial.css": 44,
  "undo/undo.css": 3,
};

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

/** The background gradients the board keeps. Everything else goes flat. */
const GRADIENT_KEEP = [
  /^linear-gradient\(\s*to top,\s*var\(--bg-sunken\) 0%,\s*var\(--bg-sunken\) 36%,/, // B45's burn front: its sunken lower part eats the card as it climbs
  /repeating-(linear|radial|conic)-gradient/, // the locked-zone and facedown hatch weaves
  /var\(--countered-bubble\)/, // R667's rising bubbles
  /var\(--drag-reticle-color\)/, // the drop reticle and its tick bars
  /\/\s*(\d+px\s+100%|100%\s+\d+px)\s+no-repeat/, // R514's cell-pick crosshair bars
  /calc\(50% [-+] 1\.5px\)/, // the countered badge's struck-circle strike
];

/** The playmat and its marks: the tavern table is board.css's `.field`, and each playmat's tap marks and
 * sand cracks are its own cosmetic (SandSurface's TAP_FAMILY); these rules keep their gradients and shadows. */
const TABLE_RULE = /^\.(field|sand-mark(--\w+)?|tap-mark--\w+)$/;

function withoutTable(css: string): string {
  return css.replace(/([^{}]+)\{[^{}]*\}/g, (rule, selector: string) => {
    const parts = selector.split(",").map((part) => part.trim());
    return parts.every((part) => TABLE_RULE.test(part)) ? "" : rule;
  });
}

/** A box-shadow layer that is a glow or a selection ring rather than a neutral shadow. */
const GLOW =
  /^(inset\s+)?0\s+0(\s+(\d+px|0)){1,2}\s+(var\(--(selected|legal|locked|mana|countered-rim|tap-tint|glow-ready|glow-condition|drag-reticle-color)\)|color-mix\(|rgba?\()/;
/** The fuse spark's thrown particles (clock.css): offset dots in ember, not a shadow with a blur. */
const SPARK_PARTICLE = /^-?[\d.]+px -?[\d.]+px 0( -?[\d.]+(px)?)? var\(--clock-ember/;

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
      ["dialog", 72],
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
    // Outside board scope, and frozen: the inspect overlay's (src/cards/) own hover layer; R3221000 put its
    // modal one on --z-dialog. No sheet gains a name of its own; one that loses its names empties its entry.
    const FOREIGN: Record<string, string[]> = {
      "cards/inspect/inspect.css": ["--z-inspect-hover"],
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
      [...offSurface(withoutTable(codeOf(sheet))), ...offShadow(withoutTable(codeOf(sheet)))].map((declaration) => `${sheet}: ${declaration}`),
    );
    expect(found, "name a radius, a shadow and a layer (surfaces.css) instead").toEqual([]);
    for (const sheet of BOARD_SHEETS) {
      expect(codeOf(sheet), sheet).not.toMatch(/var\(--radius\)/);
      expect(codeOf(sheet), sheet).not.toMatch(/var\(--seat-radius\)/);
    }
  });

  it("#2109 no other stylesheet gains a radius, a shadow, a layer or a gradient of its own", () => {
    const counts: Record<string, number> = {};
    for (const sheet of SHEETS) {
      if (BOARD_SHEETS.includes(sheet)) continue;
      const css = codeOf(sheet);
      const own = offSurface(css).length + offShadow(css).length;
      if (own > 0) counts[sheet] = own;
    }
    expect(counts, "name a radius, a shadow and a layer (surfaces.css) instead; a sheet that loses some lowers its number in UNTOKENED_SURFACES").toEqual(
      UNTOKENED_SURFACES,
    );
  });

  it("#2109 the board's components set none inline", () => {
    const components = files(join(SRC, "game"), ".tsx").filter((path) => !path.endsWith(".test.tsx"));
    expect(components.length).toBeGreaterThan(20);
    for (const path of components)
      expect(readFileSync(path, "utf8"), relative(SRC, path)).not.toMatch(/\b(?:borderRadius|boxShadow|zIndex)\b|gradient\(/);
  });
});
