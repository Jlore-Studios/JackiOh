// The foil sweep (issue #200): the animated sheen (Mythic and every Radiant face) must loop
// without a jump or a hard line, moving by `transform` only. The test reads the sheet's text
// rather than trusting the comments: Vitest stubs CSS imports, hence `node:fs`
// (game/animations.test.ts does the same for its sheet).
//
// Presentation only: no ruling, so these tests carry no R number.

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * The stylesheet as text, comments stripped: the foil comment itself names the numbers the rules
 * must carry (`170%`, `skewX(-25deg)`), so counting them in the raw text would prove nothing.
 */
const cardsCss: string = (() => {
  for (const candidate of ["src/cards/cards.css", "apps/web/src/cards/cards.css"]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  }
  throw new Error(`cards.css not found from ${process.cwd()}`);
})();

/** The balanced `{ … }` block opened at `open`, inner text without the braces. */
function blockFrom(open: number): string {
  let depth = 0;
  for (let i = open; i < cardsCss.length; i += 1) {
    if (cardsCss[i] === "{") depth += 1;
    else if (cardsCss[i] === "}") {
      depth -= 1;
      if (depth === 0) return cardsCss.slice(open + 1, i);
    }
  }
  throw new Error("unbalanced braces in cards.css");
}

/** The inner text of the first `selector { … }` rule matching `pattern` (a regex over the selector). */
function ruleBody(pattern: RegExp): string {
  const match = pattern.exec(cardsCss);
  expect(match, `no rule matches ${String(pattern)}`).not.toBeNull();
  const open = cardsCss.indexOf("{", match?.index ?? 0);
  return blockFrom(open);
}

/** Every `linear-gradient(…)` value in the sheet, with its parentheses balanced. */
function gradients(): string[] {
  const out: string[] = [];
  for (let at = cardsCss.indexOf("linear-gradient("); at >= 0; at = cardsCss.indexOf("linear-gradient(", at + 1)) {
    let depth = 0;
    for (let i = at; i < cardsCss.length; i += 1) {
      if (cardsCss[i] === "(") depth += 1;
      else if (cardsCss[i] === ")") {
        depth -= 1;
        if (depth === 0) {
          out.push(cardsCss.slice(at, i + 1));
          break;
        }
      }
    }
  }
  return out;
}

describe("the animated foil sweep", () => {
  it("is a horizontal band, clear at both ends, in gold and in Mythic", () => {
    const swept = gradients().filter((gradient) => /\(\s*90deg,/.test(gradient));
    expect(swept, "the sweep's two 90deg gradients").toHaveLength(2);
    for (const gradient of swept) {
      expect(gradient).toMatch(/transparent 0%/);
      expect(gradient).toMatch(/transparent 100%/);
    }
  });

  it("keeps the still foil exactly as it was: the 115deg band, 260% wide, resting left", () => {
    const held = gradients().filter((gradient) => /\(\s*115deg,/.test(gradient));
    expect(held, "the still 115deg gradients").toHaveLength(2);
    const still = ruleBody(/\.cf\[data-foil="static"\]::after,\s*\.cf\[data-foil="animated"\]::after/);
    expect(still).toMatch(/left:\s*-48%/);
    expect(still).toMatch(/width:\s*260%/);
    expect(still).not.toMatch(/animation\s*:/);
  });

  it("rides a layer about a third narrower than the still one", () => {
    const animated = ruleBody(
      /:root:not\(\[data-reduce-motion="true"\]\) \.cf\[data-foil="animated"\]::after/,
    );
    expect(animated).toMatch(/left:\s*0;/);
    expect(animated).toMatch(/width:\s*170%/);
  });

  it("moves by transform alone, on its own layer", () => {
    const animated = ruleBody(
      /:root:not\(\[data-reduce-motion="true"\]\) \.cf\[data-foil="animated"\]::after/,
    );
    expect(animated).toMatch(/will-change:\s*transform/);
    expect(animated).toMatch(/animation:\s*cf-foil-sweep 8s linear infinite/);
    const keyframes = ruleBody(/@keyframes cf-foil-sweep/);
    expect(keyframes).toMatch(/transform:\s*translateX\(-150%\) skewX\(-25deg\)/);
    expect(keyframes).toMatch(/transform:\s*translateX\(110%\) skewX\(-25deg\)/);
    const properties = [...keyframes.matchAll(/([a-z-]+)\s*:[^;{}]+;/gi)].map((match) => match[1]);
    expect(properties, "the keyframes declare nothing but the swept transform").toEqual([
      "transform",
      "transform",
    ]);
  });

  it("rests off the face at both ends of the loop for faces up to 3.6 times as tall as wide", () => {
    const keyframes = ruleBody(/@keyframes cf-foil-sweep/);
    const slides = [...keyframes.matchAll(/translateX\((-?[\d.]+)%\)/g)].map((m) => Number(m[1]));
    const skews = [...keyframes.matchAll(/skewX\((-?[\d.]+)deg\)/g)].map((m) => Number(m[1]));
    expect(slides).toEqual([-150, 110]);
    expect(skews).toEqual([-25, -25]);
    // A skewX(a) moves a face's top and bottom by tan(|a|) of its height each way about the
    // middle; the layer is 170% of the card wide at left 0, and translateX counts in layer widths,
    // so the nearer corner clears the face by the margins below, in card widths.
    const radians = (Math.abs(skews[0] ?? 0) * Math.PI) / 180;
    const shiftPerHeight = Math.tan(radians) / 2;
    const layerWidths = 1.7;
    const rightCornerAtStart = (Math.abs(slides[0] ?? 0) / 100 - 1) * layerWidths;
    const leftCornerAtEnd = ((slides[1] ?? 0) / 100) * layerWidths - 1;
    for (const ratio of [5 / 7, 1.4, 2, 3.6]) {
      expect(rightCornerAtStart - shiftPerHeight * ratio, `start clears a ${String(ratio)} face`).toBeGreaterThan(0);
      expect(leftCornerAtEnd - shiftPerHeight * ratio, `end clears a ${String(ratio)} face`).toBeGreaterThan(0);
    }
  });

  it("runs only with motion allowed, and an animated face otherwise wears the still foil", () => {
    expect(cardsCss).toMatch(/@media \(prefers-reduced-motion: no-preference\)/);
    expect(cardsCss).not.toMatch(/@media \(prefers-reduced-motion: reduce\)/);
    const gated = ruleBody(/@media \(prefers-reduced-motion: no-preference\)/);
    expect(gated).toContain(':root:not([data-reduce-motion="true"]) .cf[data-foil="animated"]::after');
    expect(gated).toContain(
      ':root:not([data-reduce-motion="true"]) .cf[data-rarity="Mythic"][data-foil="animated"]::after',
    );
    // Outside the gate the only animated rule is the shared still one, with no animation of its own.
    const ungated = cardsCss.replace(/@media \(prefers-reduced-motion: no-preference\)[\s\S]*$/, "");
    expect(ungated).toMatch(/\.cf\[data-foil="animated"\]::after/);
    expect(ungated, "no animation outside the motion gate").not.toMatch(/animation\s*:/);
  });
});
