// #3608, R3608000: a loop that never ends moves only `transform` and `opacity` (and the individual
// `translate`, `scale` and `rotate` that compose with them), so the compositor runs it and the browser
// repaints nothing per frame. A keyframe that sets a paint property (a shadow, a filter, a gradient's
// position, a dash offset, a clip, a text shadow) makes the browser repaint the element every frame, and
// a loop never stops doing it. The sheets are read as text, the way motion.test.ts and fx/css.test.ts
// read them: every stylesheet under src/, every `@keyframes`, every `animation` that repeats forever.
//
// What it cannot see: an `animation-iteration-count` set in another rule than its `animation-name`, and
// an animation set from a style attribute. The sheets set both in one declaration block.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

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

/** The properties whose animation repaints the element each frame. */
const PAINT_PROPS: readonly string[] = [
  "box-shadow",
  "filter",
  "backdrop-filter",
  "background-position",
  "background-position-x",
  "background-position-y",
  "stroke-dashoffset",
  "clip-path",
  "text-shadow",
];

/** A loop that must keep a paint property, by its keyframes' name, with the reason it must. Empty: every loop has a compositor form. */
const ALLOWED_PAINT_LOOPS: Record<string, string> = {};

/** `value` split at its top-level commas. */
function layers(value: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let layer = "";
  for (const char of value) {
    if (char === "(") depth += 1;
    if (char === ")") depth -= 1;
    if (char === "," && depth === 0) {
      out.push(layer.trim());
      layer = "";
    } else {
      layer += char;
    }
  }
  out.push(layer.trim());
  return out;
}

/** `value` split at its top-level semicolons: a `url(data:…;…)` keeps its own. */
function declarations(body: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let decl = "";
  for (const char of body) {
    if (char === "(") depth += 1;
    if (char === ")") depth -= 1;
    if (char === ";" && depth === 0) {
      out.push(decl);
      decl = "";
    } else {
      decl += char;
    }
  }
  out.push(decl);
  return out;
}

