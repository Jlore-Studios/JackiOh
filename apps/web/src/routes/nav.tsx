// Inner screens need a visible exit when direct links have no useful browser Back history.

import type { MouseEvent, ReactElement, ReactNode } from "react";

import { navigate, paths } from "../net/navigate.ts";
import { SettingsButton } from "../settings/index.ts";

export const navTestid = {
  back: "nav-back",
  account: "nav-account",
} as const;

export type BackLinkProps = {
  to?: string;
  label?: string;
  /** Releases screen-owned state before navigation. */
  onLeave?: () => void;
  /** Overrides the default release-and-navigate sequence for confirmation flows. */
  onPress?: () => void;
  /** Extra top-bar content; settings remains the top-right control. */
  children?: ReactNode;
  /** The board supplies settings separately. */
  showSettings?: boolean;
};

/** Uses `navigate`, not history, so direct links stay in the app. */
export function BackLink({
  to = paths.landing,
  label = "← Back",
  onLeave,
  onPress,
  children,
  showSettings = true,
}: BackLinkProps): ReactElement {
  return (
    <nav className="row screen-nav">
      <button
        type="button"
        className="link-button"
        data-testid={navTestid.back}
        onClick={() => {
          if (onPress !== undefined) {
            onPress();
            return;
          }
          onLeave?.();
          navigate(to);
        }}
      >
        {label}
      </button>
      {children === undefined ? null : <div className="screen-nav__extra">{children}</div>}
      {showSettings ? <SettingsButton placement="nav" /> : null}
    </nav>
  );
}

/** Preserves modified and non-left-click anchor behaviour. */
export function followInApp(to: string): (event: MouseEvent<HTMLAnchorElement>) => void {
  return (event) => {
    if (event.defaultPrevented) return;
    if (event.button !== 0) return;
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    navigate(to);
  };
}
