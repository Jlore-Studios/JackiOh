// #2105: the board's colors come from one named palette, palette.css, and no stylesheet gains a
// color of its own unless this file says so.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

/** The palette's size limit: a new color raises it in the same change, in plain sight. */
const PALETTE_MAX = 43;
/** WCAG 2.2 AA's contrast for text (1.4.3). */
const AA_TEXT_CONTRAST = 4.5;
/** R5012000: the Tailwind defaults palette.css held before #5012; none comes back. */
const TAILWIND_DEFAULTS = [
  "#6b7280",
  "#fca5a5",
  "#f87171",
  "#ef4444",
  "#7f1d1d",
  "#fb923c",
  "#d9f99d",
  "#a3e635",
  "#bbf7d0",
  "#4ade80",
  "#22c55e",
  "#15803d",
  "#14532d",
  "#facc15",
  "#fde68a",
  "#22d3ee",
  "#7dd3fc",
  "#38bdf8",
  "#bfdbfe",
  "#1d4ed8",
  "#1e3a8a",
  "#c4b5fd",
  "#8b5cf6",
  "#6d28d9",
  "#c084fc",
];
/** #5012: UNMIGRATED's ceiling; it only falls. */
const UNMIGRATED_MAX = 867;

/** The CSS named colors (CSS Color 4), which count as colors too. */
const NAMED =
  "aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred papayawhip peru pink plum powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen";
const LITERAL = new RegExp(
  String.raw`#[0-9a-f]{3,8}(?![\w-])|\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(|(?<![\w.#@-])(?:${NAMED.replaceAll(" ", "|")})(?![\w(-])`,
  "gi",
);

function colorLiterals(css: string): string[] {
  const code = css
    .replace(/\/\*[\s\S]*?\*\//g, " ")
    .replace(/url\([^)]*\)/g, "url()")
    .replace(/"[^"\n]*"|'[^'\n]*'/g, '""');
  return [...code.matchAll(LITERAL)].map((match) => match[0]);
}

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

const SHEETS = cssFiles(SRC).map((path) => relative(SRC, path).replaceAll("\\", "/")).sort();
const read = (sheet: string): string => readFileSync(join(SRC, sheet), "utf8");