/** The name and the body of every `@keyframes`, its braces matched. */
function keyframesOf(css: string): { name: string; body: string }[] {
  const found: { name: string; body: string }[] = [];
  for (const head of css.matchAll(/@(?:-webkit-)?keyframes\s+([\w-]+)\s*\{/g)) {
    const start = (head.index ?? 0) + head[0].length;
    let depth = 1;
    let end = start;
    while (end < css.length && depth > 0) {
      if (css[end] === "{") depth += 1;
      if (css[end] === "}") depth -= 1;
      end += 1;
    }
    found.push({ name: head[1] ?? "", body: css.slice(start, end - 1) });
  }
  return found;
}

/** Every property a keyframes' stops set, custom properties left out. */
function propsOf(body: string): Set<string> {
  const props = new Set<string>();
  for (const stop of body.matchAll(/\{([^{}]*)\}/g))
    for (const decl of declarations(stop[1] ?? "")) {
      const name = decl.slice(0, Math.max(decl.indexOf(":"), 0)).trim().toLowerCase();
      if (/^[a-z][a-z-]*$/.test(name)) props.add(name);
    }
  return props;
}

/** `layer` without its function calls: what is left are its words, and one of them is the keyframes' name. */
function wordsOf(layer: string): string[] {
  let bare = layer;
  for (let before = ""; before !== bare; ) {
    before = bare;
    bare = bare.replace(/\([^()]*\)/g, " ");
  }
  return bare.match(/[A-Za-z_][\w-]*/g) ?? [];
}

/** Every word in the sheet's `animation` layers and `animation-name` lists that repeats forever, among them every keyframes' name that does. */
function infiniteWords(css: string): Set<string> {
  const words = new Set<string>();
  for (const rule of css.matchAll(/\{([^{}]*)\}/g)) {
    let names: string[] = [];
    let counts: string[] = [];
    for (const decl of declarations(rule[1] ?? "")) {
      const colon = decl.indexOf(":");
      if (colon < 0) continue;
      const prop = decl.slice(0, colon).trim().toLowerCase();
      const value = decl.slice(colon + 1).replace(/!important/g, "");
      if (prop === "animation")
        for (const layer of layers(value)) if (/(?<![\w-])infinite(?![\w-])/.test(layer)) for (const word of wordsOf(layer)) words.add(word);
      if (prop === "animation-name") names = layers(value);
      if (prop === "animation-iteration-count") counts = layers(value).map((count) => count.trim().toLowerCase());
    }
    if (counts.length > 0)
      names.forEach((name, index) => {
        if (counts[index % counts.length] === "infinite") words.add(name.trim());
      });
  }
  return words;
}

/** The keyframes in `css` that loop forever. */
function infiniteLoops(css: string): string[] {
  const infinite = infiniteWords(css);
  return [...new Set(keyframesOf(css).map((frames) => frames.name))].filter((name) => infinite.has(name)).sort();
}

/** `"<keyframes>: <property>"` for every loop in `css` that sets a paint property and is no entry of `allowed`. */
function paintLoops(css: string, allowed: Record<string, string> = ALLOWED_PAINT_LOOPS): string[] {
  const code = css.replace(/\/\*[\s\S]*?\*\//g, " ");
  const loops = new Set(infiniteLoops(code));
  const found = new Set<string>();
  for (const frames of keyframesOf(code)) {
    if (!loops.has(frames.name) || Object.hasOwn(allowed, frames.name)) continue;
    for (const prop of propsOf(frames.body)) if (PAINT_PROPS.includes(prop)) found.add(`${frames.name}: ${prop}`);
  }
  return [...found].sort();
}

/** Every stylesheet's code, in one text: a keyframes' name is global, so a sheet may loop what another one declares. */
const ALL_CSS = SHEETS.map(codeOf).join("\n");

describe("R3608000 an infinite animation moves only what the compositor moves", () => {
  it("R3608000 the guard catches an infinite shorthand loop over each paint property", () => {
    for (const prop of PAINT_PROPS) {
      const css = `@keyframes spin { from { ${prop}: none; } to { ${prop}: none; } } .x { animation: spin 1s linear infinite alternate; }`;
      expect(paintLoops(css, {}), prop).toEqual([`spin: ${prop}`]);
    }
  });

  it("R3608000 the guard catches a longhand infinite loop, and a loop whose name comes after the count", () => {
    const frames = "@keyframes pulse { 50% { box-shadow: 0 0 4px red; } }";
    expect(paintLoops(`${frames} .x { animation-name: pulse; animation-duration: 1s; animation-iteration-count: infinite; }`, {})).toEqual([
      "pulse: box-shadow",
    ]);
    expect(paintLoops(`${frames} .x { animation: 1s infinite var(--ease-in-out) pulse; }`, {})).toEqual(["pulse: box-shadow"]);
    expect(paintLoops(`${frames} .x { animation-iteration-count: infinite; animation-name: other, pulse; }`, {})).toEqual(["pulse: box-shadow"]);
  });

  it("R3608000 the guard reads a loop across layers and keyframes of other sheets, and a vendor prefix", () => {
    const frames = "@keyframes a { to { box-shadow: none; } } @keyframes b { to { opacity: 0; } }";
    expect(paintLoops(`${frames} .x { animation: b 1s infinite, a 1s 2; }`, {})).toEqual([]);
    expect(paintLoops(`${frames} .x { animation: a 1s 2, b 1s infinite; }`, {})).toEqual([]);
    expect(paintLoops(`${frames} .x { animation: a 1s 2, b 1s infinite, a calc(2s * var(--s, 1)) linear infinite; }`, {})).toEqual(["a: box-shadow"]);
    expect(paintLoops(`${frames} .x { animation-name: a, b; animation-iteration-count: 2, infinite; }`, {})).toEqual([]);
    expect(paintLoops(`${frames} .x { animation-name: a, b; animation-iteration-count: infinite, 2; }`, {})).toEqual(["a: box-shadow"]);
    expect(paintLoops(`${frames} .x { animation-name: a, b; animation-iteration-count: infinite; }`, {})).toEqual(["a: box-shadow"]);
    expect(paintLoops("@-webkit-keyframes w { to { filter: blur(1px); } } .x { animation: w 1s infinite; }", {})).toEqual(["w: filter"]);
    expect(paintLoops("@keyframes w { to { filter: blur(1px); } } @media (min-width: 1px) { .x:hover { animation: w 1s infinite; } }", {})).toEqual(["w: filter"]);
  });

  it("R3608000 the guard lets pass what is not a loop that paints", () => {
    const paint = "@keyframes a { to { box-shadow: none; filter: none; background-position: 1px 0; stroke-dashoffset: 4; clip-path: none; text-shadow: none; } }";
    expect(paintLoops(`${paint} .x { animation: a 1s linear 3; }`, {})).toEqual([]);
    expect(paintLoops(`${paint} .x { animation: a 1s linear both; }`, {})).toEqual([]);
    expect(paintLoops(`${paint} .x { animation-name: a; animation-iteration-count: 2; }`, {})).toEqual([]);
    expect(paintLoops(`${paint} .x { animation-name: a; }`, {})).toEqual([]);
    expect(paintLoops(`${paint} .x { transition: box-shadow 1s infinite; }`, {})).toEqual([]);
    const composed =
      "@keyframes c { from { transform: translateX(0); opacity: 0; scale: 1; translate: 0 0; rotate: 0deg; --glow: 1px; } to { transform: translateX(9px); opacity: 1; scale: 2; translate: 1px 0; rotate: 9deg; --glow: 2px; } } .x { animation: c 1s infinite; }";
    expect(infiniteLoops(composed)).toEqual(["c"]);
    expect(paintLoops(composed, {})).toEqual([]);
    expect(paintLoops(`/* @keyframes q { to { filter: none; } } .y { animation: q 1s infinite; } */ ${composed}`, {})).toEqual([]);
  });

  it("R3608000 a loop on the allow-list is skipped, and only that one", () => {
    const css =
      "@keyframes a { to { box-shadow: none; } } @keyframes b { to { stroke-dashoffset: 9; } } .x { animation: a 1s infinite; } .y { animation: b 1s infinite; }";
    expect(paintLoops(css, {})).toEqual(["a: box-shadow", "b: stroke-dashoffset"]);
    expect(paintLoops(css, { a: "one path, no compositor form" })).toEqual(["b: stroke-dashoffset"]);
    expect(paintLoops(css, { a: "x", b: "y" })).toEqual([]);
  });

  it("R3608000 no stylesheet loops a paint property outside the allow-list", () => {
    expect(SHEETS.length).toBeGreaterThan(40);
    // A parser that stopped reading would find none: the board's own loops are a floor.
    const loops = infiniteLoops(ALL_CSS);
    expect(loops.length).toBeGreaterThan(50);
    for (const name of [
      "glow-breathe-ready",
      "glow-breathe-condition",
      "jk-facedown-breathe",
      "jk-veil-drift",
      "cf-mythic-hand-shimmer",
      "end-turn-ready-pulse",
      "clock-pill-pulse",
      "lane-pick-breathe",
      "duos-portal-flash",
      "coach-pulse",
      "skeleton-shimmer",
      "drag-arrow-march",
      "opponent-aim-run",
    ])
      expect(loops, name).toContain(name);
    expect(paintLoops(ALL_CSS), "move transform and opacity instead, or name the loop in ALLOWED_PAINT_LOOPS with its reason").toEqual([]);
  });

  it("R3608000 every allowed loop has its reason and still loops a paint property", () => {
    const standing = paintLoops(ALL_CSS, {}).map((found) => found.split(":")[0]);
    for (const [name, reason] of Object.entries(ALLOWED_PAINT_LOOPS)) {
      expect(reason.trim().length, name).toBeGreaterThan(0);
      expect(standing, `${name} no longer loops a paint property: take it off the allow-list`).toContain(name);
    }
  });
});
