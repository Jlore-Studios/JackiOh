// R3156000 (#3156): the Dev tools' door on the front screen, a button in the top bar beside the
// account's, for a dev account on a computer (no `DEV_PANEL_HIDDEN_QUERY` window). It opens the Dev tools
// (`DevTools.tsx`) in a dialog over the front screen. The server decides (CLAUDE.md rule 7): the
// button shows only when `/api/auth/me` says `isDev`, and every tool's route checks
// `DEV_PROFILE_IDS` again. Escape, Back (R2127000), the scrim and the ✕ close the dialog, drawn
// through Dialog.tsx (R3221000), and the focus goes back to the button.

import { useCallback, useId, useRef, useState, useSyncExternalStore, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { Dialog } from "../game/Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import type { Account } from "../net/gate.ts";
import DevTools from "./DevTools.tsx";
import { DEV_PANEL_HIDDEN_QUERY } from "./devPanel.ts";

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
    return window.matchMedia(DEV_PANEL_HIDDEN_QUERY);
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

/** A host with no media queries shows no button: it cannot say it is a computer. */
function onComputer(): boolean {
  const list = queryList();
  return list !== null && !list.matches;
}

/** Whether this window is a computer's (not `DEV_PANEL_HIDDEN_QUERY`'s), kept current by the query's `change` event. */
export function useOnComputer(): boolean {
  return useSyncExternalStore(subscribe, onComputer, () => false);
}

/** R3221000: the Dev tools on Dialog.tsx, under a `.tavern` for the panel's colours. */
function DevPanel({ id, token, onClose }: { id: string; token: string; onClose: () => void }): ReactElement {
  return (
    <div className="tavern">
      <Dialog
        role="dialog"
        id={id}
        testid={devButtonTestid.panel}
        closeTestid={devButtonTestid.close}
        className="dev-panel"
        title={t("Dev tools")}
        onClose={onClose}
        actions={[]}
      >
        <DevTools token={token} />
      </Dialog>
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
