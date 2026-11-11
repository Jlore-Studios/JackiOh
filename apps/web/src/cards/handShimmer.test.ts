// #3674 (R3351000, R3345000): a Mythic's slow edge shimmer in hand (cards.css `cf-mythic-hand-shimmer`) paints to one
// extent at every step. Its coloured shadow grows and shrinks as before; the glow at 0%, as wide as the widest step,
// holds the face's painted extent still, so the browser does not sort the whole page into its layers again every frame.
// The frame times are e2e's cypress/component/fx-phone-living-game.cy.tsx.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "cards.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** The keyframes' steps, each its selector and its box-shadow. */
function shimmerSteps(): { at: string; shadow: string }[] {
  const start = css.indexOf("@keyframes cf-mythic-hand-shimmer");
  expect(start).toBeGreaterThanOrEqual(0);
  const body = css.slice(start, css.indexOf("\n}\n", start));
  return [...body.matchAll(/([0-9]+%)\s*\{\s*box-shadow:\s*([^;]+);\s*\}/g)].map((m) => ({ at: m[1] ?? "", shadow: (m[2] ?? "").replace(/\s+/g, " ").trim() }));
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
      out.push(current.trim());
      current = "";
    } else current += ch;
  }
  out.push(current.trim());
  return out;
}

/** How far a shadow `x y blur spread colour` paints past the box: its blur and its spread, in px. */
function reachOf(shadow: string): number {
  const lengths = [...shadow.matchAll(/(-?[0-9.]+)px/g)].map((m) => Number(m[1]));
  return (lengths[2] ?? 0) + (lengths[3] ?? 0);
}

describe("R3351000 a Mythic's hand shimmer paints to one extent (cards.css)", () => {
  it("R3351000 every step of the shimmer reaches as far as the widest, its colours and sizes as R3345000 drew them", () => {
    const steps = shimmerSteps();
    expect(steps.map((step) => step.at)).toEqual(["0%", "50%", "100%"]);
    const reaches = steps.map((step) => Math.max(...shadowsOf(step.shadow).map(reachOf)));
    expect(new Set(reaches).size).toBe(1);
    // The coloured shadow of each step is the one it always was; the other is a glow at 0%, which draws nothing.
    expect(steps.map((step) => shadowsOf(step.shadow))).toEqual([
      ["0 0 4px 1px color-mix(in srgb, var(--pal-violet-300) 35%, transparent)", "0 0 6px 2px color-mix(in srgb, var(--pal-sky-300) 0%, transparent)"],
      ["0 0 6px 2px color-mix(in srgb, var(--pal-sky-300) 35%, transparent)", "0 0 6px 2px color-mix(in srgb, var(--pal-sky-300) 0%, transparent)"],
      ["0 0 4px 1px color-mix(in srgb, var(--pal-green-200) 35%, transparent)", "0 0 6px 2px color-mix(in srgb, var(--pal-sky-300) 0%, transparent)"],
    ]);
  });
});
