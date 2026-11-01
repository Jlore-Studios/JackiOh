// #2107: the board's text takes its sizes from one type scale, typescale.css, and its faces from
// fonts.css's four families, and no stylesheet gains a size or a face of its own unless this file
// says so.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

/** The scale's size limit: a new step raises it in the same change, in plain sight. */
const TYPE_MAX = 10;
/** The step the board's body is set in; every other step's em form is a fraction of it. */
const BODY_STEP = "md";
/** The faces a stylesheet may name: fonts.css's four families, or its parent's. */
const FAMILIES = ["inherit", "var(--font-display)", "var(--font-text)", "var(--font-mono)", "var(--font-signature)"];

const STEP = /var\(--type-[a-z0-9]+(?:-em)?\)/;
/** A viewport or container term, which a clamp() or max() may hold beside a step. */
const FLUID = /\d*\.?\d+(?:vw|vh|vmin|vmax|dvh|svh|lvh|cqw|cqh|cqi|cqb|cqmin|cqmax)\b/g;
const KEYWORD = /\b(?:xx-small|x-small|small|medium|large|x-large|xx-large|xxx-large|smaller|larger)\b/;

/** Every font-size, font-family and font shorthand `css` states off the scale, outside its comments. */
function offScale(css: string): string[] {
  const code = css.replace(/\/\*[\s\S]*?\*\//g, " ");
  const found: string[] = [];
  for (const match of code.matchAll(/(?<![\w-])font-size\s*:\s*([^;}]+)/g)) {
    const value = (match[1] ?? "").trim();
    const rest = value.replace(/var\(--[a-z0-9-]+\)/g, "").replace(FLUID, "");
    if (value !== "inherit" && (!STEP.test(value) || /\d/.test(rest) || KEYWORD.test(rest))) found.push(`font-size: ${value}`);
  }
  for (const match of code.matchAll(/(?<![\w-])font-family\s*:\s*([^;}]+)/g)) {
    const value = (match[1] ?? "").trim();
    if (!FAMILIES.includes(value)) found.push(`font-family: ${value}`);
  }
  for (const match of code.matchAll(/(?<![\w-])font\s*:\s*([^;}]+)/g)) {
    const value = (match[1] ?? "").trim();
    if (value !== "inherit") found.push(`font: ${value.replace(/\s+/g, " ")}`);
  }
  return found;
}

function files(dir: string, ending: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : files(path, ending);
    return name.endsWith(ending) ? [path] : [];
  });
}

const SHEETS = files(SRC, ".css").map((path) => relative(SRC, path).replaceAll("\\", "/")).sort();
const read = (sheet: string): string => readFileSync(join(SRC, sheet), "utf8");

