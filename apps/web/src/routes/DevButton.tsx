// R3156000 (#3156): the Dev tools' door on the front screen, a button in the top bar beside the
// account's, for a dev account on a computer (`DEV_PANEL_QUERY`). It opens the Dev tools
// (`DevTools.tsx`) in a dialog over the front screen. The server decides (CLAUDE.md rule 7): the
// button shows only when `/api/auth/me` says `isDev`, and every tool's route checks
// `DEV_PROFILE_IDS` again. Escape, Back (R2127000) and the scrim close the dialog, as the friends
// panel's do, and the focus goes back to the button.

import { useCallback, useEffect, useId, useRef, useState, useSyncExternalStore, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { t, useLocale } from "../i18n/i18n.ts";
import type { Account } from "../net/gate.ts";
import { useModalKeys } from "../together/modalKeys.ts";
import DevTools from "./DevTools.tsx";
import { DEV_PANEL_QUERY } from "./devPanel.ts";

import "../auth/tavern.css";
import "./dev-panel.css";

export const devButtonTestid = {
  button: "landing-dev",
  panel: "dev-panel",
  close: "dev-panel-close",
} as const;

// It caches no `MediaQueryList`: a test swaps `window.matchMedia` between renders.
function queryList(): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  try {
    return window.matchMedia(DEV_PANEL_QUERY);
  } catch {
    return null;
  }
}

function subscribe(onChange: () => void): () => void {
  const query = queryList();
  if (query === null || typeof query.addEventListener !== "function") return () => undefined;
  query.addEventListener("change", onChange);
  return () => {
    query.removeEventListener("change", onChange);
  };
}

function onComputer(): boolean {
  return queryList()?.matches === true;
}

/** Whether this window is a computer's (`DEV_PANEL_QUERY`), kept current by the query's `change` event. */
export function useOnComputer(): boolean {
  return useSyncExternalStore(subscribe, onComputer, () => false);
}

function DevPanel({ id, token, onClose }: { id: string; token: string; onClose: () => void }): ReactElement {
  const root = useRef<HTMLDivElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  const titleId = useId();
  useModalKeys(root, onClose);
  useEffect(() => {
    close.current?.focus({ preventScroll: true });
  }, []);
  return (
    <div
      className="tavern app-shell dev-layer"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={root}
        id={id}
        className="dev-panel"
        data-testid={devButtonTestid.panel}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="dev-panel__head">
          <h2 className="dev-panel__title" id={titleId}>
            {t("Dev tools")}
          </h2>
          <button
            ref={close}
            type="button"
            className="link-button"
            data-testid={devButtonTestid.close}
            aria-label={t("Close")}
            onClick={onClose}
          >
            ×
          </button>
        </div>
        <DevTools token={token} />
      </div>
    </div>
  );
}

export function DevButton({ account }: { account: Account }): ReactElement | null {
  useLocale();
  const computer = useOnComputer();
  const button = useRef<HTMLButtonElement>(null);
  const panelId = useId();
  const [open, setOpen] = useState(false);

  const closePanel = useCallback((): void => {
    setOpen(false);
    const element = button.current;
    if (element !== null && element.isConnected) element.focus({ preventScroll: true });
  }, []);

  if (account.kind !== "ready" || account.me.isDev !== true || !computer) return null;

  return (
    <>
      <button
        ref={button}
        type="button"
        className="landing-corner-link landing-dev-button"
        data-testid={devButtonTestid.button}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        onClick={() => {
          setOpen(true);
        }}
      >
        {t("Dev")}
      </button>
      {open ? createPortal(<DevPanel id={panelId} token={account.token} onClose={closePanel} />, document.body) : null}
    </>
  );
}
