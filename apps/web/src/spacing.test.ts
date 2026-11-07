// #3225: the board's gaps, paddings and margins take their lengths from one scale, spacing.css; no
// stylesheet gains a fixed length of its own unless this file says so.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

/** A spacing declaration: gap, padding, margin (and their longhands), and the `--gap` the board's rows read. */
const SPACING =
  /(?<![\w-])((?:grid-)?(?:row-|column-)?gap|padding(?:-(?:top|right|bottom|left|inline|block)(?:-(?:start|end))?)?|margin(?:-(?:top|right|bottom|left|inline|block)(?:-(?:start|end))?)?|--gap)\s*:\s*([^;}]+)/g;
/** A length of the sheet's own: a px or rem number that is not 0. An em, %, viewport or container term, or an env(), is relative. */
const FIXED = /(?<![\w.])(\d*\.?\d+)(?:px|rem)\b/g;

/** Every fixed length a gap, padding or margin states, as `property: length (value)`. */
function rawSpacing(css: string): string[] {
  const code = css.replace(/\/\*[\s\S]*?\*\//g, " ");
  const found: string[] = [];
  for (const match of code.matchAll(SPACING)) {
    const value = (match[2] ?? "").trim().replace(/\s+/g, " ");
    for (const length of value.matchAll(FIXED)) if (Number(length[1]) !== 0) found.push(`${match[1] ?? ""}: ${length[0]} (${value})`);
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
const codeOf = (sheet: string): string => read(sheet).replace(/\/\*[\s\S]*?\*\//g, " ");

/** The board's stylesheets, as palette.test.ts has them. */
const BOARD_SHEETS = SHEETS.filter(
  (sheet) => sheet === "index.css" || (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

/** Every other sheet's own fixed spacing: one that loses some lowers its number, and none gains one. */
const UNMIGRATED: Record<string, number> = {
  "audio/audio.css": 12,
  "auth/auth.css": 8,
  "auth/code-field.css": 29,
  "auth/consent.css": 17,
  "auth/tavern.css": 66,
  "auth/two-step.css": 4,
  "cards/cards.css": 10,
  "cards/inspect/inspect.css": 95,
  "cards/inspect/list-play.css": 4,
  "cards/keywords.css": 5,
  "cards/marks.css": 2,
  "cards/tallies.css": 3,
  "cards/wheel/wheel.css": 13,
  "collection/collection.css": 44,
  "collection/tokenOffer.css": 5,
  "dungeon/dungeon.css": 14,
  "dungeon/fight/fight.css": 11,
  "dungeon/map/run.css": 37,
  "dungeon/stats/dungeon-stats.css": 23,
  "duos/duos.css": 60,
  "economy/economy.css": 21,
  "economy/jlore-pill.css": 3,
  "economy/xp.css": 7,
  "emotes/emotes.css": 32,
  "emotes/portrait.css": 10,
  "empty/empty.css": 4,
  "game/deckbuilder/deckbuilder.css": 86,
  "game/deckbuilder/workshop.css": 87,
  "i18n/language.css": 3,
  "notfound/notfound.css": 3,
  "packs/pack-object.css": 2,
  "packs/packs.css": 11,
  "patches/patches.css": 97,
  "practice/practice.css": 145,
  "practice/sandbox.css": 13,
  "rank/rankCrest.css": 2,
  "rank/streakFlame.css": 1,
  "replay/replay.css": 2,
  "routes/account.css": 11,
  "routes/command-menu.css": 11,
  "routes/dev-panel.css": 7,
  "routes/entry-card.css": 9,
  "routes/game-banner.css": 9,
  "routes/landing.css": 54,
  "routes/leaderboard.css": 3,
  "routes/lobby.css": 129,
  "routes/privacy.css": 4,
  "routes/purchases.css": 2,
  "routes/replays.css": 5,
  "routes/site-footer.css": 5,
  "routes/stats.css": 78,
  "routes/top-bar.css": 9,
  "settings/changeEmail.css": 4,
  "settings/settings.css": 43,
  "shop/shop.css": 17,
  "shortcuts/kbd.css": 1,
  "sideline/sideline.css": 7,
  "skeleton/skeleton.css": 5,
  "stats/tier.css": 2,
  "together/together.css": 50,
  "tutorial/path-visibility.css": 12,
  "tutorial/tutorial.css": 115,
  "undo/undo.css": 3,
};

const STEPS = [...read("spacing.css").matchAll(/^\s*--space-(\d+):\s*(\d+)px;$/gm)].map((match) => ({ name: Number(match[1]), px: Number(match[2]) }));

describe("#3225 the spacing scale", () => {
  it("#3225 spacing.css names eleven lengths in px, smallest first, as --space-1 up, and nothing else", () => {
    expect(STEPS.map((step) => step.px)).toEqual([2, 4, 6, 8, 10, 12, 16, 20, 24, 32, 40]);
    expect(STEPS.map((step) => step.name)).toEqual(STEPS.map((_, i) => i + 1));
    const rest = codeOf("spacing.css").replace(/^\s*--space-\d+:\s*\d+px;$/gm, "");
    expect(rest.replace(/\s+/g, "")).toBe(":root{}");
    expect(read("index.css")).toMatch(
      /^@import "\.\/fonts\.css";\n@import "\.\/palette\.css";\n@import "\.\/typescale\.css";\n@import "\.\/surfaces\.css";\n@import "\.\/motion\.css";\n@import "\.\/spacing\.css";\n/,
    );
  });

  it("#3225 every var(--space-…) names a step, and only spacing.css declares one", () => {
    const names = new Set(STEPS.map((step) => `--space-${step.name}`));
    for (const sheet of SHEETS) {
      const code = codeOf(sheet);
      for (const match of code.matchAll(/var\((--space-[\w-]+)\)/g)) expect(names.has(match[1] ?? ""), `${sheet}: ${match[0]}`).toBe(true);
      if (sheet !== "spacing.css") expect(code, sheet).not.toMatch(/--space-[\w-]+\s*:/);
    }
  });

  it("#3225 the checker counts a px or rem length in a gap, padding or margin, and nothing else", () => {
    const css = `
      a { padding: 6px var(--space-2) 0 0.2em 4%; margin: -4px 0 0 -4px; gap: calc(var(--space-2) * -1) 1.5rem; --gap: 4px; }
      b { padding: max(0px, env(safe-area-inset-top)) 3vh; margin-inline: auto; gap: var(--gap); width: 9px; padding-bottom: calc(env(x) + 12px) }
    `;
    expect(rawSpacing(css).map((entry) => entry.split(" (")[0])).toEqual([
      "padding: 6px",
      "margin: 4px",
      "margin: 4px",
      "gap: 1.5rem",
      "--gap: 4px",
      "padding-bottom: 12px",
    ]);
  });
});

describe("#3225 no one-off spacing", () => {
  it("#3225 the board's stylesheets give every gap, padding and margin a step, zero, auto, or a relative or env() term", () => {
    expect(BOARD_SHEETS.length).toBeGreaterThan(20);
    const found = BOARD_SHEETS.flatMap((sheet) => rawSpacing(read(sheet)).map((entry) => `${sheet}: ${entry}`));
    expect(found, "name a step (spacing.css) instead, or compose one from steps").toEqual([]);
  });

  it("#3225 no other stylesheet gains a fixed spacing of its own", () => {
    const counts: Record<string, number> = {};
    for (const sheet of SHEETS) {
      if (sheet === "spacing.css" || BOARD_SHEETS.includes(sheet)) continue;
      const own = rawSpacing(read(sheet)).length;
      if (own > 0) counts[sheet] = own;
    }
    expect(counts, "take the length from spacing.css; a sheet that loses some lowers its number in UNMIGRATED").toEqual(UNMIGRATED);
  });
});
