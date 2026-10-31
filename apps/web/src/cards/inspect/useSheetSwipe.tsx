// #2121 (R2121000): a touch swipe down closes a card or pile sheet as Close does. A Prompt and the
// mulligan never use it ([[§10.6]]: a choice the game asks for never closes unanswered). Presentation
// only (CLAUDE.md rule 7): it moves the sheet, fades the scrim and calls Close's `close`.

import { useEffect, useRef } from "react";
import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent, ReactElement, RefObject } from "react";
import { reducedMotionNow } from "../../game/animations.ts";
import { SHEET_DISMISS_DISTANCE, SHEET_DISMISS_VELOCITY, SHEET_DRAG_SLOP_PX, SHEET_FLICK_WINDOW_MS } from "./constants.ts";

/** Where a drag may always start, scrolled or not: the grip, a pile sheet's header. */
export const SHEET_HANDLE_PROPS = { "data-sheet-handle": "" } as const;
const HANDLE_SELECTOR = "[data-sheet-handle]";
/** What inspect.css translates a sheet by. */
const DRAG_PROPERTY = "--sheet-drag";

type Drag = {
  pointerId: number; startX: number; startY: number;
  /** pending: inside the slop; dragging: the sheet follows; ignored: up or sideways first. */
  phase: "pending" | "dragging" | "ignored";
  height: number; reduced: boolean; lastY: number; lastAt: number; velocity: number;
};

type OnPointer = (event: ReactPointerEvent<HTMLElement>) => void;
export type SheetSwipe = {
  sheetRef: RefObject<HTMLDivElement | null>;
  scrimRef: RefObject<HTMLDivElement | null>;
  handlers: { onPointerDown: OnPointer; onPointerMove: OnPointer; onPointerUp: OnPointer; onPointerCancel: OnPointer;
    onClickCapture: (event: ReactMouseEvent<HTMLElement>) => void };
};

/** The bar at a sheet's top: a handle, and the sign it can be dragged. */
export function SheetGrip(): ReactElement {
  return <span className="inspect-grip" aria-hidden="true" {...SHEET_HANDLE_PROPS} />;
}

/** On a handle, or with nothing from the press up to the sheet scrolled down. */
function mayDrag(target: EventTarget, sheet: HTMLElement): boolean {
  // A portalled child (R279's tooltip) bubbles through React but is not in the sheet.
  if (!(target instanceof Element) || !sheet.contains(target)) return false;
  if (target.closest(HANDLE_SELECTOR) !== null) return true;
  for (let node: Element | null = target; node !== null; node = node.parentElement) {
    if (node.scrollTop > 0) return false;
    if (node === sheet) return true;
  }
  return false;
}

export function useSheetSwipe(dismiss: () => void): SheetSwipe {
  const sheetRef = useRef<HTMLDivElement>(null);
  const scrimRef = useRef<HTMLDivElement>(null);
  const drag = useRef<Drag | null>(null);
  const swallowClick = useRef(false);

  // The browser's pan would take the touch (pointercancel). React's touchmove is passive, so a
  // native listener holds it off while the sheet follows, or while a press that may drag starts down.
  useEffect(() => {
    const sheet = sheetRef.current;
    if (sheet === null) return;
    const onTouchMove = (event: TouchEvent): void => {
      const current = drag.current;
      if (current === null || current.phase === "ignored" || !event.cancelable) return;
      if (current.phase === "dragging" || (event.touches[0]?.clientY ?? current.startY) > current.startY) event.preventDefault();
    };
    sheet.addEventListener("touchmove", onTouchMove, { passive: false });
    return () => sheet.removeEventListener("touchmove", onTouchMove);
  }, []);

  const follow = (offset: number, height: number): void => {
    const sheet = sheetRef.current;
    const scrim = scrimRef.current;
    if (sheet === null) return;
    sheet.dataset.swipe = "dragging";
    sheet.style.setProperty(DRAG_PROPERTY, `${String(offset)}px`);
    if (scrim === null) return;
    scrim.dataset.swipe = "dragging";
    scrim.style.opacity = String(height > 0 ? Math.max(0, 1 - offset / height) : 1);
  };

  /** Back to rest; inspect.css's transition is the spring. */
  const settle = (): void => {
    for (const element of [sheetRef.current, scrimRef.current]) {
      if (element === null) continue;
      delete element.dataset.swipe;
      element.style.removeProperty(DRAG_PROPERTY);
      element.style.removeProperty("opacity");
    }
  };

  const handlers: SheetSwipe["handlers"] = {
    onPointerDown: (event) => {
      swallowClick.current = false;
      if (event.pointerType !== "touch" || !event.isPrimary) return;
      const sheet = sheetRef.current;
      drag.current = null;
      if (sheet === null || !mayDrag(event.target, sheet)) return;
      drag.current = {
        pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, phase: "pending",
        height: sheet.offsetHeight, reduced: reducedMotionNow(), lastY: event.clientY, lastAt: Date.now(), velocity: 0,
      };
    },
    onPointerMove: (event) => {
      const current = drag.current;
      if (current === null || event.pointerId !== current.pointerId || current.phase === "ignored") return;
      const dx = event.clientX - current.startX;
      const dy = event.clientY - current.startY;
      if (current.phase === "pending") {
        if (Math.hypot(dx, dy) <= SHEET_DRAG_SLOP_PX) return;
        // Up or sideways first is the content's scroll, or nothing: the whole gesture is left alone.
        current.phase = dy > Math.abs(dx) ? "dragging" : "ignored";
        if (current.phase === "ignored") return;
      }
      const now = Date.now();
      if (now > current.lastAt) {
        current.velocity = (event.clientY - current.lastY) / (now - current.lastAt);
        current.lastY = event.clientY;
        current.lastAt = now;
      }
      // Reduce motion: nothing follows the finger; the release still decides.
      if (!current.reduced) follow(Math.max(0, dy), current.height);
    },
    onPointerUp: (event) => {
      const current = drag.current;
      if (current === null || event.pointerId !== current.pointerId) return;
      drag.current = null;
      if (current.phase !== "dragging") return;
      settle(); // before dismiss: the sheet unmounts in this same event
      const far = current.height > 0 && event.clientY - current.startY >= current.height * SHEET_DISMISS_DISTANCE;
      const fast = Date.now() - current.lastAt <= SHEET_FLICK_WINDOW_MS && current.velocity >= SHEET_DISMISS_VELOCITY;
      if (far || fast) dismiss();
      else swallowClick.current = true;
    },
    onPointerCancel: (event) => {
      if (drag.current === null || event.pointerId !== drag.current.pointerId) return;
      drag.current = null;
      settle();
    },
    onClickCapture: (event) => {
      if (!swallowClick.current) return;
      swallowClick.current = false;
      if (event.detail === 0) return; // a keyboard's click is never a drag's lift
      event.preventDefault();
      event.stopPropagation();
    },
  };

  return { sheetRef, scrimRef, handlers };
}
