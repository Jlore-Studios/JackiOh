// #2107: the board's text takes its sizes from one type scale, typescale.css, and its faces from
// fonts.css's four families; no stylesheet gains its own unless this file says so.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

/** The scale's size limit: a new step raises it in the same change. */
const TYPE_MAX = 9;
/** The step the page's `body` is set in (#5013), at least BODY_MIN_PX. */
const BODY_STEP = "lg";
/** The step `.app-shell--wide` sets the board in; every other step's em form is a fraction of it. */
const BOARD_STEP = "md";
/** The smallest the page's body may be set in (#5013). */
const BODY_MIN_PX = 15;
/** The smallest step a sheet outside cards/ may name (#5013); card faces keep the steps under it. */
const OFF_FACE_MIN_PX = 11;
/** The most own sizes and faces the other sheets may hold in all: half of the 432 they held before #5013. It only falls. */
const UNSCALED_MAX = 216;
/** The faces a stylesheet may name: fonts.css's four families (R2715002: and the signature's hands), or its parent's. */
const FAMILIES = [
  "inherit",
  "var(--font-display)",
  "var(--font-text)",
  "var(--font-mono)",
  "var(--font-signature)",
  // R2715002: the sets' signature hands, which only a Mythic full art's signature is set in.
  "var(--font-signature-brush)",
  "var(--font-signature-copperplate)",
  "var(--font-signature-flourish)",
  "var(--font-signature-ink)",
  "var(--font-signature-tag)",
  "var(--font-signature-marker)",
];

const STEP = /var\(--type-[a-z0-9]+(?:-em)?\)/;
/** Viewport and container units, in the order the pattern below tried them. */
const FLUID_UNITS = ["vw", "vh", "vmin", "vmax", "dvh", "svh", "lvh", "cqw", "cqh", "cqi", "cqb", "cqmin", "cqmax"];

/**
 * `value` without its `<number><fluid-unit>` terms, as `/\d*\.?\d+(?:vw|…)\b/g` cut them (#4009: the
 * number plus unit alternation rescans from every position).
 */
function stripFluid(value: string): string {
  let out = "";
  let at = 0;
  while (at < value.length) {
    let i = at;
    let numEnd = -1;
    if (/[0-9]/.test(value.charAt(i))) {
      while (i < value.length && /[0-9]/.test(value.charAt(i))) i++;
      if (value.charAt(i) === "." && /[0-9]/.test(value.charAt(i + 1))) {
        i++;
        while (i < value.length && /[0-9]/.test(value.charAt(i))) i++;
      }
      numEnd = i;
    } else if (value.charAt(i) === "." && /[0-9]/.test(value.charAt(i + 1))) {
      i += 2;
      while (i < value.length && /[0-9]/.test(value.charAt(i))) i++;
      numEnd = i;
    }
    if (numEnd !== -1) {
      const unit = FLUID_UNITS.find((name) => value.startsWith(name, numEnd));
      if (unit !== undefined && (numEnd + unit.length >= value.length || !/\w/.test(value.charAt(numEnd + unit.length)))) {
        at = numEnd + unit.length;
        continue;
      }
    }
    out += value.charAt(at);
    at++;
  }
  return out;
}
const KEYWORD = /\b(?:xx-small|x-small|small|medium|large|x-large|xx-large|xxx-large|smaller|larger)\b/;

