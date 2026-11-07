// #3227: every stylesheet's `@media` width and height, and every script's size query, is one of breakpoints.ts's
// named breakpoints, or a size its sheet keeps with its reason in KEEP; no max N sits beside a min N.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { BREAKPOINT_PX, PHONE_SHEET_QUERY, over, upTo, type Breakpoint } from "./breakpoints.ts";
import { HERO_DESKTOP_QUERY, HERO_TABLET_QUERY, PHONE_BAR_QUERY, PHONE_PORTRAIT_QUERY } from "./game/config.ts";
import { PATCH_LEGEND_PHONE_QUERY } from "./patches/constants.ts";
import { DEV_PANEL_HIDDEN_QUERY, DEV_PANEL_MIN_WIDTH_PX } from "./routes/devPanel.ts";
import { TOUCH_ONLY_QUERY } from "./shortcuts/shortcuts.ts";
import { COACH_DOCK_QUERY } from "./tutorial/config.ts";

const SRC = dirname(fileURLToPath(import.meta.url));

function files(dir: string, keep: (name: string) => boolean, skipDirs: readonly string[]): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return skipDirs.includes(name) ? [] : files(path, keep, skipDirs);
    return keep(name) ? [path] : [];
  });
}

const relativeToSrc = (path: string): string => relative(SRC, path).replaceAll("\\", "/");
const read = (file: string): string => readFileSync(join(SRC, file), "utf8");
/** The file with its block comments cut out. */
const code = (file: string): string => read(file).replace(/\/\*[\s\S]*?\*\//g, " ");

const SHEETS = files(SRC, (name) => name.endsWith(".css"), ["test"]).map(relativeToSrc).sort();

const SCRIPTS = files(
  SRC,
  (name) => /\.tsx?$/.test(name) && !/\.test\./.test(name) && !name.endsWith(".d.ts") && name !== "breakpoints.ts",
  ["test", "pkg"],
)
  .map(relativeToSrc)
  .sort();

/** Every `@media` prelude in a sheet, its whitespace collapsed. */
function preludes(css: string): string[] {
  return [...css.matchAll(/@media\s+([^{;]+)\{/g)].map((match) => (match[1] ?? "").replace(/\s+/g, " ").trim());
}

/** Every `(min-|max-)(width|height)` feature in a query, as `(max-width: 600px)`. */
function sizes(query: string): string[] {
  return [...query.matchAll(/\(\s*((?:min|max)-(?:width|height))\s*:\s*([^)]*?)\s*\)/g)].map(
    (match) => `(${match[1] ?? ""}: ${match[2] ?? ""})`,
  );
}

const NAMES = Object.keys(BREAKPOINT_PX) as Breakpoint[];
const NAMED = new Set(NAMES.flatMap((name) => [upTo(name), over(name)]));

/** #3227: the sizes a sheet keeps, in source order; the nearest breakpoint would move a layout. */
const KEEP: Record<string, string[]> = {
  // Tightens on 360 and 375 px phones; 390 to 430 keep the phone's spacing.
  "auth/code-field.css": ["(max-width: 400px)", "(max-width: 400px)"],
  "auth/tavern.css": ["(max-width: 400px)"],
  // 900: up to 900 px tall (1280x900 too) the info column stands beside the faces (B29). 480: a phone upright, never on its side (568+).
  "cards/inspect/inspect.css": ["(max-height: 900px)", "(max-width: 480px)"],
  // 840: a phone 390x844 and taller has height to spare in its lanes, so the field keeps a strip above and below for the corner props (R3348001); 390x664 and 360x640 have none.
  "game/arena/arena.css": ["(min-height: 840px)"],
  // The control bar tightens where it ran off 360 and 375 px phones (B29); 390 keeps its gaps.
  "game/board.css": ["(max-width: 380px)"],
  // The filters fold under about 860 px tall, where three chip rows left one row of cards.
  "game/deckbuilder/deckbuilder.css": ["(max-height: 860px)"],
  // Rail, pool and sidebar side by side need 1101 px (DeckWorkshop.tsx's WORKSHOP_SPLIT_MIN_WIDTH_PX).
  "game/deckbuilder/workshop.css": ["(min-width: 1101px)", "(max-width: 1100px)"],
  // A dialog's answers stack on the narrowest phones only (R3220000).
  "game/dialog.css": ["(max-width: 360px)"],
  // 480: 21vh cards two by two suit a phone upright; at 600 a 568x320 phone draws them 67 px wide. 701: 224 px Discover cards.
  "game/prompt.css": ["(max-width: 480px)", "(max-width: 480px)", "(min-height: 701px)"],
  // 820: Start sat on a laptop's fold up to 820 px tall. 420: buttons stack up to 412 px; 430 keeps a row.
  "practice/practice.css": ["(max-height: 820px)", "(max-width: 420px)"],
  // #1577: the hero shrinks up to 800 px tall (1280x720, 1280x800); 390x844 keeps it whole.
  "routes/landing.css": ["(max-height: 800px)", "(max-height: 800px)"],
  // A row stacks up to 412 px; 430 keeps it.
  "routes/replays.css": ["(max-width: 420px)"],
  // 820: one column up to an iPad Air upright; 760 squeezes 768 px, 899 takes 844x390. 480: the sheet is for a phone upright.
  "together/together.css": ["(max-width: 820px)", "(max-width: 480px)"],
  // The coach's lines, measured at phones' heights: 2 up to 679 px (390x664), 1 up to 655 (360x640).
  "tutorial/tutorial.css": ["(max-height: 679px)", "(max-height: 655px)"],
};

/** The scripts' size queries, each built from breakpoints.ts. */
const SCRIPT_QUERIES: Record<string, string> = {
  PHONE_BAR_QUERY,
  PHONE_PORTRAIT_QUERY,
  HERO_DESKTOP_QUERY,
  HERO_TABLET_QUERY,
  COACH_DOCK_QUERY,
  PATCH_LEGEND_PHONE_QUERY,
  DEV_PANEL_HIDDEN_QUERY,
  TOUCH_ONLY_QUERY,
};

/** The scripts that spell a size query of their own: the workshop's split mirrors workshop.css's 1100 px. */
const SCRIPT_KEEP: Record<string, number> = { "game/deckbuilder/DeckWorkshop.tsx": 1 };

describe("#3227 the named breakpoints", () => {
  it("#3227 names five breakpoints, each a max N beside a min N + 1", () => {
    expect(BREAKPOINT_PX).toEqual({ phone: 600, narrow: 760, medium: 899, tablet: 1024, short: 500 });
    expect(upTo("phone")).toBe("(max-width: 600px)");
    expect(over("phone")).toBe("(min-width: 601px)");
    expect(over("tablet")).toBe("(min-width: 1025px)");
    expect(upTo("short")).toBe("(max-height: 500px)");
    expect(over("short")).toBe("(min-height: 501px)");
    expect(PHONE_SHEET_QUERY).toBe("(max-width: 600px), (orientation: landscape) and (max-height: 500px)");
  });

  it("#3227 every @media width and height in src/'s stylesheets is a named breakpoint or KEEP's", () => {
    expect(SHEETS.length).toBeGreaterThan(0);
    for (const sheet of SHEETS) {
      const own = preludes(code(sheet))
        .flatMap(sizes)
        .filter((size) => !NAMED.has(size));
      expect(own, sheet).toEqual(KEEP[sheet] ?? []);
    }
    for (const sheet of Object.keys(KEEP)) expect(SHEETS, sheet).toContain(sheet);
  });

  it("#3227 a size query is written max- or min- in px, never in range syntax", () => {
    for (const sheet of SHEETS) {
      for (const prelude of preludes(code(sheet))) {
        expect(prelude, sheet).not.toMatch(/\b(?:width|height)\s*[<>]/);
        for (const size of sizes(prelude)) expect(/: (.*)\)$/.exec(size)?.[1], `${sheet}: ${size}`).toMatch(/^\d+px$/);
      }
    }
  });

  it("#3227 no max-width N beside a min-width N, nor max-height N beside min-height N", () => {
    const all = [...SHEETS.flatMap((sheet) => preludes(code(sheet)).flatMap(sizes)), ...Object.values(SCRIPT_QUERIES).flatMap(sizes)];
    expect(all.length).toBeGreaterThan(100);
    for (const axis of ["width", "height"]) {
      const valuesOf = (bound: string): Set<string> =>
        new Set(all.flatMap((size) => (size.startsWith(`(${bound}-${axis}: `) ? [size.slice(`(${bound}-${axis}: `.length, -1)] : [])));
      const mins = valuesOf("min");
      const both = [...valuesOf("max")].filter((value) => mins.has(value));
      expect(both, axis).toEqual([]);
    }
  });

  it("#3227 the scripts' queries are the named breakpoints'", () => {
    expect(PHONE_BAR_QUERY).toBe(PHONE_SHEET_QUERY);
    expect(COACH_DOCK_QUERY).toBe(PHONE_SHEET_QUERY);
    expect(PHONE_PORTRAIT_QUERY).toBe("(max-width: 600px)");
    expect(HERO_DESKTOP_QUERY).toBe("(min-width: 1025px)");
    expect(HERO_TABLET_QUERY).toBe("(min-width: 761px) and (max-width: 1024px)");
    expect(PATCH_LEGEND_PHONE_QUERY).toBe("(max-width: 760px)");
    expect(DEV_PANEL_HIDDEN_QUERY).toBe("(max-width: 1024px), (pointer: coarse)");
    expect(DEV_PANEL_MIN_WIDTH_PX).toBe(1025);
    expect(TOUCH_ONLY_QUERY).toBe("(hover: none) and (pointer: coarse)");
    for (const [name, query] of Object.entries(SCRIPT_QUERIES)) {
      for (const size of sizes(query)) expect(NAMED, `${name}: ${size}`).toContain(size);
    }
  });

  it("#3227 no script but breakpoints.ts spells a width or height query, but the workshop's split", () => {
    expect(SCRIPTS.length).toBeGreaterThan(0);
    const spelled: Record<string, number> = {};
    for (const script of SCRIPTS) {
      const text = code(script).replace(/^\s*\/\/.*$/gm, "");
      const count = [...text.matchAll(/\((?:min|max)-(?:width|height)\s*:/g)].length;
      if (count > 0) spelled[script] = count;
    }
    expect(spelled).toEqual(SCRIPT_KEEP);
    expect(read("game/deckbuilder/DeckWorkshop.tsx")).toContain("WORKSHOP_SPLIT_MIN_WIDTH_PX = 1101;");
    const workshop = read("game/deckbuilder/workshop.css");
    expect(workshop).toContain("@media (max-width: 1100px) {");
    expect(workshop).toContain("@media (min-width: 1101px) {");
  });
});