/** The board's: not the deck builder's (a menu) nor table.css's playmats (cosmetics, R1297000). */
const BOARD_SHEETS = SHEETS.filter(
  (sheet) =>
    sheet === "index.css" ||
    sheet === "buttons.css" ||
    (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

/** Every other sheet's own colors: one that loses some lowers its number, an art sheet that gains some raises it. */
const UNMIGRATED: Record<string, number> = {
  "auth/code-field.css": 2,
  "auth/consent.css": 3,
  "auth/tavern.css": 10,
  "cards/art/art.css": 7,
  "cards/cards.css": 186,
  "cards/cardstate.css": 15,
  "cards/cosmetics.css": 254,
  "cards/glitch.css": 11,
  "cards/inspect/inspect.css": 13,
  "cards/inspect/list-play.css": 2,
  "cards/keywords.css": 18,
  "cards/marks.css": 4,
  "cards/setmark.css": 3,
  "cards/wheel/wheel.css": 6,
  "collection/collection.css": 73,
  "economy/economy.css": 9,
  "emotes/emotes.css": 1,
  "fx/fx.css": 8,
  "game/deckbuilder/deckbuilder.css": 14,
  "game/deckbuilder/workshop.css": 3,
  "game/table.css": 176,
  "packs/pack-object.css": 15,
  "patches/patches.css": 1,
  "practice/practice.css": 2,
  "practice/sandbox.css": 2,
  "routes/landing.css": 12,
  "routes/lobby.css": 10,
  "routes/stats.css": 7,
};

/**
 * A token line's captures (#4009: `/^\s*--…$/gm` line patterns rescan newline runs from every line
 * start); `pattern` is the line without its `m` flag.
 */
function tokenLines(text: string, pattern: RegExp): [string, string][] {
  const out: [string, string][] = [];
  for (const line of text.split("\n")) {
    const match = pattern.exec(line);
    if (match !== null) out.push([match[1] ?? "", match[2] ?? ""]);
  }
  return out;
}

const PALETTE = new Map(tokenLines(read("palette.css"), /^\s*(--pal-[a-z0-9-]+):\s*(#[0-9a-f]{6});$/));

/** The hex of the palette color that `sheet` gives `token` (its first declaration). */
function resolved(sheet: string, token: string): string | undefined {
  const name = new RegExp(`${token}:\\s*var\\((--pal-[a-z0-9-]+)\\);`).exec(read(sheet))?.[1];
  return name === undefined ? undefined : PALETTE.get(name);
}

/** WCAG relative luminance of `hex`, laid at `alpha` over `over` when given. */
function luminance(hex: string, over?: string, alpha = 1): number {
  const channel = (at: number): number => {
    const own = parseInt(hex.slice(at, at + 2), 16);
    const under = over === undefined ? own : parseInt(over.slice(at, at + 2), 16);
    const c = (alpha * own + (1 - alpha) * under) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

function contrast(a: number, b: number): number {
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

describe("#2105 the palette", () => {
  it("#2105 palette.css names at most PALETTE_MAX colors, each once, as #rrggbb, and nothing else", () => {
    const text = read("palette.css").replace(/\/\*[\s\S]*?\*\//g, "");
    expect(PALETTE.size).toBe([...text.matchAll(/--pal-[a-z0-9-]+:/g)].length);
    expect(PALETTE.size).toBeGreaterThan(0);
    expect(PALETTE.size).toBeLessThanOrEqual(PALETTE_MAX);
    expect(new Set(PALETTE.values()).size, "two names for one color").toBe(PALETTE.size);
    // Token lines blanked (#4009: the `/^\s*--…$/gm` pattern rescans newline runs from every line).
    expect(
      text
        .split("\n")
        .map((line) => (/^\s*--pal-[a-z0-9-]+:\s*#[0-9a-f]{6};$/.test(line) ? "" : line))
        .join("\n")
        .replace(/\s+/g, ""),
    ).toBe(":root{}");
    expect(read("index.css")).toMatch(/^@import "\.\/fonts\.css";\n@import "\.\/palette\.css";\n/);
  });

  it("#2105 every var(--pal-…) names a palette color, and only palette.css declares one", () => {
    for (const sheet of SHEETS) {
      const css = read(sheet);
      for (const match of css.matchAll(/var\((--pal-[a-z0-9-]+)\)/g)) expect(PALETTE.has(match[1] ?? ""), `${sheet}: ${match[0]}`).toBe(true);
      if (sheet !== "palette.css") expect(css, sheet).not.toMatch(/--pal-[a-z0-9-]+\s*:/);
    }
  });

  it("R5012000 no palette colour is one of Tailwind's defaults", () => {
    expect(new Set(TAILWIND_DEFAULTS).size).toBe(25);
    expect([...PALETTE].filter(([, hex]) => TAILWIND_DEFAULTS.includes(hex)).map(([name]) => name)).toEqual([]);
  });

  it("#2105 the colors that mean something keep their colors", () => {
    const MEANING: [string, string, string][] = [
      ["index.css", "--you", "#3d7bd4"],
      ["index.css", "--opponent", "#c0453f"],
      ["index.css", "--legal", "#4ec98a"],
      ["index.css", "--selected", "#f0c04a"],
      ["index.css", "--radiant", "#ffd24a"],
      ["index.css", "--plague", "#8d6bb5"],
      ["index.css", "--grade", "#5f93c2"],
      ["index.css", "--mana", "#3d7bd4"],
      ["index.css", "--locked", "#8a8577"],
      ["buttons.css", "--primary-ink", "#2b1800"],
      ["buttons.css", "--danger", "#c0453f"],
      ["game/board.css", "--selected", "#8fb3d4"],
      ["game/prompt.css", "--selected", "#8fb3d4"],
      ["game/highlights.css", "--glow-ready", "#4ec98a"],
      ["game/highlights.css", "--glow-condition", "#f0c04a"],
      ["game/highlights.css", "--glow-counted", "#c3afd6"],
      ["game/highlights.css", "--glow-pareto", "#3fb6a8"],
    ];
    for (const [sheet, token, hex] of MEANING) expect(resolved(sheet, token), `${sheet} ${token}`).toBe(hex);
    const seats = ["--you", "--opponent", "--legal", "--selected", "--radiant"].map((token) => resolved("index.css", token));
    expect(new Set(seats).size).toBe(seats.length);
  });

  it("#2105 the text and the dimmed text are readable on the three backgrounds", () => {
    const text = resolved("index.css", "--text") ?? "";
    const dim = /--text-dim:\s*color-mix\(in srgb, var\((--pal-[a-z0-9-]+)\) (\d+)%, transparent\);/.exec(read("index.css"));
    expect(dim).not.toBeNull();
    for (const background of ["--bg", "--bg-raised", "--bg-sunken"]) {
      const under = resolved("index.css", background) ?? "";
      expect(contrast(luminance(text), luminance(under)), background).toBeGreaterThanOrEqual(AA_TEXT_CONTRAST);
      const dimmed = luminance(PALETTE.get(dim?.[1] ?? "") ?? "", under, Number(dim?.[2]) / 100);
      expect(contrast(dimmed, luminance(under)), `--text-dim on ${background}`).toBeGreaterThanOrEqual(AA_TEXT_CONTRAST);
    }
  });
});

describe("#2105 no one-off colors", () => {
  it("#2105 the board's stylesheets state no color but the palette's", () => {
    expect(BOARD_SHEETS.length).toBeGreaterThan(20);
    const found = BOARD_SHEETS.flatMap((sheet) => colorLiterals(read(sheet)).map((literal) => `${sheet}: ${literal}`));
    expect(found, "name a palette color (palette.css) instead").toEqual([]);
  });

  it("#2105 no other stylesheet gains a color of its own", () => {
    const counts: Record<string, number> = {};
    for (const sheet of SHEETS) {
      if (sheet === "palette.css" || BOARD_SHEETS.includes(sheet)) continue;
      const own = colorLiterals(read(sheet)).length;
      if (own > 0) counts[sheet] = own;
    }
    expect(counts, "take the color from palette.css; an art sheet raises its number in UNMIGRATED").toEqual(UNMIGRATED);
  });

  it("#5012 the unmigrated sheets hold at most UNMIGRATED_MAX colours between them", () => {
    expect(Object.values(UNMIGRATED).reduce((sum, count) => sum + count, 0)).toBeLessThanOrEqual(UNMIGRATED_MAX);
  });
});
