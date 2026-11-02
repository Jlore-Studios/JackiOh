// A small bar of controls over the board spot the pointer or the focus is on (#936 B62, R1838000):
// what the Duos card bar (duos/CardActions.tsx) and the Trade control (game/TradeControl.tsx) share.
// Nothing here is a rule: it says which spot is pointed at and where its element sits, and the caller
// decides what the bar offers from the view and `legal`.
//
// THE FOCUS. Pointing at a hero, a zone or a hand card (a mouse over it, a tap on it, the keyboard's
// focus on it) puts the bar on it. It stays `graceMs` after the pointer leaves its card, so the
// pointer can reach it, and goes on Escape and on a press elsewhere; `close` takes it off after a pick.
//
// THE BOXES. The bar and anything else laid over the board sits at the box of the element the board
// draws for a spot (`duos/spots.ts`), re-measured every frame since cards move as the board animates.

import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { sameSpot, spotOfElement, type BoardSpot } from "../duos/spots.ts";
import { testidSelector } from "./contract.ts";

export type Box = { left: number; top: number; width: number; height: number };

const NO_BOXES: ReadonlyMap<string, Box> = new Map();

function onBar(target: EventTarget | null, barClass: string): boolean {
  return target instanceof Element && target.closest(`.${barClass}`) !== null;
}

function elementOf(target: EventTarget | null): Element | null {
  return target instanceof Element ? target : null;
}

function sameBoxes(a: ReadonlyMap<string, Box>, b: ReadonlyMap<string, Box>): boolean {
  if (a.size !== b.size) return false;
  for (const [id, box] of a) {
    const other = b.get(id);
    if (other === undefined) return false;
    if (other.left !== box.left || other.top !== box.top || other.width !== box.width || other.height !== box.height) {
      return false;
    }
  }
  return true;
}

/**
 * The boxes of the elements `idsKey` names ("|"-joined testids) inside `host`, kept current every
 * animation frame while there are any: relative to `host` (`origin: "host"`, for a bar positioned
 * inside it), or to the viewport (`"viewport"`, for a fixed one).
 */
export function useBoxes(
  host: HTMLElement | null,
  idsKey: string,
  origin: "host" | "viewport" = "host",
): ReadonlyMap<string, Box> {
  const [boxes, setBoxes] = useState<ReadonlyMap<string, Box>>(NO_BOXES);
  useLayoutEffect(() => {
    if (idsKey === "") {
      setBoxes(NO_BOXES);
      return undefined;
    }
    const ids = idsKey.split("|");
    let live = true;
    let frame: number | null = null;
    const measure = (): void => {
      if (!live) return;
      if (host !== null) {
        const from = origin === "host" ? host.getBoundingClientRect() : { left: 0, top: 0 };
        const next = new Map<string, Box>();
        for (const id of ids) {
          const element = host.querySelector(testidSelector(id));
          if (element === null) continue;
          const rect = element.getBoundingClientRect();
          next.set(id, { left: rect.left - from.left, top: rect.top - from.top, width: rect.width, height: rect.height });
        }
        setBoxes((previous) => (sameBoxes(previous, next) ? previous : next));
      }
      // As the tutorial's coach measures (Coach.tsx): every frame where there are frames.
      frame = typeof requestAnimationFrame === "function" ? requestAnimationFrame(measure) : null;
    };
    measure();
    return () => {
      live = false;
      if (frame !== null && typeof cancelAnimationFrame === "function") cancelAnimationFrame(frame);
    };
  }, [host, idsKey, origin]);
  return boxes;
}

/** The spot a bar of class `barClass` sits on inside `container`, and the way to take it off. */
export function useCardBarFocus(
  container: HTMLElement | null,
  barClass: string,
  graceMs: number,
): { focus: BoardSpot | null; close: () => void } {
  const [focus, setFocus] = useState<BoardSpot | null>(null);
  const clearTimer = useRef<number | null>(null);

  useEffect(() => {
    const host = container;
    if (host === null) return undefined;
    const cancelClear = (): void => {
      if (clearTimer.current === null) return;
      window.clearTimeout(clearTimer.current);
      clearTimer.current = null;
    };
    const clearSoon = (): void => {
      if (clearTimer.current !== null) return;
      clearTimer.current = window.setTimeout(() => {
        clearTimer.current = null;
        setFocus(null);
      }, graceMs);
    };
    const point = (spot: BoardSpot): void => {
      cancelClear();
      setFocus((current) => (sameSpot(current, spot) ? current : spot));
    };
    // Whether the last pointer was a finger. Nothing hovers on a phone, so a tap's bar stays until
    // the next press: the leave that ends every touch, and the mouse events a browser fires after a
    // tap (which carry no pointer type), close nothing.
    let touch = false;
    const noteKind = (event: Event): void => {
      const kind = (event as { pointerType?: unknown }).pointerType;
      if (typeof kind === "string" && kind !== "") touch = kind === "touch";
    };
    // A pointer or the focus arriving: the bar follows it onto a spot, and waits while it is on the bar.
    const over = (event: Event): void => {
      noteKind(event);
      if (onBar(event.target, barClass)) {
        cancelClear();
        return;
      }
      const spot = spotOfElement(elementOf(event.target));
      if (spot !== null) point(spot);
      else if (!touch) clearSoon();
    };
    // A press (a tap on a phone, where nothing hovers) moves the bar to its spot, or closes it.
    const down = (event: Event): void => {
      noteKind(event);
      if (onBar(event.target, barClass)) return;
      const spot = spotOfElement(elementOf(event.target));
      if (spot !== null) {
        point(spot);
        return;
      }
      cancelClear();
      setFocus(null);
    };
    const leave = (event: Event): void => {
      noteKind(event);
      if (!touch) clearSoon();
    };
    const key = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      cancelClear();
      setFocus(null);
    };
    host.addEventListener("pointerover", over);
    host.addEventListener("mouseover", over);
    host.addEventListener("focusin", over);
    host.addEventListener("pointerdown", down);
    host.addEventListener("pointerleave", leave);
    host.addEventListener("mouseleave", leave);
    window.addEventListener("keydown", key);
    return () => {
      host.removeEventListener("pointerover", over);
      host.removeEventListener("mouseover", over);
      host.removeEventListener("focusin", over);
      host.removeEventListener("pointerdown", down);
      host.removeEventListener("pointerleave", leave);
      host.removeEventListener("mouseleave", leave);
      window.removeEventListener("keydown", key);
      cancelClear();
    };
  }, [container, barClass, graceMs]);

  const close = (): void => {
    if (clearTimer.current !== null) {
      window.clearTimeout(clearTimer.current);
      clearTimer.current = null;
    }
    setFocus(null);
  };

  return { focus, close };
}
