// #2645: the match menu, the control bar's ⋯ on a phone (`usePhoneBar`, board.css's two phone layouts).
//
// The bar keeps to one row there, so Offer draw, Concede and Friends are rows of this menu instead of
// buttons beside End turn. It decides nothing (CLAUDE.md rule 7): its rows are the same controls the
// desktop bar holds, built by Board.tsx, and it only shows or hides them.
//
// It stays mounted while closed (`hidden`), so a Friends panel opened from it survives the menu closing
// (the panel is the button's own state). A press on a row, on ⋯, Escape and Back (R2127000) close it, and
// the focus goes back to ⋯. The keys are `useModalKeys`'s, read only while it is open (R2126000: the
// board takes no key handler of its own).

import { useCallback, useId, useRef, type ReactElement, type ReactNode, type RefObject } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { useModalKeys } from "../together/modalKeys.ts";
import { testid } from "./contract.ts";

/** Escape and Back, for as long as the menu is open: rendered only then, so the keys are only then read. */
function MenuKeys({ menu, onClose }: { menu: RefObject<HTMLElement | null>; onClose: () => void }): null {
  useModalKeys(menu, onClose);
  return null;
}

export default function MatchMenu({
  open,
  onOpenChange,
  dot,
  glow,
  animating,
  children,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Offer draw has something to say (a declined offer blocks it): ⋯ wears a dot. */
  dot: boolean;
  /** The glow of the control a tutorial points at inside the menu, worn by ⋯ while the menu is closed. */
  glow?: string;
  /** The animation of a control inside the menu, worn by ⋯ likewise. */
  animating?: string;
  children: ReactNode;
}): ReactElement {
  useLocale();
  const menuId = useId();
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const close = useCallback((): void => {
    onOpenChange(false);
    button.current?.focus({ preventScroll: true });
  }, [onOpenChange]);

  return (
    <>
      <button
        ref={button}
        type="button"
        className="match-menu-toggle"
        data-testid={testid.matchMenu}
        aria-label={t("Match menu")}
        title={t("Match menu")}
        aria-expanded={open}
        aria-controls={menuId}
        data-dot={dot ? "true" : undefined}
        data-glow={open ? undefined : glow}
        data-animating={open ? undefined : animating}
        onClick={() => {
          onOpenChange(!open);
        }}
      >
        <span className="match-menu-toggle-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" width="20" height="20" focusable="false">
            <circle cx="5" cy="12" r="2" fill="currentColor" />
            <circle cx="12" cy="12" r="2" fill="currentColor" />
            <circle cx="19" cy="12" r="2" fill="currentColor" />
          </svg>
        </span>
      </button>
      <div
        ref={menu}
        id={menuId}
        className="match-menu"
        data-testid={testid.matchMenuPanel}
        role="group"
        aria-label={t("Match menu")}
        hidden={!open}
        onClick={(event) => {
          if (event.target instanceof Element && event.target.closest("button") !== null) close();
        }}
      >
        {children}
      </div>
      {open ? <MenuKeys menu={menu} onClose={close} /> : null}
    </>
  );
}
