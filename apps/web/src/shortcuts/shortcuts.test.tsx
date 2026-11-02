// R2126000: the client's own keyboard shortcuts are shown on keyboard devices, ⌘ or Ctrl by platform,
// wherever a hint goes; a touch-only device shows no hint, and no key handler is added on a board route.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ts from "typescript";

import { INSPECT_HOVER, INSPECT_SHEET, closeInspect } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/constants.ts";
import { testid } from "../game/contract.ts";
import { ReplayToolbar, replayTestid } from "../replay/ReplayToolbar.tsx";
import { SiteFooter, siteFooterTestid } from "../routes/SiteFooter.tsx";
import { renderBoard } from "../test/board.tsx";
import { fullBoardView } from "../test/fixtures.ts";
import { KBD_TESTID, Kbd, SHORTCUT_HINT_TESTID, ShortcutHint } from "./Kbd.tsx";
import { SHORTCUTS, TOUCH_ONLY_QUERY, ariaKeyshortcuts, isApplePlatform, shortcutText } from "./shortcuts.ts";

const MAC = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";
const IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
const WINDOWS = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";
const ANDROID = "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Mobile Safari/537.36";

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

/** A touch-only device: only `TOUCH_ONLY_QUERY` matches. */
function touchOnlyDevice(): void {
  vi.spyOn(window, "matchMedia").mockImplementation(
    (query: string) =>
      ({
        media: query,
        matches: query === TOUCH_ONLY_QUERY,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList,
  );
}

function toolbar(): void {
  render(
    <ReplayToolbar at={2} last={5} playing={false} onPlaying={() => undefined} onGoTo={() => undefined} onTurn={() => undefined} readout="" />,
  );
}

let enemyId = "";

/** Hovers the first opponent unit on a full board and returns its preview. */
function hoverPreview(): HTMLElement {
  vi.useFakeTimers();
  const view = fullBoardView();
  renderBoard(view);
  const enemy = view.opponent.units.find((unit) => unit !== null);
  if (enemy === null || enemy === undefined) throw new Error("no enemy unit");
  enemyId = enemy.instanceId;
  const el = screen.getByTestId(testid.card(enemy.instanceId));
  expect(el.getAttribute("aria-keyshortcuts")).toBe("I");
  fireEvent.pointerEnter(el, { pointerType: "mouse" });
  act(() => {
    vi.advanceTimersByTime(HOVER_DELAY_MS);
  });
  return screen.getByTestId(INSPECT_HOVER);
}

describe("R2126000 shortcut hints", () => {
  it("R2126000 a shortcut reads ⌘ on an Apple device and Ctrl elsewhere", () => {
    expect(isApplePlatform(MAC)).toBe(true);
    expect(isApplePlatform(IPHONE)).toBe(true);
    expect(isApplePlatform(WINDOWS)).toBe(false);
    expect(isApplePlatform(ANDROID)).toBe(false);
    expect(isApplePlatform(navigator.userAgent)).toBe(false);

    expect(shortcutText(SHORTCUTS.commandMenu, true)).toBe("⌘K");
    expect(shortcutText(SHORTCUTS.commandMenu, false)).toBe("Ctrl+K");
    expect(shortcutText(SHORTCUTS.redo, true)).toBe("⌘⇧Z");
    expect(shortcutText(SHORTCUTS.redo, false)).toBe("Ctrl+Shift+Z");
    expect(shortcutText(SHORTCUTS.inspect, true)).toBe("I");
    expect(shortcutText(SHORTCUTS.inspect, false)).toBe("I");
    expect(shortcutText(SHORTCUTS.replayBackTurn, false)).toBe("Shift+←");

    render(<Kbd keys={SHORTCUTS.commandMenu} />);
    expect(screen.getByTestId(KBD_TESTID).textContent).toBe("Ctrl+K");
    cleanup();

    vi.spyOn(navigator, "userAgent", "get").mockReturnValue(MAC);
    render(<Kbd keys={SHORTCUTS.commandMenu} />);
    expect(screen.getByTestId(KBD_TESTID).textContent).toBe("⌘K");
  });

  it("R2126000 aria-keyshortcuts writes Ctrl/Cmd as Control and Meta", () => {
    expect(ariaKeyshortcuts(SHORTCUTS.inspect)).toBe("I");
    expect(ariaKeyshortcuts(SHORTCUTS.undo)).toBe("Control+Z Meta+Z");
    expect(ariaKeyshortcuts(SHORTCUTS.redo)).toBe("Control+Shift+Z Meta+Shift+Z");
    expect(ariaKeyshortcuts(SHORTCUTS.commandMenu)).toBe("Control+K Meta+K");
    expect(ariaKeyshortcuts(SHORTCUTS.replayBackStep)).toBe("ArrowLeft");
    expect(ariaKeyshortcuts(SHORTCUTS.replayForwardTurn)).toBe("Shift+ArrowRight");
  });

  it("R2126000 the replay's step and turn buttons name their keys in their tooltips and aria-keyshortcuts", () => {
    toolbar();
    const rows = [
      [replayTestid.backTurn, "Back a turn (Shift+←)", "Shift+ArrowLeft"],
      [replayTestid.backStep, "Back a step (←)", "ArrowLeft"],
      [replayTestid.forwardStep, "Forward a step (→)", "ArrowRight"],
      [replayTestid.forwardTurn, "Forward a turn (Shift+→)", "Shift+ArrowRight"],
    ] as const;
    for (const [id, title, keys] of rows) {
      const button = screen.getByTestId(id);
      expect(button.getAttribute("title"), id).toBe(title);
      expect(button.getAttribute("aria-keyshortcuts"), id).toBe(keys);
    }
    for (const id of [replayTestid.start, replayTestid.play, replayTestid.end]) {
      expect(screen.getByTestId(id).hasAttribute("aria-keyshortcuts"), id).toBe(false);
    }
  });

  it("R2126000 the inspect control's tooltip, a board card's hover preview, shows I", () => {
    const preview = hoverPreview();
    const inside = within(preview);
    expect(inside.getByTestId(KBD_TESTID).textContent).toBe("I");
    expect(inside.getByTestId(SHORTCUT_HINT_TESTID).textContent).toBe("Focus the card, then press I to inspect");
  });

  it("R2126000 the hint's key opens the sheet from the focused card, and does nothing from the hovered one", () => {
    const preview = hoverPreview();
    expect(preview).toBeTruthy();
    // No window handler answers I, so the hint asks for focus: a hovered card alone ignores the key.
    fireEvent.keyDown(document.body, { key: "i" });
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
    fireEvent.keyDown(screen.getByTestId(testid.card(enemyId)), { key: "i" });
    expect(screen.getByTestId(INSPECT_SHEET)).toBeTruthy();
  });

  it("R2126000 the site footer says Ctrl/Cmd+K opens the command menu", () => {
    render(<SiteFooter />);
    expect(screen.getByTestId(siteFooterTestid.shortcut).textContent).toBe("Press Ctrl+K for the command menu");
  });

  it("R2126000 hints are hidden under a touch-only media query", () => {
    touchOnlyDevice();

    render(
      <>
        <Kbd keys={SHORTCUTS.commandMenu} />
        <ShortcutHint keys={[SHORTCUTS.inspect]} words="Focus the card, then press {keys} to inspect" />
      </>,
    );
    expect(screen.queryByTestId(KBD_TESTID)).toBeNull();
    expect(screen.queryByTestId(SHORTCUT_HINT_TESTID)).toBeNull();
    cleanup();

    render(<SiteFooter />);
    expect(screen.queryByTestId(siteFooterTestid.shortcut)).toBeNull();
    cleanup();

    toolbar();
    const back = screen.getByTestId(replayTestid.backStep);
    expect(back.getAttribute("title")).toBe("Back a step");
    expect(back.getAttribute("aria-keyshortcuts")).toBe("ArrowLeft");
    cleanup();

    const preview = hoverPreview();
    expect(within(preview).queryByTestId(KBD_TESTID)).toBeNull();
    expect(within(preview).queryByTestId(SHORTCUT_HINT_TESTID)).toBeNull();
  });
});

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

/** Sources that draw a board, or that a board's routes mount (the deck builder's menu screens aside). */
const BOARD_PREFIXES = ["cards/", "duos/", "replay/", "practice/", "tutorial/", "emotes/", "shortcuts/"];
const BOARD_FILES = ["routes/match.tsx", "routes/series.tsx", "routes/dev/hotseat.tsx", "together/WatchRoute.tsx"];
const KEY_PROP = /^onKey(Down|Up|Press)(Capture)?$/u;
const KEY_EVENTS = ["keydown", "keyup", "keypress"];

/**
 * R2126000: each board-route source's key handlers (an `addEventListener` of a key event, or an `onKey*`
 * prop or entry). A hint adds none, so a new shortcut fails this test; a count is never raised to pass it.
 */
const BOARD_KEY_HANDLERS: Readonly<Record<string, number>> = {
  "cards/CardRef.tsx": 2,
  "cards/TermTip.tsx": 1,
  "cards/inspect/store.ts": 3,
  "cards/inspect/useInspectTrigger.tsx": 1,
  "cards/wheel/CardWheel.tsx": 1,
  "duos/DuosReplay.tsx": 1,
  "emotes/PortraitPicker.tsx": 1,
  "emotes/ui.tsx": 1,
  "game/ActivateControl.tsx": 1,
  "game/Board.tsx": 1,
  "game/Card.tsx": 1,
  "game/ConfirmConcede.tsx": 1,
  "game/Hero.tsx": 2,
  "game/MarketPrompt.tsx": 1,
  "game/Marketplace.tsx": 1,
  "game/ModifierList.tsx": 1,
  "game/Prompt.tsx": 4,
  "game/Zone.tsx": 1,
  "game/cardBar.ts": 1,
  "game/craft/CraftEditor.tsx": 1,
  "game/drag/DragLayer.tsx": 1,
  "game/drag/OptionDrag.tsx": 1,
  "game/showcase/CardShowcase.tsx": 1,
  "practice/PracticeLeave.tsx": 1,
  "practice/PracticeResult.tsx": 1,
  "replay/ReplayPlayer.tsx": 1,
  "tutorial/TutorialResult.tsx": 1,
};

function boardSources(dir = SRC): string[] {
  const found: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      if (name !== "test" && name !== "node_modules") found.push(...boardSources(path));
      continue;
    }
    if (!/\.tsx?$/u.test(name) || /\.test\.tsx?$/u.test(name) || name.endsWith(".d.ts")) continue;
    const rel = relative(SRC, path).split("\\").join("/");
    const inGame = rel.startsWith("game/") && !rel.startsWith("game/deckbuilder/");
    if (inGame || BOARD_PREFIXES.some((prefix) => rel.startsWith(prefix)) || BOARD_FILES.includes(rel)) found.push(path);
  }
  return found;
}

