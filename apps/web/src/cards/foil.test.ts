// The animated foil's sweep (cards/cards.css, issue #200). The band used to slide the still
// layer's 115° gradient 256% of the card: part of it was still on the face at both ends of the
// loop, so the sheen jumped every 5.5 s, and on a face taller than 5:7 the gradient reached the
// layer's left edge, which crossed the face as a hard vertical line. The band is now a horizontal
// gradient, clear at both ends, on a 170%-wide layer skewed to the same lean, and it rests
// entirely off the face at both ends of the loop. Vitest stubs CSS imports, so the sheet is read
// as text and parsed into rules and keyframes the way keywordVisuals.test.tsx reads keywords.css.

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

function readSheet(fromWeb: string): string {
  for (const candidate of [fromWeb, `apps/web/${fromWeb}`]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8");
  }
  throw new Error(`${fromWeb} not found from ${process.cwd()}`);
}

type Rule = { selectors: string[]; decls: Map<string, string>; media: string | null };
type Sheet = { rules: Rule[]; keyframes: Map<string, string>; text: string };

function closingBrace(src: string, open: number): number {
  let depth = 0;
  for (let i = open; i < src.length; i += 1) {
    if (src[i] === "{") depth += 1;
    else if (src[i] === "}") {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return src.length;
}

/** Splits on `sep` at bracket depth 0, so `linear-gradient(90deg, a, b)` stays whole. */
function splitTop(text: string, sep: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === "(" || ch === "[") depth += 1;
    else if (ch === ")" || ch === "]") depth -= 1;
    else if (ch === sep && depth === 0) {
      out.push(text.slice(start, i));
      start = i + 1;
    }
  }
  out.push(text.slice(start));
  return out.map((part) => part.trim()).filter((part) => part !== "");
}

function parseDecls(text: string): Map<string, string> {
  const decls = new Map<string, string>();
  for (const part of splitTop(text, ";")) {
    const colon = part.indexOf(":");
    if (colon > 0) {
      decls.set(part.slice(0, colon).trim().toLowerCase(), part.slice(colon + 1).trim().replace(/\s+/g, " "));
    }
  }
  return decls;
}

function walk(src: string, media: string | null, sheet: Sheet): void {
  let i = 0;
  while (i < src.length) {
    const open = src.indexOf("{", i);
    if (open < 0) return;
    const head = src.slice(i, open);
    const prelude = head.slice(Math.max(head.lastIndexOf(";"), head.lastIndexOf("}")) + 1).trim();
    const close = closingBrace(src, open);
    const body = src.slice(open + 1, close);
    if (prelude.startsWith("@keyframes")) {
      sheet.keyframes.set(prelude.replace(/^@keyframes\s+/, "").trim(), body);
    } else if (prelude.startsWith("@media")) {
      walk(body, prelude, sheet);
    } else if (prelude.startsWith("@")) {
      walk(body, media, sheet);
    } else {
      sheet.rules.push({ selectors: splitTop(prelude, ","), decls: parseDecls(body), media });
    }
    i = close + 1;
  }
}

function parseSheet(css: string): Sheet {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const sheet: Sheet = { rules: [], keyframes: new Map(), text };
  walk(text, null, sheet);
  return sheet;
}

const css = parseSheet(readSheet("src/cards/cards.css"));

const ANIMATED = '.cf[data-foil="animated"]::after';
const MYTHIC_ANIMATED = '.cf[data-rarity="Mythic"][data-foil="animated"]::after';
const NO_PREFERENCE = /prefers-reduced-motion\s*:\s*no-preference/;
const REDUCE = /prefers-reduced-motion\s*:\s*reduce/;

/**
 * The rules that draw an animated face's band: the gold one and the Mythic recolour. The shared
 * rule, which draws the still foil on `[data-foil="static"]::after` and `…="animated"]::after`
 * alike, is not one: an animated-only rule names no static face.
 */
function animatedRules(selector: string): Rule[] {
  return css.rules.filter(
    (rule) =>
      rule.selectors.some((s) => s.replace(/\s+/g, " ").endsWith(selector)) &&
      !rule.selectors.some((s) => s.includes('[data-foil="static"]')),
  );
}

/** The one number of the first `name(<n><unit>)` in `text`, or NaN. */
function fnNumber(text: string, name: string, unit: string): number {
  const match = new RegExp(`${name}\\((-?[\\d.]+)${unit}\\)`).exec(text);
  return match === null ? Number.NaN : Number.parseFloat(match[1] ?? "");
}

