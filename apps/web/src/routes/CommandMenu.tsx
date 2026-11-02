// R2118000: Ctrl+K (Cmd+K) opens a command menu on every menu screen, never over a board.
// Presentation only (CLAUDE.md rule 7): every entry is a `navigate` move (`commandEntries.ts`) and
// it reads only the public catalog. It needs a keyboard, so no touch entry point; no animation.
// R2126000: on a keyboard device the selected entry shows Enter and the foot names the keys.

import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactElement,
} from "react";

import { useBoardShown } from "../game/boardShown.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { navigate } from "../net/navigate.ts";
import { Kbd, ShortcutHint, useShortcutHints } from "../shortcuts/Kbd.tsx";
import { useModalKeys } from "../together/modalKeys.ts";
import { commandEntries, commandMenuOn, type CommandEntry } from "./commandEntries.ts";
import "./command-menu.css";

export const commandMenuTestid = {
  scrim: "command-menu-scrim",
  dialog: "command-menu",
  input: "command-menu-input",
  close: "command-menu-close",
  list: "command-menu-list",
  empty: "command-menu-empty",
  keys: "command-menu-keys",
} as const;

/** R2126000: the keys the menu answers to (R2118000), named at its foot; hints only. */
export const COMMAND_MENU_KEYS = { move: [["ArrowUp"], ["ArrowDown"]], open: ["Enter"], close: ["Escape"] } as const;

export function commandEntryTestid(id: string): string {
  return `command-menu-entry-${id}`;
}

/**
 * R2118000: whether a key press opens the menu: Ctrl+K or Cmd+K, with no other modifier, both on
 * every platform (no platform sniffing).
 */
export function isCommandMenuKey(event: {
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
  readonly key: string;
}): boolean {
  return (event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "k";
}

/** A modal that is not this menu: while one is open the shortcut stays out of its way. */
const OTHER_MODAL = `[aria-modal="true"]:not([data-testid="${commandMenuTestid.dialog}"])`;

/** R2118000: the shell's command menu on `path`: nothing on a board route or over a board. */
export default function CommandMenu({ path }: { path: string }): ReactElement | null {
  const boardShown = useBoardShown();
  if (!commandMenuOn(path) || boardShown) return null;
  return <CommandMenuKeys />;
}

function CommandMenuKeys(): ReactElement | null {
  const [open, setOpen] = useState(false);
  const close = useCallback(() => {
    setOpen(false);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent): void => {
      if (!isCommandMenuKey(event)) return;
      if (document.querySelector(OTHER_MODAL) !== null) return;
      event.preventDefault();
      setOpen(true);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  if (!open) return null;
  return <CommandMenuDialog onClose={close} />;
}

function kindLabel(kind: CommandEntry["kind"]): string {
  if (kind === "action") return t("Action");
  if (kind === "page") return t("Page");
  return t("Card");
}

function CommandMenuDialog({ onClose }: { onClose: () => void }): ReactElement {
  useLocale();
  const hints = useShortcutHints();
  const listId = useId();
  const dialog = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);

  const shown = commandEntries(query);
  const current = shown.length === 0 ? -1 : Math.min(selected, shown.length - 1);
  const optionId = (index: number): string => `${listId}-${String(index)}`;

  useLayoutEffect(() => {
    const opener = document.activeElement;
    input.current?.focus({ preventScroll: true });
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);
  // Escape closes it and Tab stays inside it.
  useModalKeys(dialog, onClose);
  useEffect(() => {
    if (current < 0) return;
    document.getElementById(`${listId}-${String(current)}`)?.scrollIntoView?.({ block: "nearest" });
  }, [current, listId]);

  const run = (entry: CommandEntry): void => {
    onClose();
    navigate(entry.path);
  };

  const onInputKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>): void => {
    if (event.nativeEvent.isComposing) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (shown.length === 0) return;
      const step = event.key === "ArrowDown" ? 1 : -1;
      setSelected((current + step + shown.length) % shown.length);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const entry = shown[current];
      if (entry !== undefined) run(entry);
    }
  };

  return (
    <div
      className="command-menu-scrim"
      data-testid={commandMenuTestid.scrim}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={dialog}
        className="command-menu"
        data-testid={commandMenuTestid.dialog}
        role="dialog"
        aria-modal="true"
        aria-label={t("Command menu")}
      >
        <input
          ref={input}
          className="command-menu__input"
          data-testid={commandMenuTestid.input}
          role="combobox"
          aria-expanded="true"
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={current === -1 ? undefined : optionId(current)}
          aria-label={t("Search pages, actions and cards")}
          placeholder={t("Search pages, actions and cards")}
          autoComplete="off"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setSelected(0);
          }}
          onKeyDown={onInputKeyDown}
        />
        <button type="button" className="command-menu__close" data-testid={commandMenuTestid.close} onClick={onClose}>
          {t("Close")}
        </button>
        {shown.length === 0 ? (
          <p className="command-menu__empty" data-testid={commandMenuTestid.empty}>
            {t("No matches")}
          </p>
        ) : (
          <ul id={listId} className="command-menu__list" data-testid={commandMenuTestid.list} role="listbox">
            {shown.map((entry, index) => (
              <li
                key={entry.id}
                id={optionId(index)}
                className="command-menu__option"
                role="option"
                aria-selected={index === current}
                data-testid={commandEntryTestid(entry.id)}
                onClick={() => {
                  run(entry);
                }}
              >
                <span className="command-menu__label">{entry.label}</span>
                <span className="command-menu__kind">
                  {index === current ? (
                    <span className="command-menu__enter" aria-hidden="true">
                      <Kbd keys={COMMAND_MENU_KEYS.open} />
                    </span>
                  ) : null}
                  {kindLabel(entry.kind)}
                </span>
              </li>
            ))}
          </ul>
        )}
        {hints.shown ? (
          <p className="command-menu__keys" data-testid={commandMenuTestid.keys}>
            <ShortcutHint keys={COMMAND_MENU_KEYS.move} words={t("{keys} to move")} />
            <ShortcutHint keys={[COMMAND_MENU_KEYS.open]} words={t("{keys} to open")} />
            <ShortcutHint keys={[COMMAND_MENU_KEYS.close]} words={t("{keys} to close")} />
          </p>
        ) : null}
      </div>
    </div>
  );
}