function offScale(css: string): string[] {
  const code = css.replace(/\/\*[\s\S]*?\*\//g, " ");
  const found: string[] = [];
  for (const match of code.matchAll(/(?<![\w-])font-size\s*:\s*([^;}]+)/g)) {
    const value = (match[1] ?? "").trim();
    const rest = stripFluid(value.replace(/var\(--[a-z0-9-]+\)/g, ""));
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

/** The board's stylesheets, as palette.test.ts has them. */
const BOARD_SHEETS = SHEETS.filter(
  (sheet) =>
    sheet === "index.css" ||
    sheet === "buttons.css" ||
    (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

/** Every other sheet's own sizes and faces: one that loses some lowers its number, and none gains one. */
const UNSCALED: Record<string, number> = {
  "audio/audio.css": 2,
  "auth/auth.css": 4,
  "auth/code-field.css": 11,
  "auth/consent.css": 3,
  "auth/tavern.css": 4,
  "auth/two-step.css": 1,
  "cards/cards.css": 31,
  "cards/cardstate.css": 8,
  "cards/inspect/list-play.css": 1,
  "cards/keywords.css": 3,
  "cards/marks.css": 1,
  "cards/wheel/wheel.css": 3,
  "collection/collection.css": 8,
  "duos/duos.css": 14,
  "economy/economy.css": 6,
  "economy/xp.css": 1,
  "emotes/emotes.css": 7,
  "emotes/portrait.css": 3,
  "fx/fx.css": 8,
  "i18n/language.css": 1,
  "packs/pack-object.css": 1,
  "packs/packs.css": 4,
  "patches/patches.css": 4,
  "practice/practice.css": 3,
  "practice/sandbox.css": 3,
  "routes/account.css": 3,
  "routes/game-banner.css": 5,
  "routes/landing.css": 20,
  "routes/privacy.css": 3,
  "routes/purchases.css": 2,
  "routes/site-footer.css": 1,
  "settings/changeEmail.css": 1,
  "settings/settings.css": 10,
  "shop/shop.css": 2,
  "together/together.css": 6,
  "tutorial/path-visibility.css": 3,
  "tutorial/tutorial.css": 1,
};

/**
 * Each line's captures (#4009: `/^\s*--…$/gm` line patterns rescan newline runs from every line
 * start); `pattern` is the line without its `m` flag.
 */
function tokenCaptures(text: string, pattern: RegExp): string[][] {
  const out: string[][] = [];
  for (const line of text.split("\n")) {
    const match = pattern.exec(line);
    if (match !== null) out.push(match.slice(1).map((group) => group ?? ""));
  }
  return out;
}

const STEPS = tokenCaptures(read("typescale.css"), /^\s*--type-([a-z0-9]+):\s*(\d+)px;$/).map(([name = "", px = ""]) => ({
  name,
  px: Number(px),
}));
const EM_FORMS = tokenCaptures(read("typescale.css"), /^\s*--type-([a-z0-9]+)-em:\s*calc\((\d+)em \/ (\d+)\);$/).map(
  ([name = "", px = "", over = ""]) => ({ name, px: Number(px), over: Number(over) }),
);

describe("#2107 the type scale", () => {
  it("#2107 typescale.css names at most TYPE_MAX sizes in px, smallest first, each but the board's with its em form, and nothing else", () => {
    const sizes = STEPS.map((step) => step.px);
    expect(STEPS.length).toBeGreaterThan(0);
    expect(STEPS.length).toBeLessThanOrEqual(TYPE_MAX);
    expect(new Set(sizes).size, "two names for one size").toBe(sizes.length);
    expect(sizes).toEqual([...sizes].sort((a, b) => a - b));
    const board = STEPS.find((step) => step.name === BOARD_STEP)?.px;
    expect(board).toBeDefined();
    expect(EM_FORMS).toEqual(STEPS.filter((step) => step.name !== BOARD_STEP).map((step) => ({ name: step.name, px: step.px, over: board })));
    const text = read("typescale.css").replace(/\/\*[\s\S]*?\*\//g, "");
    // Token lines blanked (#4009: the `/^\s*--…$/gm` patterns rescan newline runs from every line).
    const rest = text
      .split("\n")
      .map((line) =>
        /^\s*--type-[a-z0-9]+:\s*\d+px;$/.test(line) || /^\s*--type-[a-z0-9]+-em:\s*calc\(\d+em \/ \d+\);$/.test(line) ? "" : line,
      )
      .join("\n");
    expect(rest.replace(/\s+/g, "")).toBe(":root{}");
    expect(read("index.css")).toMatch(/^@import "\.\/fonts\.css";\n@import "\.\/palette\.css";\n@import "\.\/typescale\.css";\n/);
  });

  it("#2107 #5013 the page's body is set in the body step and the board in the board step, which the em forms are fractions of", () => {
    const css = read("index.css");
    expect(css).toMatch(new RegExp(`\\nbody \\{[^}]*\\n {2}font-family: var\\(--font-text\\);\\n {2}font-size: var\\(--type-${BODY_STEP}\\);`));
    expect(css).toMatch(new RegExp(`\\n\\.app-shell\\.app-shell--wide \\{[^}]*\\n {2}font-size: var\\(--type-${BOARD_STEP}\\);`));
  });

  it("#5013 the body step is at least BODY_MIN_PX", () => {
    const body = STEPS.find((step) => step.name === BODY_STEP)?.px;
    expect(body).toBeDefined();
    expect(body).toBeGreaterThanOrEqual(BODY_MIN_PX);
  });

  it("#5013 no sheet outside cards/ names a step under OFF_FACE_MIN_PX", () => {
    const px = new Map(STEPS.map((step) => [step.name, step.px]));
    const offenders = SHEETS.filter((sheet) => !sheet.startsWith("cards/")).flatMap((sheet) =>
      [...read(sheet).replace(/\/\*[\s\S]*?\*\//g, " ").matchAll(/var\(--type-([a-z0-9]+)(?:-em)?\)/g)]
        .filter((match) => (px.get(match[1] ?? "") ?? 0) < OFF_FACE_MIN_PX)
        .map((match) => `${sheet}: ${match[0]}`),
    );
    expect(offenders, "a card face's small steps stay on the card; take --type-xs or larger").toEqual([]);
  });

  it("#2107 every var(--type-…) names a step, only typescale.css declares one, and only fonts.css declares a family", () => {
    const names = new Set([...STEPS.map((step) => `--type-${step.name}`), ...EM_FORMS.map((form) => `--type-${form.name}-em`)]);
    for (const sheet of SHEETS) {
      const css = read(sheet);
      for (const match of css.matchAll(/var\((--type-[a-z0-9-]+)\)/g)) expect(names.has(match[1] ?? ""), `${sheet}: ${match[0]}`).toBe(true);
      if (sheet !== "typescale.css") expect(css, sheet).not.toMatch(/--type-[a-z0-9-]+\s*:/);
      if (sheet !== "fonts.css") expect(css, sheet).not.toMatch(/--font-[a-z0-9-]+\s*:/);
    }
    expect(tokenCaptures(read("fonts.css"), /^\s*(--font-[a-z0-9-]+):/).map(([name]) => name)).toEqual([
      "--font-display",
      "--font-text",
      "--font-mono",
      "--font-signature",
      "--font-signature-brush",
      "--font-signature-copperplate",
      "--font-signature-flourish",
      "--font-signature-ink",
      "--font-signature-tag",
      "--font-signature-marker",
    ]);
  });

  it("R5264500 the display and text stacks reach a face for the Egyptian Hieroglyphs block, for its code points alone (FvH #84 𓃠)", () => {
    const fonts = read("fonts.css").replace(/\/\*[\s\S]*?\*\//g, " ");
    const face = [...fonts.matchAll(/@font-face\s*\{([^}]*)\}/g)].map((match) => match[1] ?? "").find((body) => body.includes('"Hieroglyphs Fallback"'));
    expect(face).toBeDefined();
    expect(face).toMatch(/unicode-range:\s*U\+13000-1345F;/);
    expect(face).toContain('local("Noto Sans Egyptian Hieroglyphs")');
    expect(face).toContain('local("Segoe UI Historic")');
    const name = "𓃠".codePointAt(0) ?? 0;
    expect(name >= 0x13000 && name <= 0x1345f).toBe(true);
    for (const token of ["--font-display", "--font-text"]) {
      const stack = new RegExp(`${token}:\\s*([^;]+);`).exec(fonts)?.[1] ?? "";
      const families = stack.split(",").map((family) => family.trim());
      // After the web face's metric-matched fallback, so Latin text measures as before.
      expect(families.indexOf('"Hieroglyphs Fallback"'), token).toBe(2);
    }
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

  it("#5013 the other sheets' own sizes and faces number at most UNSCALED_MAX", () => {
    expect(Object.values(UNSCALED).reduce((sum, count) => sum + count, 0)).toBeLessThanOrEqual(UNSCALED_MAX);
  });

  it("#2107 the board's components set no size or face inline", () => {
    const components = files(join(SRC, "game"), ".tsx").filter((path) => !path.endsWith(".test.tsx"));
    expect(components.length).toBeGreaterThan(20);
    for (const path of components) expect(readFileSync(path, "utf8"), relative(SRC, path)).not.toMatch(/\bfont(?:Size|Family)\b|\bfont-(?:size|family)\b/);
  });
});
