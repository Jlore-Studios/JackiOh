// R3221000 (#3221): the menus' dialogs and sheets draw through the one Dialog (game/Dialog.tsx, R3220000).
// No source compares a key with "Escape" outside the shared hooks and the exceptions named here, no scrim
// outside dialog.css paints itself or takes a raw layer, the ✕ is DialogClose's alone and no × is a close,
// and every inline confirm puts its safe choice first.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

const SRC = dirname(fileURLToPath(import.meta.url));

function files(dir: string, keep: (name: string) => boolean): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" || name === "pkg" ? [] : files(path, keep);
    return keep(name) ? [path] : [];
  });
}

const rel = (path: string): string => relative(SRC, path).replaceAll("\\", "/");
const read = (file: string): string => readFileSync(join(SRC, file), "utf8");

const SOURCES = files(SRC, (name) => /\.tsx?$/.test(name) && !/\.test\./.test(name) && !name.endsWith(".d.ts"))
  .map(rel)
  .sort();
const SHEETS = files(SRC, (name) => name.endsWith(".css"))
  .map(rel)
  .sort();

const parsed = new Map<string, ts.SourceFile>();

/** A source's syntax tree, parsed once for every case that reads it. */
function parse(file: string): ts.SourceFile {
  let tree = parsed.get(file);
  if (tree === undefined) {
    tree = ts.createSourceFile(file, read(file), ts.ScriptTarget.Latest, true, file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
    parsed.set(file, tree);
  }
  return tree;
}

function walk(node: ts.Node, visit: (node: ts.Node) => void): void {
  visit(node);
  ts.forEachChild(node, (child) => {
    walk(child, visit);
  });
}

const EQUALITY = new Set([
  ts.SyntaxKind.EqualsEqualsEqualsToken,
  ts.SyntaxKind.ExclamationEqualsEqualsToken,
  ts.SyntaxKind.EqualsEqualsToken,
  ts.SyntaxKind.ExclamationEqualsToken,
]);

const isEscape = (node: ts.Node): boolean => ts.isStringLiteralLike(node) && node.text === "Escape";
const isKey = (node: ts.Node): boolean => ts.isPropertyAccessExpression(node) && node.name.text === "key";

/** A key compared with "Escape" (`event.key === "Escape"`, either way round), or a `case "Escape"`. */
function escapeHandlers(file: string): number {
  // A source that never writes the word holds no such comparison, and is not parsed.
  if (!read(file).includes("Escape")) return 0;
  let count = 0;
  walk(parse(file), (node) => {
    if (ts.isBinaryExpression(node) && EQUALITY.has(node.operatorToken.kind)) {
      if ((isKey(node.left) && isEscape(node.right)) || (isEscape(node.left) && isKey(node.right))) count += 1;
    } else if (ts.isCaseClause(node) && isEscape(node.expression)) {
      count += 1;
    }
  });
  return count;
}

/**
 * The sources that still read Escape themselves, each once: none of them is a dialog. A dialog's Escape is
 * `useModalKeys`'s (Dialog.tsx) or `useModalOverlay`'s (the inspect overlay and the pack scene).
 */
const ESCAPE_HANDLERS: Record<string, number> = {
  // The shared hooks: a modal's keys, the inspect overlay's, and the layers' guard that one Escape closes one.
  "together/modalKeys.ts": 1,
  "cards/inspect/store.ts": 1,
  "net/backLayers.ts": 1,
  // Tooltips and hover previews (R279, R2098000): Escape shuts the tip alone, and a dialog under it waits.
  "cards/TermTip.tsx": 1,
  "cards/CardRef.tsx": 1,
  "cards/inspect/useInspectTrigger.tsx": 1,
  "collection/LookHover.tsx": 1,
  "game/cardBar.ts": 1,
  // Popovers and menus that hang off a control, each a Back layer (R2127000) and none a modal: the page
  // under stays live, so no scrim, no Tab trap.
  "routes/landing.tsx": 1,
  "game/Marketplace.tsx": 1,
  "game/deckbuilder/DeckHistory.tsx": 1,
  "game/deckbuilder/DeckCosmeticsPicker.tsx": 1,
  "emotes/PortraitPicker.tsx": 1,
  "emotes/ui.tsx": 1,
  "game/ModifierList.tsx": 1,
  "dungeon/fight/ArtifactTray.tsx": 1,
  // The board's gestures: a drag or an aim Escape calls off, and the showcase Escape skips.
  "game/drag/DragLayer.tsx": 1,
  "game/drag/OptionDrag.tsx": 1,
  "game/showcase/CardShowcase.tsx": 1,
  // An inline question under no dialog, whose Escape answers Cancel.
  "dungeon/start/StartScreen.tsx": 1,
};

/** The board's own scrims, under the prompt and the result (R3220000 left them to the board). */
const BOARD_SCRIMS = ["prompt-scrim", "result-scrim"];

/** Every rule of a sheet, its comments cut, as `[selector, body]`; a rule inside `@media` counts as its own. */
function rules(css: string): [string, string][] {
  const code = css.replace(/\/\*[\s\S]*?\*\//g, " ");
  return [...code.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => [(match[1] ?? "").trim(), match[2] ?? ""]);
}

/** A glyph standing alone in an element, as a close's label would, or a string that is only the glyph. */
function glyphs(file: string, glyph: string): number {
  if (!read(file).includes(glyph)) return 0;
  let count = 0;
  walk(parse(file), (node) => {
    if (ts.isJsxText(node) && node.text.trim() === glyph) {
      const parent = node.parent;
      const alone =
        (ts.isJsxElement(parent) || ts.isJsxFragment(parent)) &&
        parent.children.every((child) => child === node || (ts.isJsxText(child) && child.text.trim() === ""));
      if (alone) count += 1;
    } else if (ts.isStringLiteralLike(node) && node.text === glyph) {
      count += 1;
    }
  });
  return count;
}

function countsOf(count: (file: string) => number): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const file of SOURCES) {
    const found = count(file);
    if (found > 0) counts[file] = found;
  }
  return counts;
}

