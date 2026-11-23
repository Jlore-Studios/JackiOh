// #3674 (R3351000, R3345000), held by R3608000: a Mythic's slow edge shimmer in hand (cards.css `cf-mythic-hand-shimmer`)
// paints to one extent at every step, so the browser does not sort the whole page into its layers again every frame.
// It holds by construction: the loop moves one layer's opacity and nothing else, and the rim that layer draws is inset,
// so it paints inside the card's box and nothing reaches past it. The frame times are e2e's
// cypress/component/fx-phone-living-game.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "cards.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** The text between the braces of the first block whose prelude starts with `prelude`, nested blocks and all. */
function blockOf(prelude: string): string {
  const start = css.indexOf(prelude);
  expect(start, prelude).toBeGreaterThanOrEqual(0);
  const open = css.indexOf("{", start);
  let depth = 0;
  for (let at = open; at < css.length; at += 1) {
    if (css[at] === "{") depth += 1;
    if (css[at] === "}") depth -= 1;
    if (depth === 0) return css.slice(open + 1, at);
  }
  throw new Error(`${prelude} is never closed`);
}

/** `list` split at its top-level commas (a `color-mix(…)` keeps its own). */
function shadowsOf(list: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let current = "";
  for (const ch of list) {
    if (ch === "(") depth += 1;
    if (ch === ")") depth -= 1;
    if (ch === "," && depth === 0) {
      out.push(current.replace(/\s+/g, " ").trim());
      current = "";
    } else current += ch;
  }
  out.push(current.replace(/\s+/g, " ").trim());
  return out;
}

/**
 * A keyframes body's `0% { … }` steps (#4009: `/([0-9]+%)\s*\{([^}]*)\}/g` rescans a digit run from
 * every position).
 */
function keyframeSteps(body: string): [string, string][] {
  const out: [string, string][] = [];
  let at = 0;
  while (at < body.length) {
    const percent = body.indexOf("%", at);
    if (percent === -1) return out;
    let start = percent;
    while (start > 0 && /[0-9]/.test(body.charAt(start - 1))) start--;
    if (start === percent) {
      at = percent + 1;
      continue;
    }
    let i = percent + 1;
    while (i < body.length && /\s/.test(body.charAt(i))) i++;
    if (body.charAt(i) !== "{") {
      at = percent + 1;
      continue;
    }
    const close = body.indexOf("}", i + 1);
    if (close === -1) return out;
    out.push([body.slice(start, percent + 1), body.slice(i + 1, close)]);
    at = close + 1;
  }
  return out;
}

const RIM = ':root:not([data-reduce-motion="true"]) .card-hand:has(.cf[data-rarity="Mythic"])::after';

describe("R3351000 a Mythic's hand shimmer paints to one extent (cards.css)", () => {
  it("R3351000 the loop moves the rim's opacity and nothing else, so no step can change what it paints", () => {
    const body = blockOf("@keyframes cf-mythic-hand-shimmer");
    const steps = keyframeSteps(body).map(([at, declarations]) => ({
      at,
      properties: declarations.split(";").map((declaration) => declaration.split(":")[0]?.trim() ?? "").filter((name) => name !== ""),
    }));
    expect(steps.map((step) => step.at)).toEqual(["0%", "100%"]);
    for (const step of steps) expect(step.properties, step.at).toEqual(["opacity"]);
  });

  it("R3351000 the rim is inset, so it paints inside the card's box at every step, in the colours and sizes R3345000 drew", () => {
    const rim = blockOf(RIM);
    // No `\s*` before the capture (#4009): it trades characters with `[^;]+` super-linearly.
    const list = (/box-shadow:([^;]+);/.exec(rim)?.[1] ?? "").trimStart();
    expect(shadowsOf(list)).toEqual([
      "inset 0 0 6px 2px color-mix(in srgb, var(--pal-sky-300) 35%, transparent)",
      "inset 0 0 4px 1px color-mix(in srgb, var(--pal-violet-300) 35%, transparent)",
      "inset 0 0 3px 1px color-mix(in srgb, var(--pal-green-200) 35%, transparent)",
    ]);
    // An inset shadow reaches nowhere outside the box, and the layer is the box itself.
    expect(rim).toMatch(/inset:\s*0;/);
    expect(rim).not.toMatch(/filter:/);
    expect(rim).toMatch(/animation:\s*cf-mythic-hand-shimmer\b/);
  });
});
