// #2123 (part of #2036): tabular numbers and balanced headlines across the menus. The sheets are read
// as text (Vitest stubs CSS imports) and parsed with `fx/css.test.ts`'s parser, copied below. Card
// faces get neither: `cards/fit.ts` fits their text.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

type Rule = { selectors: string[]; decls: Map<string, string>; media: string | null };
type Sheet = { rules: Rule[]; keyframes: Set<string>; text: string };

function closingBrace(src: string, open: number): number {
  let depth = 0;
  for (let i = open; i < src.length; i += 1) {
    const ch = src[i];
    if (ch === "{") depth += 1;
    else if (ch === "}") {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return src.length;
}

/** Splits on `sep` at bracket depth 0, so `:is(a, b)` and `rgb(1, 2, 3)` stay whole. */
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
    if (colon <= 0) continue;
    decls.set(part.slice(0, colon).trim().toLowerCase(), part.slice(colon + 1).trim().replace(/\s+/g, " "));
  }
  return decls;
}

/** Native nesting: `&` stands for the parent, and a bare child selector is a descendant of it. */
function nest(parents: readonly string[] | null, selectors: readonly string[]): string[] {
  if (parents === null) return [...selectors];
  return parents.flatMap((parent) =>
    selectors.map((child) => (child.includes("&") ? child.replaceAll("&", parent) : `${parent} ${child}`)),
  );
}

function walk(src: string, media: string | null, parents: string[] | null, sheet: Sheet): string {
  let flat = "";
  let i = 0;
  while (i < src.length) {
    const open = src.indexOf("{", i);
    if (open < 0) {
      flat += src.slice(i);
      break;
    }
    const head = src.slice(i, open);
    const cut = Math.max(head.lastIndexOf(";"), head.lastIndexOf("}"));
    flat += head.slice(0, cut + 1);
    const prelude = head.slice(cut + 1).trim();
    const close = closingBrace(src, open);
    const body = src.slice(open + 1, close);
    if (/^@(-webkit-)?keyframes\s/.test(prelude)) {
      sheet.keyframes.add(prelude.replace(/^@(-webkit-)?keyframes\s+/, "").trim());
    } else if (prelude.startsWith("@media")) {
      walk(body, prelude, parents, sheet);
    } else if (prelude.startsWith("@")) {
      walk(body, media, parents, sheet);
    } else {
      const selectors = nest(parents, splitTop(prelude, ","));
      const own = walk(body, media, selectors, sheet);
      sheet.rules.push({ selectors, decls: parseDecls(own), media });
    }
    i = close + 1;
  }
  return flat;
}

function parseSheet(css: string): Sheet {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const sheet: Sheet = { rules: [], keyframes: new Set(), text };
  walk(text, null, null, sheet);
  return sheet;
}