/** The dialogs #3221 moved onto Dialog, each drawn through it. */
const MOVED = [
  "settings/SettingsPanel.tsx",
  "together/FriendsButton.tsx",
  "together/ChatPanel.tsx",
  "together/ChallengeDialog.tsx",
  "collection/TokenOffer.tsx",
  "routes/account.tsx",
  "routes/stats.tsx",
  "dungeon/stats/DungeonStats.tsx",
  "tutorial/FieldGuide.tsx",
  "patches/PatchNotes.tsx",
  "collection/ItemPreview.tsx",
  "collection/CardCosmeticsView.tsx",
  "collection/LooksScreen.tsx",
  "shop/CosmeticPacks.tsx",
  "routes/DevButton.tsx",
  "routes/CommandMenu.tsx",
  "tutorial/TutorialResult.tsx",
];

/** Every `className` string in a source. */
function classNames(file: string): string[] {
  const found: string[] = [];
  walk(parse(file), (node) => {
    if (ts.isJsxAttribute(node) && node.name.getText() === "className") {
      walk(node, (inner) => {
        if (ts.isStringLiteralLike(inner) || ts.isTemplateLiteralToken(inner)) found.push(inner.text);
      });
    }
  });
  return found;
}

describe("R3221000 the menus' dialogs on the one Dialog", () => {
  it("R3221000 no hand-rolled Escape handler is left outside the shared hooks", () => {
    expect(countsOf(escapeHandlers)).toEqual(ESCAPE_HANDLERS);
  });

  it("R3221000 no scrim sets a background or a raw z-index outside dialog.css", () => {
    const found: string[] = [];
    let scrims = 0;
    for (const sheet of SHEETS) {
      if (sheet === "game/dialog.css") continue;
      for (const [selector, body] of rules(read(sheet))) {
        const names = [...selector.matchAll(/\.([\w-]*scrim)\b/g)].map((match) => match[1] ?? "");
        if (!names.some((name) => !BOARD_SCRIMS.includes(name))) continue;
        scrims += 1;
        for (const match of body.matchAll(/(?<![\w-])(background(?:-[\w-]+)?|z-index)\s*:\s*([^;]+)/g)) {
          const [property, value] = [match[1] ?? "", (match[2] ?? "").trim()];
          if (property === "z-index" && /^var\(--z-[\w-]+\)$/.test(value)) continue;
          found.push(`${sheet}: ${selector} { ${property}: ${value} }`);
        }
      }
    }
    expect(scrims, "the rule reads the sheets' scrims").toBeGreaterThan(0);
    expect(found).toEqual([]);
  });

  it("R3221000 the ✕ is DialogClose's alone, and no × is a close", () => {
    expect(countsOf((file) => glyphs(file, "✕"))).toEqual({ "game/Dialog.tsx": 1 });
    // The craft editor's Remove-hat buttons and the deck editor's Stop-comparing chip: each removes a
    // thing from its row, named for it, and closes nothing.
    expect(countsOf((file) => glyphs(file, "×"))).toEqual({ "game/craft/CraftEditor.tsx": 2, "game/deckbuilder/DeckEditor.tsx": 1 });
  });

  it("R3221000 the shells' button sizes pass the ✕ over", () => {
    // Each outweighs `.dialog__close` (0,1,0) and would resize the ✕ of every dialog drawn in its shell.
    const SIZES: readonly (readonly [string, string])[] = [
      ["index.css", ".app-shell button"],
      ["auth/tavern.css", ".tavern.app-shell:where(:not(.deckbuilder)) button:where("],
    ];
    for (const [sheet, start] of SIZES) {
      const sized = rules(read(sheet)).filter(([selector, body]) => selector.startsWith(start) && /\bpadding\s*:/.test(body));
      expect(sized.length, sheet).toBe(1);
      expect(sized[0]?.[0], sheet).toContain(".dialog__close");
    }
  });

  it("R3221000 each listed dialog draws through Dialog.tsx", () => {
    for (const file of MOVED) {
      expect(read(file), file).toMatch(/<Dialog\b(?!Close)/);
      expect(
        classNames(file).filter((name) => name.includes("scrim")),
        file,
      ).toEqual([]);
    }
    // R3221000's exceptions with their own bodies: the inspect overlay's card and the pack scene take the ✕.
    for (const file of ["cards/inspect/CardDetail.tsx", "packs/PackOpening.tsx"]) expect(read(file), file).toContain("<DialogClose");
  });

  it("R3221000 every inline confirm puts the safe choice first", () => {
    // The questions that stay in their row, under no dialog: `[file, the safe answer's testid, the act's]`.
    const INLINE: readonly (readonly [string, string, string])[] = [
      ["together/FriendRows.tsx", "togetherTestid.keep}", "togetherTestid.confirm}"],
      ["routes/series.tsx", "seriesTestid.forfeitCancel}", "seriesTestid.forfeitConfirm}"],
      ["routes/QuestsPanel.tsx", "questsTestid.replaceCancel}", "questsTestid.replaceConfirm}"],
      ["dungeon/start/StartScreen.tsx", "dungeonTestid.abandonCancel}", "dungeonTestid.abandonConfirm}"],
      ["auth/MoveToExisting.tsx", "moveTestid.keep}", "moveTestid.confirm}"],
    ];
    for (const [file, safe, act] of INLINE) {
      const source = read(file);
      const [at, actAt] = [source.indexOf(`data-testid={${safe}`), source.indexOf(`data-testid={${act}`)];
      expect(at, `${file}: ${safe}`).toBeGreaterThan(-1);
      expect(actAt, `${file}: ${act}`).toBeGreaterThan(at);
    }
  });
});
