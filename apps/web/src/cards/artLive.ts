// R3340000: what a card's art answers on the board. A face in play (CardFace `board`, MinionFace) listens on its
// card's host (`.cf-host`: `.cf` itself takes no pointer) for the pointer and for focus: it is `hovered` while either is on
// the card, which ranks it first for a motion slot (motionSlots.ts) and sets `data-art-hover`; the pointer's place in the
// face is written on it as `--art-aim-x` and `--art-aim-y`, -1 to 1 each way (`aimOf`), which artLive.css turns the art's
// subject toward; keyboard focus alone uses a small fixed aim because it has no pointer coordinate, while a pointer inside
// wins. A press sets `data-art-press` until it lifts, leaves or is cancelled. An `IntersectionObserver`, where the browser
// has one, tells the face when it is off screen (`onScreen`), when its art holds still and frees its slot. A listener reads
// the event alone, and a face that is not enabled listens to nothing and holds none of the marks.
// Presentation only (CLAUDE.md rule 7): nothing here reaches the engine, an action or the log.

import { useEffect, useState, type RefObject } from "react";

/** R3340000: the custom properties a face holds the pointer's place in it in, -1 (left, top) to 1 (right, bottom). */
export const ART_AIM_X_PROPERTY = "--art-aim-x";
export const ART_AIM_Y_PROPERTY = "--art-aim-y";
/** The attribute a face wears while it is pressed. */
export const ART_PRESS_ATTRIBUTE = "data-art-press";

/** The digits an aim is written with. */
const AIM_DIGITS = 3;
/** R3340000: the subtle upper-right turn and lean a keyboard-focused face uses when there is no pointer place to follow. */
const FOCUS_AIM_X = 0.25;
const FOCUS_AIM_Y = -0.25;

/** `value` held to -1 to 1, and 0 for no number. */
function unitClamp(value: number): number {
  return Number.isFinite(value) ? Math.min(1, Math.max(-1, value)) : 0;
}

/** R3340000: the pointer at (`clientX`, `clientY`) in the box `rect`, -1 to 1 each way from its middle; a side of no length gives 0 along it. */
export function aimOf(clientX: number, clientY: number, rect: { left: number; top: number; width: number; height: number }): { x: number; y: number } {
  return {
    x: rect.width > 0 ? unitClamp(((clientX - rect.left) / rect.width) * 2 - 1) : 0,
    y: rect.height > 0 ? unitClamp(((clientY - rect.top) / rect.height) * 2 - 1) : 0,
  };
}

/**
 * R3340000: while `enabled`, follows the pointer and focus on the card the face at `ref` is in. `hovered` is true while the
 * pointer or focus is on it, `onScreen` false while the browser says it is out of view (true with no observer to ask).
 */
export function useArtLive(ref: RefObject<HTMLElement | null>, enabled: boolean): { hovered: boolean; onScreen: boolean } {
  const [hovered, setHovered] = useState(false);
  const [onScreen, setOnScreen] = useState(true);
  useEffect(() => {
    const el = ref.current;
    if (!enabled || el === null) return undefined;
    const host = el.closest<HTMLElement>(".cf-host") ?? el;
    let pointerInside = false;
    let focusInside = false;
    // A press focuses the card it lands on (Card.tsx's tabIndex); that focus is the pointer's, not the keyboard's.
    let pressing = false;
    const updateHovered = (): void => {
      setHovered(pointerInside || focusInside);
    };
    const pointerEnter = (): void => {
      pointerInside = true;
      updateHovered();
    };
    const writeAim = (x: number, y: number): void => {
      el.style.setProperty(ART_AIM_X_PROPERTY, x.toFixed(AIM_DIGITS));
      el.style.setProperty(ART_AIM_Y_PROPERTY, y.toFixed(AIM_DIGITS));
    };
    const clearAim = (): void => {
      el.style.removeProperty(ART_AIM_X_PROPERTY);
      el.style.removeProperty(ART_AIM_Y_PROPERTY);
    };
    const focusAim = (): void => {
      writeAim(FOCUS_AIM_X, FOCUS_AIM_Y);
    };
    const focusIn = (): void => {
      if (pressing) return;
      focusInside = true;
      if (!pointerInside) focusAim();
      updateHovered();
    };
    const move = (event: PointerEvent): void => {
      const aim = aimOf(event.clientX, event.clientY, el.getBoundingClientRect());
      writeAim(aim.x, aim.y);
    };
    const press = (): void => {
      pressing = true;
      el.setAttribute(ART_PRESS_ATTRIBUTE, "true");
    };
    const lift = (): void => {
      pressing = false;
      el.removeAttribute(ART_PRESS_ATTRIBUTE);
    };
    const pointerLeave = (): void => {
      lift();
      pointerInside = false;
      if (focusInside) focusAim();
      else clearAim();
      updateHovered();
    };
    const focusOut = (event: FocusEvent): void => {
      focusInside = event.relatedTarget instanceof Node && host.contains(event.relatedTarget);
      if (!focusInside && !pointerInside) clearAim();
      updateHovered();
    };
    host.addEventListener("pointerenter", pointerEnter);
    host.addEventListener("focusin", focusIn);
    host.addEventListener("pointermove", move, { passive: true });
    host.addEventListener("pointerdown", press);
    host.addEventListener("pointerup", lift);
    host.addEventListener("pointercancel", lift);
    host.addEventListener("pointerleave", pointerLeave);
    host.addEventListener("focusout", focusOut);
    const observer =
      typeof IntersectionObserver === "function"
        ? new IntersectionObserver((entries) => {
            const latest = entries[entries.length - 1];
            if (latest !== undefined) setOnScreen(latest.isIntersecting);
          })
        : null;
    observer?.observe(el);
    return () => {
      host.removeEventListener("pointerenter", pointerEnter);
      host.removeEventListener("focusin", focusIn);
      host.removeEventListener("pointermove", move);
      host.removeEventListener("pointerdown", press);
      host.removeEventListener("pointerup", lift);
      host.removeEventListener("pointercancel", lift);
      host.removeEventListener("pointerleave", pointerLeave);
      host.removeEventListener("focusout", focusOut);
      observer?.disconnect();
      lift();
      clearAim();
      setHovered(false);
      setOnScreen(true);
    };
  }, [ref, enabled]);
  return { hovered: enabled && hovered, onScreen: !enabled || onScreen };
}
