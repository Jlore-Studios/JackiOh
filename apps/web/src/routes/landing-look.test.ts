// R5130000: the title screen keeps the gold, the hearth and the embers that issue #5014's motion and glow budget
// (R5014000) took away and issue #5130 brought back.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const CSS = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "landing.css"), "utf8").replace(
  /\/\*[\s\S]*?\*\//g,
  " ",
);

/** The first top-level rule for a selector, as the sheet writes it. */
function rule(selector: string): string {
  const start = CSS.indexOf(`\n${selector} {`);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  return CSS.slice(start, CSS.indexOf("\n}", start));
}

/** A gradient's arguments split at its top-level commas. */
function gradientArgs(body: string, kind: string): string[] {
  const match = new RegExp(String.raw`background:\s*${kind}\(([\s\S]*)\);`).exec(body);
  expect(match, kind).not.toBeNull();
  const args = match?.[1] ?? "";
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  [...args].forEach((ch, i) => {
    if (ch === "(") depth += 1;
    else if (ch === ")") depth -= 1;
    else if (ch === "," && depth === 0) {
      out.push(args.slice(start, i).trim());
      start = i + 1;
    }
  });
  return [...out, args.slice(start).trim()];
}

describe("R5130000 R5014000 withdrawn: the title screen keeps its gold, its hearth and its embers", () => {
  it("R5130000 R5014000 the wordmark is gold leaf, a gradient of several stops, not one flat color", () => {
    const stops = gradientArgs(rule(".landing .landing-wordmark"), "linear-gradient").filter((arg) => !/deg$/.test(arg));
    expect(stops.length).toBeGreaterThanOrEqual(3);
  });

  it("R5130000 R5014000 the hearth, the embers and the fan's float loop for as long as the screen is open", () => {
    for (const name of ["landing-hearth", "landing-ember", "landing-float"]) {
      expect(CSS, name).toMatch(new RegExp(String.raw`\b${name}\b[^;,]*\binfinite\b`));
    }
  });

  it("R5130000 R5014000 the table keeps its shadow and the fan its pool of hearth light", () => {
    expect(rule(".landing .landing-table")).toMatch(/box-shadow:\s*inset\b/);
    expect(gradientArgs(rule(".landing .landing-fan::before"), "radial-gradient").length).toBeGreaterThanOrEqual(3);
  });
});