function keyHandlerCount(path: string): number {
  const file = ts.createSourceFile(
    path,
    readFileSync(path, "utf8"),
    ts.ScriptTarget.Latest,
    true,
    path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  let count = 0;
  const visit = (node: ts.Node): void => {
    if (
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression) &&
      node.expression.name.text === "addEventListener"
    ) {
      const first = node.arguments[0];
      if (first !== undefined && ts.isStringLiteralLike(first) && KEY_EVENTS.includes(first.text)) count += 1;
    } else if (ts.isJsxAttribute(node) && KEY_PROP.test(node.name.getText(file))) {
      count += 1;
    } else if ((ts.isPropertyAssignment(node) || ts.isShorthandPropertyAssignment(node)) && KEY_PROP.test(node.name.getText(file))) {
      count += 1;
    }
    ts.forEachChild(node, visit);
  };
  visit(file);
  return count;
}

describe("R2126000 no new key handler", () => {
  it("R2126000 no new key handler is added on a board route", () => {
    const counts: Record<string, number> = {};
    for (const path of boardSources()) {
      const count = keyHandlerCount(path);
      if (count > 0) counts[relative(SRC, path).split("\\").join("/")] = count;
    }
    expect(Object.keys(BOARD_KEY_HANDLERS).length).toBeGreaterThan(20);
    expect(counts).toEqual(BOARD_KEY_HANDLERS);
  });
});
