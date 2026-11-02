// R2126000: the client's own keyboard shortcuts, each named once, and how a hint writes them.
// Hints only (CLAUDE.md rule 7): nothing here listens for a key, and no shortcut is added.
import { INSPECT_KEY_SHORTCUT } from "../cards/inspect/keys.ts";

/** Keys in order: `MOD` is Ctrl, or ⌘ on an Apple device; the rest are `KeyboardEvent.key` values. */
export type Shortcut = readonly string[];
export const MOD = "Mod";

/** R2126000: every shortcut the client has. The command menu's has no control (R2118000). */
export const SHORTCUTS = {
  inspect: [INSPECT_KEY_SHORTCUT],
  undo: [MOD, "Z"],
  redo: [MOD, "Shift", "Z"],
  commandMenu: [MOD, "K"],
  replayBackStep: ["ArrowLeft"],
  replayForwardStep: ["ArrowRight"],
  replayBackTurn: ["Shift", "ArrowLeft"],
  replayForwardTurn: ["Shift", "ArrowRight"],
} as const satisfies Readonly<Record<string, Shortcut>>;

/** R2126000: a touch-only device, which shows no hint. */
export const TOUCH_ONLY_QUERY = "(hover: none) and (pointer: coarse)";

export type ShortcutHints = { readonly shown: boolean; readonly apple: boolean };

const ARROWS = { ArrowLeft: "←", ArrowRight: "→", ArrowUp: "↑", ArrowDown: "↓", Escape: "Esc" };
const APPLE_LABELS: Readonly<Record<string, string>> = { ...ARROWS, [MOD]: "⌘", Shift: "⇧" };
const OTHER_LABELS: Readonly<Record<string, string>> = { ...ARROWS, [MOD]: "Ctrl", Shift: "Shift" };

export function isApplePlatform(userAgent: string): boolean {
  return /Mac|iPhone|iPad|iPod/u.test(userAgent);
}

export function applePlatform(): boolean {
  return typeof navigator !== "undefined" && isApplePlatform(navigator.userAgent);
}

/** How a hint writes `keys`: "⌘⇧Z" on an Apple device, "Ctrl+Shift+Z" elsewhere. */
export function shortcutText(keys: Shortcut, apple: boolean): string {
  const labels = apple ? APPLE_LABELS : OTHER_LABELS;
  return keys.map((key) => labels[key] ?? key).join(apple ? "" : "+");
}

/** `aria-keyshortcuts` for `keys`, `MOD` as both Control and Meta. */
export function ariaKeyshortcuts(keys: Shortcut): string {
  if (!keys.includes(MOD)) return keys.join("+");
  return ["Control", "Meta"].map((mod) => keys.map((key) => (key === MOD ? mod : key)).join("+")).join(" ");
}

/** A tooltip naming its shortcut, or the bare label where hints are hidden. */
export function shortcutTitle(label: string, keys: Shortcut, hints: ShortcutHints): string {
  return hints.shown ? `${label} (${shortcutText(keys, hints.apple)})` : label;
}

function touchQuery(): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  try {
    return window.matchMedia(TOUCH_ONLY_QUERY);
  } catch {
    return null;
  }
}

/** A host without `matchMedia` is not touch-only. */
export function touchOnly(): boolean {
  return touchQuery()?.matches === true;
}

export function subscribeTouchOnly(onChange: () => void): () => void {
  const query = touchQuery();
  if (query === null || typeof query.addEventListener !== "function") return () => undefined;
  query.addEventListener("change", onChange);
  return () => {
    query.removeEventListener("change", onChange);
  };
}
