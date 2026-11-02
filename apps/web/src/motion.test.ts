// #2110: the board's motions come from motion.css's four durations and five curves; BUILD's event durations keep their numbers, and what else keeps its own time is in KEEP.

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

/** The board's stylesheets, as palette.test.ts has them. */
const BOARD_SHEETS = SHEETS.filter(
  (sheet) => sheet === "index.css" || (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

/** The vocabulary's sizes: a new duration or curve raises its number in the same change. */
const MOTION_DURATIONS = 4;
const MOTION_CURVES = 5;

const DURATIONS = [...read("motion.css").matchAll(/^\s*--motion-([\w-]+):\s*(\d+ms);$/gm)].map((match) => [match[1] ?? "", match[2] ?? ""]);
const CURVES = [...read("motion.css").matchAll(/^\s*--ease-([\w-]+):\s*(cubic-bezier\([^()]*\));$/gm)].map((match) => [match[1] ?? "", match[2] ?? ""]);

/** Every time a board sheet keeps of its own, in source order. */
const KEEP: Record<string, string[]> = {
  "game/activate.css": ["300ms"], // BUILD's `activated` row
  "game/aim/opponentAim.css": ["700ms", "900ms"], // loops
  "game/board.css": [
    "90ms", // R434's flip stagger
    ...["420ms", "1600ms", "260ms", "520ms", "2400ms", "700ms"], // the table's tap marks
    ...["250ms", "400ms", "220ms", "250ms", "320ms", "400ms"], // R701's wobbles and flashes
    "1.2s", // the End turn button's loop
    "150ms", // BUILD's `turnEnded` row
  ],
  // Clock.tsx's TICK_MS repaint (the 200ms) and loops.
  "game/clock.css": ["200ms", "1s", "500ms", "1s", "200ms", "200ms", "200ms", "140ms", "520ms", "90ms", "1.2s", "700ms"],
  "game/countered.css": ["2.6s", "4.1s"], // loops
  "game/drag/drag.css": ["0.7s", "0.8s"], // loops
  "game/facedown.css": ["9s", "4.8s"], // loops
  "game/highlights.css": ["1.8s", "1.4s"], // loops
  "game/market.css": ["3.2s"], // loop
  "game/notices.css": ["5s"], // the draw toast's hold
  "game/reveal.css": ["70ms"], // the deal's stagger
  "game/showcase/showcase.css": ["1.1s"], // loop
  "game/spent.css": ["2.4s"], // loop
};

/** How many durations in each board sheet scale by --anim-scale: none loses the factor. */
const ANIM_SCALED: Record<string, number> = {
  "game/activate.css": 1, "game/animations.css": 65, "game/board.css": 12, "game/drag/blocked.css": 1, "game/facedown.css": 2,
  "game/highlights.css": 2, "game/notices.css": 1, "game/prompt.css": 2, "game/reveal.css": 2, "game/spent.css": 1,
};

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

function decls(css: string, names: string): { name: string; value: string }[] {
  return [...css.matchAll(new RegExp(`(?<![\\w-])(${names})\\s*:\\s*([^;{}]+)`, "g"))].map((match) => ({
    name: match[1] ?? "",
    value: (match[2] ?? "").trim().replace(/\s+/g, " "),
  }));
}

const KEYFRAMES = /@(?:-webkit-)?keyframes\s+[\w-]+\s*\{(?:[^{}]*\{[^{}]*\})*[^{}]*\}/g;

/** A keyframe stop cannot read a var() (the browser falls back to `ease`), so it spells a curve's value out. */
function offCurve(css: string): string[] {
  const values = new Set(CURVES.map((curve) => curve[1]));
  const found: string[] = [];
  for (const block of css.match(KEYFRAMES) ?? [])
    for (const decl of decls(block, "animation-timing-function"))
      if (decl.value !== "linear" && !decl.value.startsWith("steps(") && !values.has(decl.value)) found.push(`@keyframes: ${decl.value}`);
  for (const decl of decls(css.replace(KEYFRAMES, ""), "transition|transition-timing-function|animation|animation-timing-function")) {
    for (const layer of layers(decl.value)) {
      if (/^(none|inherit)( !important)?$/.test(layer)) continue;
      const named = /var\(--ease-[\w-]+\)|(?<![\w-])linear(?![\w-])|steps\(/.test(layer);
      const own = /(?<![\w-])(ease(-in-out|-in|-out)?(?![\w-])|cubic-bezier\()/.test(layer.replace(/var\(--ease-[\w-]+\)/g, ""));
      if (!named || own) found.push(`${decl.name}: ${layer}`);
    }
  }
  return found;
}

/** BUILD's durations: the event table's, in animations.css (animations.ts, M5-T4). */
const BUILD_DURATION = /calc\(\d+ms \* var\(--anim-scale\) \* var\(--anim-squeeze, 1\)\)/g;

/** The non-zero times in `css` that are no token, not BUILD's and not fed in by a var(). */
function ownTimes(sheet: string, css: string): string[] {
  const found: string[] = [];
  for (const decl of decls(css, "transition|transition-duration|animation|animation-duration")) {
    let value = decl.value.replace(/var\(--motion-[\w-]+\)/g, "");
    if (sheet === "game/animations.css") value = value.replace(BUILD_DURATION, "");
    for (let before = ""; before !== value; ) {
      before = value;
      value = value.replace(/var\([^()]*\)/g, "");
    }
    for (const match of value.matchAll(/(?<![\w.-])(\d*\.?\d+)(ms|s)(?![\w-])/g)) if (Number(match[1]) !== 0) found.push(match[0]);
  }
  return found;
}

describe("#2110 the motion vocabulary", () => {
  it("#2110 motion.css names 4 durations and 5 curves in order, and nothing else", () => {
    expect(DURATIONS).toEqual([["instant", "80ms"], ["fast", "150ms"], ["base", "250ms"], ["slow", "400ms"]]);
    expect(DURATIONS.length).toBe(MOTION_DURATIONS);
    expect(CURVES).toEqual([
      ["standard", "cubic-bezier(0.4, 0, 0.2, 1)"],
      ["out", "cubic-bezier(0.3, 0.7, 0.4, 1)"],
      ["in", "cubic-bezier(0.45, 0, 0.8, 0.7)"],
      ["in-out", "cubic-bezier(0.45, 0, 0.55, 1)"],
      ["overshoot", "cubic-bezier(0.2, 0.9, 0.25, 1.1)"],
    ]);
    expect(CURVES.length).toBe(MOTION_CURVES);
    const rest = codeOf("motion.css").replace(/^\s*--motion-[\w-]+:\s*\d+ms;$/gm, "").replace(/^\s*--ease-[\w-]+:\s*cubic-bezier\([^()]*\);$/gm, "");
    expect(rest.replace(/\s+/g, "")).toBe(":root{}");
  });

  it("#2110 index.css imports motion.css fifth", () => {
    expect(read("index.css")).toMatch(
      /^@import "\.\/fonts\.css";\n@import "\.\/palette\.css";\n@import "\.\/typescale\.css";\n@import "\.\/surfaces\.css";\n@import "\.\/motion\.css";\n/,
    );
  });

  it("#2110 every var(--motion-*/--ease-*) names a token, and only motion.css declares one", () => {
    const names = new Set([...DURATIONS.map((token) => `--motion-${token[0]}`), ...CURVES.map((token) => `--ease-${token[0]}`)]);
    for (const sheet of SHEETS) {
      const css = codeOf(sheet);
      for (const match of css.matchAll(/var\((--(?:motion|ease)-[\w-]+)\)/g)) expect(names.has(match[1] ?? ""), `${sheet}: ${match[0]}`).toBe(true);
      const declared = [...css.matchAll(/--(?:motion|ease)-[\w-]+\s*:/g)].map((match) => match[0].replace(/\s*:$/, ""));
      expect(new Set(declared), sheet).toEqual(sheet === "motion.css" ? names : new Set());
    }
  });
});

describe("#2110 the board moves on the vocabulary", () => {
  it("#2110 every transition and animation on the board names a curve, linear or steps()", () => {
    expect(BOARD_SHEETS.length).toBeGreaterThan(20);
    const found = BOARD_SHEETS.flatMap((sheet) => offCurve(codeOf(sheet)).map((layer) => `${sheet}: ${layer}`));
    expect(found, "name a curve (motion.css) instead").toEqual([]);
  });

  it("#2110 every duration on the board is a token, BUILD's, fed in, or one KEEP names", () => {
    for (const sheet of BOARD_SHEETS) expect(ownTimes(sheet, codeOf(sheet)), sheet).toEqual(KEEP[sheet] ?? []);
    expect(Object.keys(KEEP).filter((sheet) => !BOARD_SHEETS.includes(sheet))).toEqual([]);
  });

  it("#2110 BUILD's animation timings stay as BUILD states them: animations.css's durations take no token", () => {
    expect(codeOf("game/animations.css")).not.toMatch(/var\(--motion-/);
  });

  it("#2110 reduced motion still zeroes every duration, the tokens' included, and none loses --anim-scale", () => {
    const index = codeOf("index.css").replace(/\s+/g, " ");
    expect(index).toContain(
      ':root[data-reduce-motion="true"] *, :root[data-reduce-motion="true"] *::before, :root[data-reduce-motion="true"] *::after { animation-duration: 0s !important; animation-delay: 0s !important; animation-iteration-count: 1 !important; transition-duration: 0s !important; transition-delay: 0s !important;',
    );
    expect(index).toContain("@media (prefers-reduced-motion: reduce) { :root { --anim-scale: 0; } }");
    expect(codeOf("game/animations.css").replace(/\s+/g, " ")).toMatch(
      /@media \(prefers-reduced-motion: reduce\) \{ \*, \*::before, \*::after,[^{]*\{ animation-duration: 0s !important; animation-delay: 0s !important; animation-iteration-count: 1 !important; transition-duration: 0s !important; transition-delay: 0s !important;/,
    );
    const scaled = BOARD_SHEETS.map((sheet) => [sheet, (codeOf(sheet).match(/var\(--anim-scale/g) ?? []).length] as const);
    expect(Object.fromEntries(scaled.filter((entry) => entry[1] > 0))).toEqual(ANIM_SCALED);
  });
});