/** Selector text with quotes dropped and the whitespace that carries no meaning removed. */
function normSelector(selector: string): string {
  return selector
    .replace(/['"]/g, "")
    .replace(/\s+/g, " ")
    .replace(/\s*([>,+~=])\s*/g, "$1")
    .replace(/\(\s+/g, "(")
    .replace(/\s+\)/g, ")")
    .replace(/\[\s+/g, "[")
    .replace(/\s+\]/g, "]")
    .trim();
}

function sheet(path: string): Sheet {
  return parseSheet(readFileSync(join(SRC, path), "utf8"));
}

/** The declarations every rule listing `selector` (compared normalized) gives it, later rules winning. */
function declsFor(parsed: Sheet, selector: string): Map<string, string> {
  const wanted = normSelector(selector);
  const merged = new Map<string, string>();
  for (const rule of parsed.rules) {
    if (rule.selectors.some((s) => normSelector(s) === wanted)) for (const [k, v] of rule.decls) merged.set(k, v);
  }
  return merged;
}

/** Every stylesheet under src/, walked as cursors.test.ts walks them. */
function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

/** Each number that changes in place or stands in a column, by the sheet that draws it. */
const TABULAR: Readonly<Record<string, readonly string[]>> = {
  "game/clock.css": [".clock-digits"],
  "shop/shop.css": [".shop-wallet", ".shop-row__info", ".token-held", ".cosmetic-packs-held"],
  "packs/packs.css": [".packs-held"],
  "economy/economy.css": [".fill-bar__caption"],
  "economy/xp.css": [".xp-reveal__level", ".xp-reveal__quest-pay"],
  "collection/collection.css": [".tavern.app-shell .collection-set__toggle", ".tavern.app-shell .collection-list"],
  "routes/leaderboard.css": [".tavern.app-shell.leaderboard-screen ol", ".tavern.app-shell.leaderboard-screen ul"],
  "routes/stats.css": [".stats-table", ".stats-grid-figures dd", ".stats-pager-page"],
};

/** The headings, the dialog and toast titles that are not headings, and the result's word. */
const BALANCED = [
  "h1", "h2", "h3", ".prompt-title", ".hero-inspect-title", ".challenge-toast__text", ".draw-toast__line", ".result-overlay__word",
];

/** A card face's selectors: the face `.cf` and its parts `.cf-*` (cards/CardFace.tsx). */
const CARD_FACE = /\.cf(?![\w-])|\.cf-[\w-]/;

/** The card-face rules that set font-variant-numeric before #2123 (sorted). */
const FACE_NUMERIC = [
  "cards/cards.css: .cf .cf-num",
  "cards/cards.css: .cf .cost-gem",
  "cards/cards.css: .cf .stat",
  "cards/cardstate.css: .cf .cf-state-text",
];

describe("#2123 tabular numbers", () => {
  for (const [path, selectors] of Object.entries(TABULAR)) {
    it(`#2123 ${path} draws its counts, prices and figures in tabular numbers`, () => {
      const parsed = sheet(path);
      for (const selector of selectors) {
        expect(declsFor(parsed, selector).get("font-variant-numeric"), selector).toBe("tabular-nums");
      }
    });
  }
});

describe("#2123 balanced headlines", () => {
  const index = sheet("index.css");

  it("#2123 index.css balances every heading, the dialog and toast titles and the result's word", () => {
    for (const selector of BALANCED) expect(declsFor(index, selector).get("text-wrap-style"), selector).toBe("balance");
  });

  it("#2123 index.css wraps paragraphs pretty, before the titles, so a title that is a <p> still balances", () => {
    expect(declsFor(index, "p").get("text-wrap-style")).toBe("pretty");
    const at = (selector: string, value: string): number =>
      index.rules.findIndex(
        (rule) => rule.selectors.some((s) => normSelector(s) === selector) && rule.decls.get("text-wrap-style") === value,
      );
    expect(at("p", "pretty")).toBeGreaterThanOrEqual(0);
    expect(at(".prompt-title", "balance")).toBeGreaterThan(at("p", "pretty"));
  });

  it("#2123 index.css sets only the style half, so a white-space: nowrap an element inherits still holds", () => {
    const offenders = index.rules.filter((rule) => rule.decls.has("text-wrap") || rule.decls.has("text-wrap-mode"));
    expect(offenders.map((rule) => rule.selectors.join(", "))).toEqual([]);
  });
});

describe("#2123 card faces keep the wrap and the figures cards/fit.ts measures", () => {
  const faceRules = cssFiles(SRC).flatMap((path) =>
    parseSheet(readFileSync(path, "utf8")).rules.flatMap((rule) => {
      const faces = rule.selectors.filter((selector) => CARD_FACE.test(selector));
      return faces.length === 0 ? [] : [{ where: `${relative(SRC, path)}: ${faces.join(", ")}`, decls: rule.decls }];
    }),
  );

  it("#2123 the scan reaches the card faces' rules", () => {
    expect(faceRules.length).toBeGreaterThan(100);
  });

  it("#2123 no card-face selector sets text-wrap", () => {
    const offenders = faceRules.filter(({ decls }) => [...decls.keys()].some((name) => name.startsWith("text-wrap")));
    expect(offenders.map(({ where }) => where)).toEqual([]);
  });

  it("#2123 no card-face selector gets a font-variant-numeric it did not have", () => {
    const setting = faceRules.filter(({ decls }) => decls.has("font-variant-numeric")).map(({ where }) => where);
    expect(setting.sort()).toEqual(FACE_NUMERIC);
  });
});
