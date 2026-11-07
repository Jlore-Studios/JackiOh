// The keys of a modal: `game/Dialog.tsx`'s, every dialog drawn through it (#3220, R3220000), and the
// challenge dialog's and the friends panel's (#1831): Escape and Back (R2127000) close it and Tab stays
// inside it. Given the control to focus, it also takes the focus on open and gives it back on close.

import { useEffect, useLayoutEffect, type RefObject } from "react";

import { useBackLayer } from "../net/backLayers.ts";

const TAB_STOPS = "a[href], button:not([disabled]), input:not([disabled]), select:not([disabled])";

/** A radio group is one stop, its checked radio, as the browser's own Tab takes it. */
export function tabStops(modal: HTMLElement): HTMLElement[] {
  return [...modal.querySelectorAll<HTMLElement>(TAB_STOPS)].filter(
    (element) => !(element instanceof HTMLInputElement && element.type === "radio" && !element.checked),
  );
}

/** Both keys are read at the window, in the capture phase, so they hold wherever a click left the focus. */
export function useModalKeys(
  root: RefObject<HTMLElement | null>,
  onClose: () => void,
  focus?: RefObject<HTMLElement | null>,
): void {
  useBackLayer(onClose);
  // R3220000: given `focus`, the modal focuses it on open and gives the focus back to its opener on close,
  // unless the caller has moved it by then (Game.tsx's refocusConcede).
  useLayoutEffect(() => {
    if (focus === undefined) return;
    const active = document.activeElement;
    const opener = active instanceof HTMLElement && active !== document.body ? active : null;
    const modal = root.current;
    focus.current?.focus({ preventScroll: true });
    return () => {
      const now = document.activeElement;
      const left = now === null || now === document.body || (modal?.contains(now) ?? false);
      if (left && opener?.isConnected === true) opener.focus({ preventScroll: true });
    };
  }, [root, focus]);
  useEffect(() => {
    function onKey(event: globalThis.KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const modal = root.current;
      if (modal === null) return;
      const order = tabStops(modal);
      if (order.length === 0) return;
      const focused = document.activeElement;
      const at = focused instanceof HTMLElement ? order.indexOf(focused) : -1;
      const next = event.shiftKey ? (at <= 0 ? order.length - 1 : at - 1) : at === order.length - 1 ? 0 : at + 1;
      event.preventDefault();
      event.stopPropagation();
      order[next]?.focus();
    }
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
    };
  }, [root, onClose]);
}
