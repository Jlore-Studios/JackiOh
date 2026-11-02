// R2126000: a shortcut drawn as keys, and a sentence around some; nothing on a touch-only device.
import { Fragment, useSyncExternalStore } from "react";
import type { ReactElement } from "react";
import { applePlatform, shortcutText, subscribeTouchOnly, touchOnly, type Shortcut, type ShortcutHints } from "./shortcuts.ts";
import "./kbd.css";

export const KBD_TESTID = "kbd";
export const SHORTCUT_HINT_TESTID = "shortcut-hint";

/** Where a sentence puts its shortcuts. */
const KEYS_SLOT = "{keys}";

/** Whether hints show now, kept current by the media query, and whether to write ⌘ for Ctrl. */
export function useShortcutHints(): ShortcutHints {
  const shown = useSyncExternalStore(subscribeTouchOnly, () => !touchOnly(), () => true);
  return { shown, apple: applePlatform() };
}

/** A keycap for `keys`; nothing where hints are hidden. */
export function Kbd({ keys }: { keys: Shortcut }): ReactElement | null {
  const hints = useShortcutHints();
  if (!hints.shown) return null;
  return (
    <kbd className="kbd" data-testid={KBD_TESTID}>
      {shortcutText(keys, hints.apple)}
    </kbd>
  );
}

/** `words` with its shortcuts drawn at `{keys}`; nothing where hints are hidden. */
export function ShortcutHint({
  keys,
  words,
  testid = SHORTCUT_HINT_TESTID,
}: {
  keys: readonly Shortcut[];
  words: string;
  testid?: string;
}): ReactElement | null {
  const hints = useShortcutHints();
  if (!hints.shown) return null;
  const [before = "", after = ""] = words.split(KEYS_SLOT);
  return (
    <span className="shortcut-hint" data-testid={testid}>
      {before}
      {keys.map((shortcut, index) => (
        <Fragment key={shortcut.join("+")}>
          {index > 0 ? " " : null}
          <Kbd keys={shortcut} />
        </Fragment>
      ))}
      {after}
    </span>
  );
}
