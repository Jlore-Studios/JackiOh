// Inner screens need a visible exit when direct links have no useful browser Back history.

import type { MouseEvent, ReactElement, ReactNode } from "react";

import JlorePill from "../economy/JlorePill.tsx";
import LanguageButton from "../i18n/LanguageButton.tsx";
import { InboxBell } from "../inbox/InboxBell.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { navigate, paths } from "../net/navigate.ts";
import { SettingsButton } from "../settings/index.ts";
import { useMenuShell } from "../shell/context.ts";
import { FriendsButton } from "../together/FriendsButton.tsx";
import { ArrowIcon } from "./ArrowIcon.tsx";
import { PageTitle } from "./PageTitle.tsx";
import "./top-bar.css";

export const navTestid = {
  back: "nav-back",
  account: "nav-account",
} as const;

export type BackLinkProps = {
  to?: string;
  label?: string;
  /** Releases screen-owned state before navigation. */
  onLeave?: () => void;
  /** Replaces release-and-navigate on a guarded exit; drops the Jlore pill, a way past the guard (R1651000). */
  onPress?: () => void;
  /** The screen's name, drawn after Back as its one `h1` (PageTitle, #3223); the bar adds none. */
  title?: string;
  /** The screen's own extras, drawn before the Jlore pill; settings remains the top-right control. */
  children?: ReactNode;
  /** Back's test id; the replay player has its own. */
  testId?: string;
  /** The board supplies settings separately. */
  showSettings?: boolean;
  /** The Jlore pill; off on signed-out screens, where a session only lands as the screen leaves. */
  showWallet?: boolean;
};

/**
 * The top bar's one layout (R3218000, routes/top-bar.css): Back, the title and the extras, a flexible
 * spacer, then the end group (the Jlore pill, Friends, language, the gear), the gear always last.
 * Inside the menus' shell (R3728000, R3728002) Friends is the shell's navigation's and the inbox's bell
 * (R3729002) stands after the pill, and the bar ends with the shell's status row, under the rest. Uses
 * `navigate`, not history, so direct links stay in the app.
 */
export function BackLink({
  to = paths.landing,
  label,
  onLeave,
  onPress,
  title,
  children,
  testId = navTestid.back,
  showSettings = true,
  showWallet = true,
}: BackLinkProps): ReactElement {
  useLocale();
  const shell = useMenuShell();
  return (
    <nav className="row screen-nav">
      <button
        type="button"
        className="link-button top-bar-button top-bar-button--text"
        data-testid={testId}
        data-route={to}
        onClick={() => {
          if (onPress !== undefined) {
            onPress();
            return;
          }
          onLeave?.();
          navigate(to);
        }}
      >
        <ArrowIcon direction="back" />
        {label === undefined ? t("Back") : t(label)}
      </button>
      {title === undefined && children === undefined ? null : (
        <div className="screen-nav__line">
          {title === undefined ? null : <PageTitle>{title}</PageTitle>}
          {children === undefined ? null : <div className="screen-nav__extra">{children}</div>}
        </div>
      )}
      {showSettings ? (
        <div className="screen-nav__end">
          {showWallet && onPress === undefined ? <JlorePill /> : null}
          {/* R3729002: the inbox's bell, after the pill, inside the shell, where Friends is the rail's; off the
              shell the Friends button below carries the requests and challenges as it did. */}
          {showWallet && shell !== null ? <InboxBell /> : null}
          {/* #1831: friends are one press away on every signed-in screen, as the pill and settings are;
              inside the shell, in its navigation (R3728000). */}
          {showWallet && shell === null ? <FriendsButton placement="nav" /> : null}
          <LanguageButton placement="nav" />
          <SettingsButton placement="nav" />
        </div>
      ) : null}
      {/* R3728002: the live game, or the queue search, on a row of its own under the bar. */}
      {shell === null ? null : <div className="screen-nav__status">{shell.status}</div>}
    </nav>
  );
}

/** Preserves modified and non-left-click anchor behavior. */
export function followInApp(to: string): (event: MouseEvent<HTMLAnchorElement>) => void {
  return (event) => {
    if (event.defaultPrevented) return;
    if (event.button !== 0) return;
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    navigate(to);
  };
}
