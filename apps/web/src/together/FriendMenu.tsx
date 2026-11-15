// A friend row's ⋯ menu (R3732002): what the row's one primary button leaves (`friendGroups.ts`'s
// `menuItemsOf`): Chat, Daily challenge, Invite to lobby, Watch, Remove and Block. It is a layer of its own,
// as `game/MatchMenu.tsx`'s is: Escape and Back (R2127000) close it alone and the focus goes back to ⋯, a
// press outside closes it, and a press on an item closes it, then acts. While it is open its root wears
// `data-tip-open`, which the dialog under it reads as "an open layer of mine": Escape closes the menu, never
// the drawer. It decides nothing (CLAUDE.md rule 7): the items are what the screen offers.

import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactElement,
  type RefObject,
} from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { type MenuItem } from "./friendGroups.ts";
import { useModalKeys } from "./modalKeys.ts";
import { togetherTestid } from "./testids.ts";

/** Escape and Back, for as long as the menu is open: rendered only then, so the keys are only then read. */
function MenuKeys({
  menu,
  first,
  onClose,
}: {
  menu: RefObject<HTMLElement | null>;
  first: RefObject<HTMLElement | null>;
  onClose: () => void;
}): null {
  useModalKeys(menu, onClose, first);
  return null;
}

/** The test id and the words of one item; each word is its own literal, so the translation guard sees it. */
function itemOf(item: MenuItem): { testid: string; label: string } {
  switch (item) {
    case "chat":
      return { testid: togetherTestid.chatOpen, label: t("Chat") };
    case "daily":
      return { testid: togetherTestid.daily, label: t("Daily challenge") };
    case "lobby":
      return { testid: togetherTestid.lobby, label: t("Invite to lobby") };
    case "watch":
      return { testid: togetherTestid.watchItem, label: t("Watch") };
    case "remove":
      return { testid: togetherTestid.remove, label: t("Remove") };
    case "block":
      return { testid: togetherTestid.block, label: t("Block") };
  }
}

export type FriendMenuProps = {
  /** The friend's name, for the button's label. */
  name: string;
  items: readonly MenuItem[];
  /** The unread messages from this friend: a badge on ⋯ and on Chat. */
  unread: number;
  busy: boolean;
  /** Runs after the menu has closed; `opener` is ⋯, which has the focus back once a dialog the item opens closes. */
  onItem: (item: MenuItem, opener: HTMLElement) => void;
};

export function FriendMenu({ name, items, unread, busy, onItem }: FriendMenuProps): ReactElement {
  useLocale();
  const [open, setOpen] = useState(false);
  const menuId = useId();
  const root = useRef<HTMLSpanElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLUListElement>(null);
  const first = useRef<HTMLButtonElement>(null);

  /** Closes the menu and, by default, gives ⋯ the focus, as the match menu does. */
  const close = useCallback((refocus = true): void => {
    setOpen(false);
    if (refocus) button.current?.focus({ preventScroll: true });
  }, []);

  // A press that begins outside the menu closes it, wherever it lands.
  useEffect(() => {
    if (!open) return;
    const onPress = (event: PointerEvent): void => {
      if (event.target instanceof Node && root.current?.contains(event.target) === true) return;
      close(false);
    };
    document.addEventListener("pointerdown", onPress, true);
    return () => {
      document.removeEventListener("pointerdown", onPress, true);
    };
  }, [open, close]);

  /** The arrow keys, Home and End move between the items, as a menu does. */
  const onKeyDown = (event: ReactKeyboardEvent<HTMLUListElement>): void => {
    const buttons = [...(menu.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not([disabled])') ?? [])];
    if (buttons.length === 0) return;
    const at = buttons.findIndex((item) => item === document.activeElement);
    let to: number;
    if (event.key === "ArrowDown") to = at === buttons.length - 1 ? 0 : at + 1;
    else if (event.key === "ArrowUp") to = at <= 0 ? buttons.length - 1 : at - 1;
    else if (event.key === "Home") to = 0;
    else if (event.key === "End") to = buttons.length - 1;
    else return;
    event.preventDefault();
    buttons[to]?.focus();
  };

  return (
    <span ref={root} className="friend-menu" data-tip-open={open ? "true" : undefined}>
      <button
        ref={button}
        type="button"
        className="friend-menu__button"
        data-testid={togetherTestid.menu}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        aria-label={t("More for {name}", { name })}
        title={t("More for {name}", { name })}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        <span aria-hidden="true">⋯</span>
        {unread > 0 ? (
          <span className="together-badge" data-testid={togetherTestid.menuUnread} aria-label={t("{count} unread", { count: unread })}>
            {unread}
          </span>
        ) : null}
      </button>
      {open ? (
        <>
          <ul ref={menu} id={menuId} className="friend-menu__list" role="menu" aria-label={t("More for {name}", { name })} onKeyDown={onKeyDown}>
            {items.map((item, index) => {
              const { testid, label } = itemOf(item);
              return (
                <li key={item} role="none">
                  <button
                    ref={index === 0 ? first : undefined}
                    type="button"
                    role="menuitem"
                    className={item === "remove" || item === "block" ? "friend-menu__item friend-menu__item--quiet" : "friend-menu__item"}
                    data-testid={testid}
                    disabled={busy && item !== "chat"}
                    onClick={() => {
                      // The menu is gone before the item acts: an item that opens a dialog finds nothing of ours open.
                      close(false);
                      onItem(item, button.current ?? root.current ?? document.body);
                    }}
                  >
                    {label}
                    {item === "chat" && unread > 0 ? (
                      <span className="together-badge" data-testid={togetherTestid.unread} aria-label={t("{count} unread", { count: unread })}>
                        {unread}
                      </span>
                    ) : null}
                  </button>
                </li>
              );
            })}
          </ul>
          <MenuKeys menu={menu} first={first} onClose={close} />
        </>
      ) : null}
    </span>
  );
}
