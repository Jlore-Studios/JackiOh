// #2109: the board's corners, stacking and neutral shadows come from one named surface scale, surfaces.css;
// no stylesheet gains a radius, a shadow or a layer of its own unless this file says so. Gradients stay only
// on card faces (src/cards/), the main button (--primary-face*), the table (game/table.css and board.css's
// `.field`) and the named texture keeps below. Every other stylesheet, the menus' included, is held to its
// count of them in UNTOKENED_SURFACES (#3224). #5015: no stylesheet but surfaces.css writes a border-radius length,
// the table's and the card faces' included, and the pill is held to the rules PILL_RULES lists.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

/** The surface scale's sizes: a new radius, shadow or layer raises its number in the same change. */
const SURFACE_RADII = 38;
const SURFACE_SHADOWS = 4;
const SURFACE_LAYERS = 21;

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
  (sheet) =>
    sheet === "index.css" ||
    sheet === "buttons.css" ||
    (sheet.startsWith("game/") && !sheet.startsWith("game/deckbuilder/") && sheet !== "game/table.css"),
);

/** Every other sheet's own shadows, layers and gradients, counted as the board's test finds them (card faces, the table and fx keep their gradients by design; #5015 keeps every radius out, so none is counted here): one that moves values onto surfaces.css lowers its number, and none gains one. */
const UNTOKENED_SURFACES: Record<string, number> = {
  "audio/audio.css": 1,
  "auth/code-field.css": 12,
  "auth/consent.css": 1,
  "auth/tavern.css": 14,
  "cards/art/art.css": 3,
  "cards/art/artLive.css": 3,
  "cards/cards.css": 145,
  "cards/cardstate.css": 16,
  "cards/cosmetics.css": 37,
  // R3344000: the glare's two radial gradients, over a card face's art and its frame, where a gradient means something.
  "cards/depth.css": 2,
  "cards/dormant.css": 5,
  "cards/glitch.css": 4,
  "cards/inspect/inspect.css": 15,
  "cards/inspect/list-play.css": 1,
  "cards/keywords.css": 15,
  "cards/marks.css": 4,
  "cards/setmark.css": 5,
  "cards/stamps.css": 1,
  "cards/wheel/wheel.css": 4,
  "collection/collection.css": 13,
  "collection/tokenOffer.css": 1,
  "duos/duos.css": 12,
  "economy/economy.css": 4,
  "economy/jlore-pill.css": 3,
  "emotes/emotes.css": 9,
  "emotes/portrait.css": 13,
  "fx/aura/aura.css": 3,
  "fx/aura/tile.css": 2,
  "fx/fx.css": 77,
  "game/deckbuilder/deckbuilder.css": 40,
  "game/deckbuilder/workshop.css": 10,
  "game/table.css": 35,
  "notfound/notfound.css": 2,
  "packs/pack-object.css": 6,
  "packs/packs.css": 5,
  "patches/patches.css": 8,
  "practice/practice.css": 40,
  "practice/sandbox.css": 2,
  "routes/dev-panel.css": 2,
  "routes/game-banner.css": 4,
  "routes/landing.css": 53,
  "routes/lobby.css": 12,
  "routes/stats.css": 1,
  "routes/top-bar.css": 3,
  "settings/settings.css": 10,
  "skeleton/skeleton.css": 2,
  "together/together.css": 1,
  "tutorial/tutorial.css": 22,
  "undo/undo.css": 2,
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

const RADII = tokenLines(read("surfaces.css"), /^\s*--radius-([\w-]+):\s*([^\s;][^;]*);$/);
const SHADOWS = tokenLines(read("surfaces.css"), /^\s*--shadow-([\w-]+):([^;]+);$/).map(([name, value]) => [
  name,
  value.trim().replace(/\s+/g, " "),
]);
const LAYERS = tokenLines(read("surfaces.css"), /^\s*--z-([a-z-]+):\s*(-?\d+);$/).map(([name, value]) => [name, Number(value)]);

/** The background gradients the board keeps. Everything else goes flat. */
const GRADIENT_KEEP = [
  /^linear-gradient\(\s*to top,\s*var\(--bg-sunken\) 0%,\s*var\(--bg-sunken\) 36%,/, // B45's burn front: its sunken lower part eats the card as it climbs
  /repeating-(linear|radial|conic)-gradient/, // the locked-zone and facedown hatch weaves
  /var\(--countered-bubble\)/, // R667's rising bubbles
  /var\(--drag-reticle-color\)/, // the drop reticle and its tick bars
  /\/\s*(\d+px\s+100%|100%\s+\d+px)\s+no-repeat/, // R514's cell-pick crosshair bars
  /calc\(50% [-+] 1\.5px\)/, // the countered badge's struck-circle strike
  /^radial-gradient\(ellipse at center, var\(--pal-black\) 0%, var\(--pal-slate-900\) 55%/, // R3554000's Black Hole, a dark well
];

/** The playmat and its marks: the tavern table is board.css's `.field`, and each playmat's tap marks and
 * sand cracks are its own cosmetic (SandSurface's TAP_FAMILY); these rules keep their gradients and shadows. */
const TABLE_RULE = /^\.(field|sand-mark(--\w+)?|tap-mark--\w+)$/;

/**
 * `css` without the table's own rules (#4009: `/([^{}]+)\{[^{}]*\}/g` rescans a brace-free run from
 * every position).
 */
function withoutTable(css: string): string {
  const kept: string[] = [];
  const chunks = css.split("}");
  const terminated = css.endsWith("}");
  chunks.forEach((chunk, index) => {
    if (index === chunks.length - 1 && !terminated) {
      kept.push(chunk);
      return;
    }
    const open = chunk.lastIndexOf("{");
    if (open === -1) {
      kept.push(chunk);
      return;
    }
    const before = chunk.slice(0, open);
    const selector = before.slice(before.lastIndexOf("{") + 1);
    const body = chunk.slice(open + 1);
    if (selector === "" || body.includes("{")) {
      kept.push(chunk);
      return;
    }
    const rule = `${selector}{${body}}`;
    const parts = selector.split(",").map((part) => part.trim());
    if (!parts.every((part) => TABLE_RULE.test(part))) kept.push(rule);
  });
  return kept.join("}");
}

/** A box-shadow layer that is a glow or a selection ring rather than a neutral shadow. */
const GLOW =
  /^(inset\s+)?0\s+0(\s+(\d+px|0)){1,2}\s+(var\(--(selected|legal|locked|mana|countered-rim|tap-tint|glow-ready|glow-condition|drag-reticle-color)\)|color-mix\(|rgba?\()/;
/** The fuse spark's thrown particles (clock.css): offset dots in ember, not a shadow with a blur. */
const SPARK_PARTICLE = /^-?[\d.]+px -?[\d.]+px 0( -?[\d.]+(px)?)? var\(--clock-ember/;

function offShadow(css: string): string[] {
  const found: string[] = [];
  for (const match of css.matchAll(/(?<![\w-])box-shadow\s*:\s*([^;]+)/g)) {
    const value = (match[1] ?? "").trim().replace(/\s+/g, " ");
    let depth = 0;
    let layer = "";
    const layers: string[] = [];
    for (const char of value) {
      if (char === "(") depth += 1;
      if (char === ")") depth -= 1;
      if (char === "," && depth === 0) {
        layers.push(layer);
        layer = "";
      } else {
        layer += char;
      }
    }
    layers.push(layer);
    for (const raw of layers) {
      const trimmed = raw.trim();
      if (trimmed === "" || trimmed === "none" || trimmed === "inherit") continue;
      if (trimmed.includes("var(--shadow-") || trimmed.includes("var(--primary-shadow")) continue;
      if (GLOW.test(trimmed) || SPARK_PARTICLE.test(trimmed)) continue;
      found.push(`box-shadow: ${trimmed}`);
    }
  }
  return found;
}

/** A border-radius declaration, its longhands (the logical ones too) included; group 1 is its value. */
const RADIUS_DECLARATION =
  /(?<![\w-])(?:border(?:-top|-bottom)?-(?:left|right)-radius|border-(?:start|end)-(?:start|end)-radius|border-radius)\s*:\s*([^;}]+)/g;

/** Every border-radius in `css` that is not 0, inherit or the surface scale's tokens (#5015). */
function rawRadii(css: string): string[] {
  const found: string[] = [];
  for (const match of css.matchAll(RADIUS_DECLARATION)) {
    const value = (match[1] ?? "").trim().replace(/\s+/g, " ");
    const rest = value.replace(/var\(--radius-[\w-]+\)/g, "").trim();
    if (value !== "inherit" && !/^[\s0/]*$/.test(rest)) found.push(`border-radius: ${value}`);
  }
  return found;
}

/** `<sheet>: <selector>` for every rule that takes --radius-pill (string methods only: no regex rescans a run). */
function pillRules(): string[] {
  const rules = new Set<string>();
  for (const sheet of SHEETS) {
    if (sheet === "surfaces.css") continue;
    const css = codeOf(sheet);
    for (const match of css.matchAll(RADIUS_DECLARATION)) {
      if (!(match[1] ?? "").includes("var(--radius-pill)")) continue;
      const before = css.slice(0, match.index);
      const open = before.lastIndexOf("{");
      const start = Math.max(before.lastIndexOf("}", open), before.lastIndexOf("{", open - 1), before.lastIndexOf(";", open)) + 1;
      rules.add(`${sheet}: ${before.slice(start, open).replace(/\s+/g, " ").trim()}`);
    }
  }
  return [...rules].sort();
}

/**
 * The rules allowed the pill (#5015), as `<sheet>: <selector>`: chips, badges and toggles, and the
 * effects' capsules. A new one joins the list in the same change; a control that stops being a pill
 * leaves it.
 */
const PILL_RULES: string[] = [
  "auth/tavern.css: .tavern.app-shell .account-status",
  "cards/cards.css: .cf .cf-sick",
  "cards/cards.css: .cf .stat",
  'cards/cardstate.css: .cf .cf-state[data-state="quest"]',
  'cards/cardstate.css: .cf .cf-state[data-state="questStage"]',
  "cards/cardstate.css: .cf .cf-states-more",
  "cards/fullscreen.css: .fullscreen-art-open",
  "cards/inspect/inspect.css: .inspect-close",
  "cards/inspect/inspect.css: .inspect-face-column .shortcut-hint",
  "cards/inspect/inspect.css: .inspect-list-badge",
  "cards/inspect/list-play.css: .inspect-list-play",
  'cards/keywords.css: .cf .kw-fx[data-keyword-fx="Lucky"]::after',
  "cards/tallies.css: .tally-badge",
  "collection/collection.css: .tavern.app-shell .collection-card__new",
  "daily/daily.css: .daily-badge",
  "daily/daily.css: .daily-clock",
  "dungeon/fight/fight.css: .artifact-tray__open",
  "dungeon/map/run.css: .dungeon-run-hud__relic",
  "duos/duos.css: .duos-ping-wheel",
  "economy/economy.css: .reward-reveal__item",
  "economy/jlore-pill.css: .jlore-pill.jlore-pill, .tavern.app-shell a.jlore-pill:not(.link-button):not(.button-primary):not(.button-secondary)",
  "emotes/emotes.css: .portrait-preview-toggle",
  "emotes/portrait.css: .hero-inspect-stat",
  'fx/fx.css: .fx-figure[data-figure="bambooCane"]::after',
  'fx/fx.css: .fx-figure[data-figure="bambooCane"]::before',
  'fx/fx.css: .fx-figure[data-figure="campus"]::before',
  'fx/fx.css: .fx-figure[data-figure="caneStreak"]::before',
  'fx/fx.css: .fx-figure[data-figure="chains"]::before, .fx-figure[data-figure="chains"]::after',
  'fx/fx.css: .fx-figure[data-figure="chalkTendril"]::after',
  'fx/fx.css: .fx-figure[data-figure="chalkboard"]::after',
  'fx/fx.css: .fx-figure[data-figure="cushion"]::before',
  'fx/fx.css: .fx-figure[data-figure="forgeShell"]::after',
  'fx/fx.css: .fx-figure[data-figure="forgeShell"]::before',
  'fx/fx.css: .fx-figure[data-figure="inkScholar"]::after',
  'fx/fx.css: .fx-figure[data-figure="ledge"]::after',
  'fx/fx.css: .fx-figure[data-figure="paperclip"]::after',
  'fx/fx.css: .fx-figure[data-figure="paperclip"]::before',
  'fx/fx.css: .fx-figure[data-figure="rod"]::before, .fx-figure[data-figure="rod"]::after',
  'fx/fx.css: .fx-figure[data-figure="scroll"]::after',
  'fx/fx.css: .fx-figure[data-figure="silkFan"]::after',
  'fx/fx.css: .fx-figure[data-figure="silkFan"]::before',
  'fx/fx.css: .fx-figure[data-figure="spirit"]::before',
  'fx/fx.css: .fx-figure[data-figure="turret"]::before',
  'fx/fx.css: .fx-roll-item[data-kind="coin"] .fx-roll-face',
  "fx/fx.css: .takeover-art-colossus, .takeover-art-bandlight",
  "fx/fx.css: .takeover-art-hall-cane",
  "fx/fx.css: .takeover-art-magma",
  "fx/fx.css: .takeover-art-rack-cane",
  "fx/fx.css: .takeover-art-strand",
  "game/activate.css: .board .activate-control",
  'game/animations.css: [data-testid="result-overlay"][data-form="chip"]',
  "game/board.css: .choices-truncated-badge",
  "game/board.css: .hero button.modifier-badge",
  "game/board.css: .hero-health, .hero-armor, .hero-jade, .hero-credits",
  "game/board.css: .modifier-badge",
  "game/board.css: .pile",
  "game/board.css: .power-button, .power-tag",
  "game/board.css: .power-used",
  "game/board.css: .stat",
  "game/board.css: .turn-mechanism-clock",
  'game/clock.css: .clock-line[data-urgency="final"], .clock-line[data-urgency="last10"]',
  "game/deckbuilder/deckbuilder.css: .db-deck-meter",
  "game/deckbuilder/deckbuilder.css: .db-deck-status",
  "game/deckbuilder/deckbuilder.css: .db-drag-ghost",
  "game/deckbuilder/deckbuilder.css: .db-held",
  "game/deckbuilder/deckbuilder.css: .db-new",
  "game/deckbuilder/deckbuilder.css: .db-search",
  "game/deckbuilder/deckbuilder.css: .deckbuilder.app-shell .db-chip",
  "game/deckbuilder/deckbuilder.css: .deckbuilder.app-shell .db-deck-fold",
  "game/deckbuilder/deckbuilder.css: .deckbuilder.app-shell .db-filter-toggle",
  "game/deckbuilder/workshop.css: .deckbuilder.app-shell .ws-back",
  "game/deckbuilder/workshop.css: .deckbuilder.app-shell .ws-chip-button",
  "game/deckbuilder/workshop.css: .deckbuilder.app-shell .ws-patch-badge",
  "game/deckbuilder/workshop.css: .deckbuilder.app-shell .ws-sync",
  "game/deckbuilder/workshop.css: .ws-chip",
  "game/dialog.css: .dialog__close",
  "game/drag/drag.css: .drag-ghost-attack, .drag-ghost-health",
  "game/facedown.css: .unrevealed-tag",
  "game/handHints.css: .hand-hint",
  "game/market.css: .board .marketplace-toggle",
  "game/modifierList.css: .modifier-list .modifier-list__chip",
  "game/overflow.css: .cancel-tag, .counter-tag",
  "game/overflow.css: .pile > .pile-fatigue",
  "game/overflow.css: .pile-notice-tag, .burn-tag",
  "game/prompt.css: .prompt .prompt-rewards > button::before",
  "game/prompt.css: .prompt-card-verdict",
  "game/prompt.css: .prompt-number-value",
  "game/prompt.css: .prompt[data-prompt-kind][data-board-pick]",
  "game/prompt.css: .prompt[data-prompt-kind][data-board-pick] > .prompt-actions button",
  'game/prompt.css: .prompt[data-prompt-layout="cards"][data-bare] > .prompt-actions button',
  'game/prompt.css: .prompt[data-prompt-layout="cards"][data-bare] > .prompt-answer-stop',
  "game/showcase/showcase.css: .showcase-caption",
  "game/showcase/showcase.css: .showcase-ordinal",
  "game/spent.css: .card-unit > .spent-badge",
  "game/starscape.css: .starscape-node",
  "patches/patches.css: .patch-retry",
  "patches/patches.css: .tavern.app-shell .patch-micro__title .patch-version",
  "patches/patches.css: .tavern.app-shell .patch__title .patch-version",
  "practice/practice.css: .practice .practice-hud__outcome",
  "practice/practice.css: .practice-thinking",
  "routes/dev-panel.css: .landing .landing-corner .landing-dev-button",
  "routes/entry-card.css: .entry-card__pill",
  "routes/entry-card.css: .entry-card__tag",
  "routes/landing.css: .landing .landing-news-count",
  "routes/lobby.css: .tavern.play-screen .play-mode-tile__waiting",
  "routes/lobby.css: .tavern.play-screen .play-trio-deck",
  "routes/play-hub.css: .tavern.play-screen.play-hub .play-card__fact",
  "routes/play-hub.css: .tavern.play-screen.play-hub .play-card__last",
  "routes/stats.css: .stats-current-patch",
  "routes/stats.css: .stats-version-live",
  "routes/top-bar.css: :is(.screen-nav, .landing-corner) .top-bar-button",
  "settings/settings.css: .settings-slider-value",
  "together/together.css: .together-badge",
  'tutorial/tutorial.css: .coach[data-coach-mode="discovery"] .coach__eyebrow',
  "tutorial/tutorial.css: .tutorial-lesson__chip",
];

function offSurface(css: string): string[] {
  const found: string[] = [];
  for (const match of css.matchAll(/(?<![\w-])z-index\s*:\s*([^;}]+)/g)) {
    const value = (match[1] ?? "").trim();
    if (value !== "auto" && !/^var\(--z-[\w-]+\)$/.test(value)) found.push(`z-index: ${value}`);
  }
  for (const match of css.matchAll(/(?<![\w-])background(?:-image)?\s*:\s*([^;]+)/g)) {
    const value = (match[1] ?? "").trim();
    if (value.includes("gradient(") && !GRADIENT_KEEP.some((keep) => keep.test(value)))
      found.push(`background: ${value.replace(/\s+/g, " ").slice(0, 120)}`);
  }
  return found;
}

describe("#2109 the surface scale", () => {
  it("#2109 #5015 surfaces.css names 38 radii, 4 shadows and 21 layers in order, and nothing else", () => {
    expect(RADII).toEqual([
      ["xs", "2px"],
      ["sm", "4px"],
      ["md", "6px"],
      ["lg", "10px"],
      ["pill", "999px"],
      ["round", "50%"],
      ["switch", "15px / 22px"],
      ["card", "7% / 5%"],
      ["card-scale", "6% / 4.3%"],
      ["card-minion", "12% / 9%"],
      ["card-minion-scale", "10.7% / 8%"],
      ["card-edge", "10% / 7%"],
      ["art-portrait", "48% 48% 40% 40% / 55% 55% 45% 45%"],
      ["art-window", "6% / 8%"],
      ["art-arch", "50% 50% 4% 4% / 40% 40% 4% 4%"],
      ["card-gem", "0.6cqh"],
      ["card-text", "2.5cqw"],
      ["card-name", "2.2cqh"],
      ["card-name-core", "3.8cqh"],
      ["card-name-minion", "1.2cqh"],
      ["card-tag", "2.3cqh"],
      ["card-chip", "max(3px, 2cqmin)"],
      ["card-plate", "0.8cqmin"],
      ["card-armor", "20% 20% 45% 45%"],
      ["card-brittle", "18%"],
      ["card-inline", "0.2em"],
      ["card-kw-chip", "0.3em"],
      ["blob-a", "42% 58% 63% 37% / 41% 44% 56% 59%"],
      ["blob-b", "63% 37% 41% 59% / 58% 36% 64% 42%"],
      ["blob-c", "37% 63% 55% 45% / 35% 62% 38% 65%"],
      ["fog", "45% / 50%"],
      ["sweep", "6%"],
      ["flame", "50% 50% 45% 45% / 30% 30% 70% 70%"],
      ["trail", "50% 50% 0 0 / 20% 20% 0 0"],
      ["sand", "50% 43% 52% 45%"],
      ["parchment", "52% 46% 55% 44%"],
      ["petal", "80% 0"],
      ["leaf", "0 100%"],
    ]);
    expect(RADII.length).toBe(SURFACE_RADII);
    expect(SHADOWS.map((shadow) => shadow[0])).toEqual(["1", "2", "3", "inset-top"]);
    expect(SHADOWS.length).toBe(SURFACE_SHADOWS);
    const blurs = SHADOWS.slice(0, 3).map((shadow) => Number(/^\s*0 (\d+)px/.exec(shadow[1] ?? "")?.[1]));
    expect(blurs).toEqual([...blurs].sort((a, b) => a - b));
    expect(LAYERS).toEqual([
      ["below", -1],
      ["base", 0],
      ["content", 1],
      ["raised", 2],
      ["overlay", 3],
      ["hover", 4],
      ["popover", 5],
      ["overflow", 18],
      ["strip", 20],
      ["veil", 30],
      ["banner", 35],
      ["market", 36],
      ["clock", 37],
      ["showcase", 39],
      ["modal", 40],
      ["result", 45],
      ["spotlight", 50],
      ["aim", 59],
      ["drag", 60],
      ["toast", 70],
      ["dialog", 72],
    ]);
    expect(LAYERS.length).toBe(SURFACE_LAYERS);
    const text = read("surfaces.css").replace(/\/\*[\s\S]*?\*\//g, "");
    // Token lines blanked (#4009: the `/^\s*--…$/gm` patterns rescan newline runs from every line).
    const blanked = (line: string): string =>
      /^\s*--radius-[\w-]+:\s*[^\s;][^;]*;$/.test(line) ||
      /^\s*--shadow-[\w-]+:[^;]+;$/.test(line) ||
      /^\s*--z-[a-z-]+:\s*-?\d+;$/.test(line)
        ? ""
        : line;
    const rest = text
      .split("\n")
      .map(blanked)
      .join("\n");
    expect(rest.replace(/\s+/g, "")).toBe(":root{}");
  });

  it("#2109 index.css imports surfaces.css fourth; #5015 the old --radius alias is gone", () => {
    expect(read("index.css")).toMatch(
      /^@import "\.\/fonts\.css";\n@import "\.\/palette\.css";\n@import "\.\/typescale\.css";\n@import "\.\/surfaces\.css";\n/,
    );
    expect(codeOf("index.css")).not.toMatch(/--radius\s*:/);
    expect(read("game/board.css")).toMatch(/--seat-radius: var\(--radius-md\);/);
  });

  it("#2109 every var(--radius-*/--shadow-*/--z-*) names a token, and only surfaces.css declares one", () => {
    const names = new Set([
      ...RADII.map((radius) => `--radius-${radius[0]}`),
      ...SHADOWS.map((shadow) => `--shadow-${shadow[0]}`),
      ...LAYERS.map((layer) => `--z-${layer[0]}`),
    ]);
    // Outside board scope, and frozen: the inspect overlay's (src/cards/) own hover layer; R3221000 put its
    // modal one on --z-dialog. No sheet gains a name of its own; one that loses its names empties its entry.
    const FOREIGN: Record<string, string[]> = {
      "cards/inspect/inspect.css": ["--z-inspect-hover"],
    };
    for (const sheet of SHEETS) {
      const css = codeOf(sheet);
      const allowed = new Set([...names, ...(FOREIGN[sheet] ?? [])]);
      for (const match of css.matchAll(/var\((--(?:radius|shadow|z)-[\w-]+)\)/g))
        expect(allowed.has(match[1] ?? ""), `${sheet}: ${match[0]}`).toBe(true);
      const declared = [...css.matchAll(/--(?:radius|shadow|z)-[\w-]+\s*:/g)].map((match) => match[0].replace(/:\s*$/, ""));
      if (sheet === "surfaces.css") {
        expect(new Set(declared)).toEqual(names);
      } else if (sheet in FOREIGN) {
        expect(declared.sort()).toEqual([...(FOREIGN[sheet] ?? [])].sort());
      } else {
        expect(declared, sheet).toEqual([]);
      }
    }
  });
});

describe("#2109 no one-off surfaces", () => {
  it("#2109 the board's stylesheets state no radius, shadow, layer or background gradient of their own", () => {
    expect(BOARD_SHEETS.length).toBeGreaterThan(20);
    const found = BOARD_SHEETS.flatMap((sheet) =>
      [...offSurface(withoutTable(codeOf(sheet))), ...offShadow(withoutTable(codeOf(sheet)))].map((declaration) => `${sheet}: ${declaration}`),
    );
    expect(found, "name a radius, a shadow and a layer (surfaces.css) instead").toEqual([]);
    for (const sheet of BOARD_SHEETS) {
      expect(codeOf(sheet), sheet).not.toMatch(/var\(--radius\)/);
      expect(codeOf(sheet), sheet).not.toMatch(/var\(--seat-radius\)/);
    }
  });

  it("#2109 no other stylesheet gains a radius, a shadow, a layer or a gradient of its own", () => {
    const counts: Record<string, number> = {};
    for (const sheet of SHEETS) {
      if (BOARD_SHEETS.includes(sheet)) continue;
      const css = codeOf(sheet);
      const own = offSurface(css).length + offShadow(css).length;
      if (own > 0) counts[sheet] = own;
    }
    expect(counts, "name a shadow and a layer (surfaces.css) instead; a sheet that loses some lowers its number in UNTOKENED_SURFACES").toEqual(
      UNTOKENED_SURFACES,
    );
  });

  it("#2109 the board's components set none inline", () => {
    const components = files(join(SRC, "game"), ".tsx").filter((path) => !path.endsWith(".test.tsx"));
    expect(components.length).toBeGreaterThan(20);
    for (const path of components)
      expect(readFileSync(path, "utf8"), relative(SRC, path)).not.toMatch(/\b(?:borderRadius|boxShadow|zIndex)\b|gradient\(/);
  });
});

describe("#5015 one corner language", () => {
  it("#5015 the radius check passes 0, inherit and the tokens, and refuses any length, percent, fallback, calc or alias", () => {
    const passes = [
      ".a { border-radius: 0; }",
      ".a { border-radius: inherit; }",
      ".a { border-radius: var(--radius-sm) var(--radius-sm) 0 0; }",
      ".a { border-top-left-radius: var(--radius-pill); }",
      ".a { border-radius: var(--radius-card); }",
    ];
    for (const css of passes) expect(rawRadii(css), css).toEqual([]);
    const refused = [
      ".a { border-radius: 10px; }",
      ".a { border-radius: 50%; }",
      ".a { border-radius: var(--radius-sm, 4px); }",
      ".a { border-radius: calc(var(--radius-md) + 2px); }",
      ".a { border-radius: var(--radius); }",
      ".a { border-bottom-right-radius: 0.5rem; }",
      ".a { border-start-end-radius: 2cqh; }",
    ].join("\n");
    expect(rawRadii(refused)).toEqual([
      "border-radius: 10px",
      "border-radius: 50%",
      "border-radius: var(--radius-sm, 4px)",
      "border-radius: calc(var(--radius-md) + 2px)",
      "border-radius: var(--radius)",
      "border-radius: 0.5rem",
      "border-radius: 2cqh",
    ]);
  });

  it("#5015 no stylesheet but surfaces.css writes a border-radius length, the table's and the card faces' included", () => {
    expect(SHEETS.length).toBeGreaterThan(100);
    const found = SHEETS.filter((sheet) => sheet !== "surfaces.css").flatMap((sheet) =>
      rawRadii(codeOf(sheet)).map((declaration) => `${sheet}: ${declaration}`),
    );
    expect(found, "name a radius (surfaces.css) instead").toEqual([]);
  });

  it("#5015 only the rules in PILL_RULES take the pill", () => {
    expect(pillRules()).toEqual(PILL_RULES);
  });
});