/** The translateX share of its layer's own width at one end of the sweep, in that width. */
function sweepTranslate(stop: "from" | "to"): number {
  const body = css.keyframes.get("cf-foil-sweep") ?? "";
  const match = new RegExp(`${stop}\\s*\\{([^}]*)\\}`).exec(body);
  return fnNumber(match?.[1] ?? "", "translateX", "%") / 100;
}

describe("#200 the animated foil's band", () => {
  it("#200 is a horizontal gradient, transparent at both ends, so the layer's edges are never a line", () => {
    for (const selector of [ANIMATED, MYTHIC_ANIMATED]) {
      const rules = animatedRules(selector);
      expect(rules.length, selector).toBeGreaterThan(0);
      const background = rules.map((rule) => rule.decls.get("background") ?? "").join(" ");
      expect(background, selector).toMatch(/^linear-gradient\(\s*90deg,/);
      expect(background, `${selector} opens clear`).toMatch(/transparent 0%/);
      expect(background, `${selector} closes clear`).toMatch(/transparent 100%/);
    }
  });

  it("#200 the layer is narrower than the still foil's: a band, not a second face", () => {
    const animated = animatedRules(ANIMATED)[0];
    expect(animated?.decls.get("width")).toBe("170%");
    const still = css.rules.find((rule) =>
      rule.selectors.some((s) => s.includes('[data-foil="static"]')),
    );
    expect(still?.decls.get("width")).toBe("260%");
  });

  it("#200 the sweep moves transform alone, on its own layer", () => {
    const body = css.keyframes.get("cf-foil-sweep") ?? "";
    expect(body, "@keyframes cf-foil-sweep").not.toBe("");
    const animated = new Set<string>();
    for (const stop of body.matchAll(/\{([^{}]*)\}/g)) {
      for (const name of parseDecls(stop[1] ?? "").keys()) animated.add(name);
    }
    expect([...animated]).toEqual(["transform"]);
    expect(animatedRules(ANIMATED).some((rule) => rule.decls.get("will-change") === "transform")).toBe(true);
  });

  it("#200 rests fully off the face at both ends of the loop, however tall the face is", () => {
    const rule = animatedRules(ANIMATED)[0];
    const width = Number.parseFloat(rule?.decls.get("width") ?? "") / 100; // of the card's width
    const skew = Math.abs(fnNumber(css.keyframes.get("cf-foil-sweep") ?? "", "skewX", "deg"));
    // The skew leans the top edge one way and the bottom the other, each by tan(angle)/2 of the
    // layer's height — the face's height, the layer spanning top to bottom (transform-origin centre).
    const lean = Math.tan((skew * Math.PI) / 180) / 2;
    const [fromX, toX] = [sweepTranslate("from"), sweepTranslate("to")];
    expect([width, lean, fromX, toX].every(Number.isFinite)).toBe(true);
    // Faces run from a squat minion to a hand card some three times as tall as it is wide.
    for (const ratio of [0.5, 0.7, 1, 1.4, 2.5, 3.6]) {
      const height = ratio; // the face's height, in card widths
      // Left rest: the nearest corner is the top right, skewed toward the face.
      const rightmost = fromX * width + width + lean * height;
      // Right rest: the nearest corner is the bottom left, skewed toward the face.
      const leftmost = toX * width - lean * height;
      expect(rightmost, `a face ${String(ratio)}x as tall as wide, left rest`).toBeLessThan(0);
      expect(leftmost, `a face ${String(ratio)}x as tall as wide, right rest`).toBeGreaterThan(1);
    }
  });

  it("#200 is gated twice — the media query and the Reduce motion flag — and under them the face wears the still foil", () => {
    for (const selector of [ANIMATED, MYTHIC_ANIMATED]) {
      const rules = animatedRules(selector);
      expect(rules.length, selector).toBeGreaterThan(0);
      for (const rule of rules) {
        expect(rule.media ?? "", selector).toMatch(NO_PREFERENCE);
        for (const s of rule.selectors.filter((s) => s.endsWith(selector))) {
          expect(s, selector).toContain(':not([data-reduce-motion="true"])');
        }
      }
    }
    // Nothing in a reduce block singles a foil out: an animated face falls back to the shared
    // rule above, which draws the still foil on static and animated alike.
    const singled = css.rules.filter(
      (rule) =>
        rule.media !== null &&
        REDUCE.test(rule.media) &&
        rule.selectors.some((s) => s.includes("data-foil")),
    );
    expect(singled.map((rule) => rule.selectors.join(", "))).toEqual([]);
    const shared = css.rules.find(
      (rule) =>
        rule.media === null &&
        rule.selectors.includes('.cf[data-foil="animated"]::after') &&
        rule.selectors.includes('.cf[data-foil="static"]::after'),
    );
    expect(shared, "the still foil still covers an animated face").toBeDefined();
  });
});
