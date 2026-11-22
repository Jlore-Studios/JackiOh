// The Friends button (#1831): on every screen a signed-in account has, a match included
// (`placement`: the menus' shell's navigation, R3728000; `routes/nav.tsx`'s top bar off the shell; a
// board's control bar; a phone's match menu, #2645), so friends are one press away. Its badge reads
// what the shell's one poll published (`socialPoll.ts`), so the button adds no request of its own.
//
// #4444: the drawer it opens (the friends panel, the chat panel, the challenge dialog) arrives
// lazily through `friendsDrawerChunk.ts`, prefetched on rail intent and shown behind the
// skeleton. Data fetching is untouched (poll while open, socket reads). R3221000: it is the one
// Dialog's, a sheet: the focus stays inside and returns to this button. R3728001: opened from the
// shell's navigation it is the Dialog's drawer instead, down the right edge, and the screen stays
// where it is. R3732002: what it holds is the list `FriendList.tsx` draws: the requests, the friends
// in groups, one primary button and a ⋯ menu on each row; Chat opens as a drawer too, in the list's place.
//
// CLAUDE.md rule 7: nothing here is a rule. Which friend can be challenged or watched is the server's
// status; Challenge is off while the player is in a game only to say what the server would, and the
// server still refuses it. Every refusal is shown in the server's words.

import { Suspense, useCallback, useId, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { t, useLocale } from "../i18n/i18n.ts";
import { readSession } from "../net/session.ts";
import type { PlayerRef } from "../net/social.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { friendsDrawerChunk, prefetchFriendsDrawer } from "./friendsDrawerChunk.ts";
import type { Challenging } from "./FriendRows.tsx";
import { friendsBadgeCount, reloadSocial, useSocialReads } from "./socialPoll.ts";
import { friendsTestid } from "./testids.ts";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "./together.css";
import "../routes/top-bar.css";

/** Two figures, side by side, on a 24-unit square, filled in `currentColor` so every skin's button color applies. */
export function FriendsIcon(): ReactElement {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" focusable="false">
      <circle cx="9" cy="8" r="3.2" fill="currentColor" />
      <path d="M2.5 19.5c0-3.6 2.9-6 6.5-6s6.5 2.4 6.5 6z" fill="currentColor" />
      <circle cx="17" cy="9" r="2.6" fill="currentColor" />
      <path d="M16.6 13.6c3 0 5.4 1.9 5.4 4.9h-4.6c0-1.9-.5-3.5-1.5-4.6z" fill="currentColor" />
    </svg>
  );
}

export type FriendsButtonProps = {
  /** Where it sits: a board's control bar, the top bar, a phone's match menu, the menus' shell (R3728000). */
  placement: "game" | "nav" | "menu" | "rail";
  /** What stands in its place while it has no friends to show (the shell's link to Play together). */
  fallback?: ReactElement | null;
};

export function FriendsButton({ placement, fallback = null }: FriendsButtonProps): ReactElement | null {
  useLocale();
  const reads = useSocialReads();
  const panelId = useId();
  const button = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [chatWith, setChatWith] = useState<PlayerRef | null>(null);
  const [challenging, setChallenging] = useState<Challenging | null>(null);

  /** Gives the focus back to the button, as a modal should leave it (WAI-ARIA APG; `SettingsButton.tsx`). */
  const refocus = useCallback((): void => {
    const element = button.current;
    if (element === null || !element.isConnected) return;
    // #2645: in a phone's match menu, closed by now (the press that opened the panel closed it): the
    // control that opens that menu is the one to land on.
    const closed = element.closest<HTMLElement>("[hidden]");
    const opener = closed === null || closed.id === "" ? null : document.querySelector<HTMLElement>(`[aria-controls="${closed.id}"]`);
    (opener ?? element).focus({ preventScroll: true });
  }, []);

  const closePanel = useCallback((): void => {
    setOpen(false);
    refocus();
  }, [refocus]);

  const closeChat = useCallback((): void => {
    setChatWith(null);
    reloadSocial();
    refocus();
  }, [refocus]);

  const closeChallenge = useCallback((): void => {
    setChallenging(null);
    refocus();
  }, [refocus]);

  const sentChallenge = useCallback((): void => {
    setChallenging(null);
    reloadSocial();
    refocus();
  }, [refocus]);

  // Signed out, an account the server refuses, or no read of the challenges yet: no friends to show.
  if (readSession() === null || reads.challengeReads === 0 || reads.refused) return fallback;

  const count = friendsBadgeCount(reads);
  // The top bar's round control (routes/top-bar.css), the shell's item (shell/menu-shell.css); the board's
  // control bar and the match menu draw their own.
  const bar = placement === "nav" ? " top-bar-button top-bar-button--round" : placement === "rail" ? " menu-shell__item" : "";
  const rail = placement === "rail";

  return (
    <>
      <button
        ref={button}
        type="button"
        className={`friends-button friends-button--${placement}${bar}`}
        data-testid={friendsTestid.button}
        aria-label={count > 0 ? t("Friends, {count} waiting for you", { count }) : t("Friends")}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        title={t("Friends")}
        onClick={() => {
          setOpen(true);
        }}
        onPointerEnter={prefetchFriendsDrawer}
        onFocus={prefetchFriendsDrawer}
        onTouchStart={prefetchFriendsDrawer}
      >
        <span aria-hidden="true" className={rail ? "friends-button__icon menu-shell__icon" : "friends-button__icon"}>
          <FriendsIcon />
        </span>
        {placement === "menu" ? <span className="friends-button__label">{t("Friends")}</span> : null}
        {rail ? <span className="menu-shell__label">{t("Friends")}</span> : null}
        {count > 0 ? (
          <span className="together-badge friends-button__badge" data-testid={friendsTestid.badge} aria-hidden="true">
            {count}
          </span>
        ) : null}
      </button>
      {open || chatWith !== null || challenging !== null
        ? createPortal(
            <Suspense fallback={<Skeleton variant="rows" label={t("Loading your friends…")} />}>
              <friendsDrawerChunk.Screen
                id={panelId}
                drawer={rail}
                unread={reads.unread}
                open={open}
                chatWith={chatWith}
                challenging={challenging}
                onClosePanel={closePanel}
                onChat={(friend) => {
                  setOpen(false);
                  setChatWith(friend);
                }}
                onChallenge={(friend) => {
                  setOpen(false);
                  setChallenging(friend);
                }}
                onCloseChat={closeChat}
                onCloseChallenge={closeChallenge}
                onSentChallenge={sentChallenge}
              />
            </Suspense>,
            document.body,
          )
        : null}
    </>
  );
}
