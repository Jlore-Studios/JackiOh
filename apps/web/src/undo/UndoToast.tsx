// R2119000: the undo toast a held deletion shows (`hold.ts`). It is a status, so a reader hears it
// without the focus moving: nothing here takes the focus, and Undo is the next stop of the keyboard's
// own order. Under Reduce motion (the media query or the settings switch) it appears, not slides in.

import type { ReactElement } from "react";

import { useOsReducedMotion } from "../game/useOsReducedMotion.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { useSetting } from "../settings/store.ts";
import type { HeldDeletion } from "./hold.ts";

import "./undo.css";

export const UNDO_TOAST = "undo-toast";
export const UNDO_TOAST_UNDO = "undo-toast-undo";

export function UndoToast({ held, onUndo }: { held: HeldDeletion | null; onUndo: () => void }): ReactElement {
  useLocale();
  const panelReduces = useSetting("reduceMotion");
  const osReduces = useOsReducedMotion();
  const reduced = panelReduces || osReduces;
  // The region is always there, so a screen reader announces each toast put inside it.
  return (
    <div className="undo-toast-region" role="status" aria-live="polite">
      {held === null ? null : (
        <div className="undo-toast" data-testid={UNDO_TOAST} data-motion={reduced ? "reduced" : "full"}>
          <span>{held.words()}</span>
          <span aria-hidden="true">{" · "}</span>
          <button type="button" className="link-button undo-toast__undo" data-testid={UNDO_TOAST_UNDO} onClick={onUndo}>
            {t("Undo")}
          </button>
        </div>
      )}
    </div>
  );
}
