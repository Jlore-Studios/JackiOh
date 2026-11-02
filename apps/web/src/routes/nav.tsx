// Inner screens need a visible exit when direct links have no useful browser Back history.

import type { MouseEvent, ReactElement, ReactNode } from "react";

import JlorePill from "../economy/JlorePill.tsx";
import LanguageButton from "../i18n/LanguageButton.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { navigate, paths } from "../net/navigate.ts";
import { SettingsButton } from "../settings/index.ts";
import { FriendsButton } from "../together/FriendsButton.tsx";

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
  /** Extra top-bar content; settings remains the top-right control. */
  children?: ReactNode;
  /** The board supplies settings separately. */
  showSettings?: boolean;
  /** The Jlore pill; off on signed-out screens, where a session only lands as the screen leaves. */
  showWallet?: boolean;
};

/** Uses `navigate`, not history, so direct links stay in the app. */
export function BackLink({
  to = paths.landing,
  label,
  onLeave,
  onPress,
  children,
  showSettings = true,
  showWallet = true,
}: BackLinkProps): ReactElement {
  useLocale();
  return (
    <nav className="row screen-nav">
      <button
        type="button"
        className="link-button"
        data-testid={navTestid.back}
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
        {label === undefined ? t("← Back") : t(label)}
      </button>
      {children === undefined ? null : <div className="screen-nav__extra">{children}</div>}
      {showSettings && showWallet && onPress === undefined ? <JlorePill /> : null}
      {/* #1831: friends are one press away on every signed-in screen, as the pill and settings are. */}
      {showSettings && showWallet ? <FriendsButton placement="nav" /> : null}
      {showSettings ? <LanguageButton placement="nav" /> : null}
      {showSettings ? <SettingsButton placement="nav" /> : null}
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