/** The board's stylesheets, as palette.test.ts has them: index.css and game/'s, but the deck builder's and table.css. */
const BOARD_SHEETS = SHEETS.filter(
  (sheet) => sheet === "index.css" || (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

/** Every other sheet's own sizes and faces: one that loses some lowers its number, and none gains one. */
const UNSCALED: Record<string, number> = {
  "audio/audio.css": 2,
  "auth/auth.css": 4,
  "auth/code-field.css": 11,
  "auth/consent.css": 3,
  "auth/tavern.css": 23,
  "auth/two-step.css": 1,
  "cards/cards.css": 29,
  "cards/cardstate.css": 8,
  "cards/inspect/inspect.css": 34,
  "cards/inspect/list-play.css": 1,
  "cards/keywords.css": 3,
  "cards/marks.css": 1,
  "cards/wheel/wheel.css": 3,
  "collection/collection.css": 10,
  "creator/creator.css": 8,
  "duos/duos.css": 14,
  "economy/economy.css": 6,
  "economy/xp.css": 1,
  "emotes/emotes.css": 7,
  "emotes/portrait.css": 3,
  "fx/fx.css": 8,
  "game/deckbuilder/deckbuilder.css": 19,
  "game/deckbuilder/workshop.css": 28,
  "i18n/language.css": 1,
  "packs/pack-object.css": 1,
  "packs/packs.css": 4,
  "patches/patches.css": 30,
  "practice/practice.css": 42,
  "practice/sandbox.css": 3,
  "routes/account.css": 3,
  "routes/game-banner.css": 5,
  "routes/landing.css": 20,
  "routes/lobby.css": 54,
  "routes/privacy.css": 5,
  "routes/purchases.css": 2,
  "routes/site-footer.css": 1,
  "routes/stats.css": 30,
  "settings/changeEmail.css": 1,
  "settings/settings.css": 12,
  "shop/shop.css": 2,
  "together/together.css": 10,
  "tutorial/path-visibility.css": 3,
  "tutorial/tutorial.css": 33,
};

const STEPS = [...read("typescale.css").matchAll(/^\s*--type-([a-z0-9]+):\s*(\d+)px;$/gm)].map((match) => ({
  name: match[1] ?? "",
  px: Number(match[2]),
}));
const EM_FORMS = [...read("typescale.css").matchAll(/^\s*--type-([a-z0-9]+)-em:\s*calc\((\d+)em \/ (\d+)\);$/gm)].map((match) => ({
  name: match[1] ?? "",
  px: Number(match[2]),
  over: Number(match[3]),
}));

describe("#2107 the type scale", () => {
  it("#2107 typescale.css names at most TYPE_MAX sizes in px, smallest first, each but the body's with its em form, and nothing else", () => {
    const sizes = STEPS.map((step) => step.px);
    expect(STEPS.length).toBeGreaterThan(0);
    expect(STEPS.length).toBeLessThanOrEqual(TYPE_MAX);
    expect(new Set(sizes).size, "two names for one size").toBe(sizes.length);
    expect(sizes).toEqual([...sizes].sort((a, b) => a - b));
    const body = STEPS.find((step) => step.name === BODY_STEP)?.px;
    expect(body).toBeDefined();
    expect(EM_FORMS).toEqual(STEPS.filter((step) => step.name !== BODY_STEP).map((step) => ({ name: step.name, px: step.px, over: body })));
    const text = read("typescale.css").replace(/\/\*[\s\S]*?\*\//g, "");
    const rest = text.replace(/^\s*--type-[a-z0-9]+:\s*\d+px;$/gm, "").replace(/^\s*--type-[a-z0-9]+-em:\s*calc\(\d+em \/ \d+\);$/gm, "");
    expect(rest.replace(/\s+/g, "")).toBe(":root{}");
    expect(read("index.css")).toMatch(/^@import "\.\/fonts\.css";\n@import "\.\/palette\.css";\n@import "\.\/typescale\.css";\n/);
  });

  it("#2107 the board's body is set in the body step, which the em forms are fractions of", () => {
    const css = read("index.css");
    expect(css).toMatch(/\nbody \{[^}]*\n {2}font-family: var\(--font-text\);\n {2}font-size: var\(--type-md\);/);
    expect(css).toMatch(/\n\.app-shell\.app-shell--wide \{[^}]*\n {2}font-size: var\(--type-md\);/);
  });

  it("#2107 every var(--type-…) names a step, only typescale.css declares one, and only fonts.css declares a family", () => {
    const names = new Set([...STEPS.map((step) => `--type-${step.name}`), ...EM_FORMS.map((form) => `--type-${form.name}-em`)]);
    for (const sheet of SHEETS) {
      const css = read(sheet);
      for (const match of css.matchAll(/var\((--type-[a-z0-9-]+)\)/g)) expect(names.has(match[1] ?? ""), `${sheet}: ${match[0]}`).toBe(true);
      if (sheet !== "typescale.css") expect(css, sheet).not.toMatch(/--type-[a-z0-9-]+\s*:/);
      if (sheet !== "fonts.css") expect(css, sheet).not.toMatch(/--font-[a-z0-9-]+\s*:/);
    }
    expect([...read("fonts.css").matchAll(/^\s*(--font-[a-z0-9-]+):/gm)].map((match) => match[1])).toEqual([
      "--font-display",
      "--font-text",
      "--font-mono",
      "--font-signature",
    ]);
  });
});

describe("#2107 no one-off sizes or faces", () => {
  it("#2107 the board's stylesheets state no size but the scale's and no face but the four families", () => {
    expect(BOARD_SHEETS.length).toBeGreaterThan(20);
    const found = BOARD_SHEETS.flatMap((sheet) => offScale(read(sheet)).map((declaration) => `${sheet}: ${declaration}`));
    expect(found, "name a step (typescale.css) or a family (fonts.css) instead").toEqual([]);
  });

  it("#2107 no other stylesheet gains a size or a face of its own", () => {
    const counts: Record<string, number> = {};
    for (const sheet of SHEETS) {
      if (sheet === "fonts.css" || sheet === "typescale.css" || BOARD_SHEETS.includes(sheet)) continue;
      const own = offScale(read(sheet)).length;
      if (own > 0) counts[sheet] = own;
    }
    expect(counts, "take the size from typescale.css and the face from fonts.css; a sheet that loses some lowers its number in UNSCALED").toEqual(UNSCALED);
  });

  it("#2107 the board's components set no size or face inline", () => {
    const components = files(join(SRC, "game"), ".tsx").filter((path) => !path.endsWith(".test.tsx"));
    expect(components.length).toBeGreaterThan(20);
    for (const path of components) expect(readFileSync(path, "utf8"), relative(SRC, path)).not.toMatch(/\bfont(?:Size|Family)\b|\bfont-(?:size|family)\b/);
  });
});
