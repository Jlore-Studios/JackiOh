// R658: a prompt's options are dragged as well as clicked (docs/polish/7-mobile-ux.md S9, #261).
//
// A press on one of the open picker's options (`prompt-option-<key>`) that travels
// DRAG_THRESHOLD_PX lifts it: a ghost with the option's name follows the pointer, and the option
// itself fades where it sits. Released outside the picker's panel (over the board), the option is
// clicked, so the drop does exactly what a click on it does: a one-of-N picker (a Discover, a
// "Choose one", a target list) sends its answer, and a picker of several picks (the mulligan)
// toggles that pick. Released back over the panel, or cancelled (Escape, the context menu, a lost
// pointer), it does nothing. Nothing here reads a rule or an option's meaning (CLAUDE.md rule 7):
// what the click does is Prompt.tsx's, and its options came from the engine.
//
// The board's own drags are DragLayer.tsx's; this one never starts on the board, and that one never
// starts in a picker, since the picker sits outside `[data-testid="board"]`.

import { useEffect, useState, type CSSProperties, type ReactElement } from "react";

import { readSettings } from "../../settings/index.ts";
import { DRAG_THRESHOLD_PX } from "./model.ts";

const PANEL = '[data-testid="prompt-modal"]';
const OPTION = '[data-testid^="prompt-option-"]';
/** Set on the option being dragged, so drag.css fades it while its ghost is away. */
export const DRAG_SOURCE_ATTRIBUTE = "data-drag-source";

type Point = { x: number; y: number };
type Press = { pointerId: number; start: Point; option: HTMLElement };
type Flight = { pointerId: number; option: HTMLElement; pointer: Point; touch: boolean; valid: boolean };

/** The option a press landed on, when it is one the picker would take a click on. */
function optionAt(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof Element)) return null;
  if (target.closest(PANEL) === null) return null;
  const option = target.closest(OPTION);
  if (!(option instanceof HTMLElement)) return null;
  if (option.getAttribute("aria-disabled") === "true") return null;
  if (option instanceof HTMLButtonElement && option.disabled) return null;
  return option;
}

/**
 * Over the panel the option came from, by its box: the panel and the scrim take no pointer events
 * (prompt.css), so a hit test would see the board through them.
 */
function overPanel(option: HTMLElement, point: Point): boolean {
  const panel = option.closest(PANEL);
  if (panel === null) return false;
  const rect = panel.getBoundingClientRect();
  return point.x >= rect.left && point.x <= rect.right && point.y >= rect.top && point.y <= rect.bottom;
}

/** What the ghost reads: the option's accessible name, else its text. */
function nameOf(option: HTMLElement): string {
  const label = option.getAttribute("aria-label");
  if (label !== null && label !== "") return label;
  return (option.textContent ?? "").replace(/\s+/g, " ").trim();
}

export default function OptionDrag(): ReactElement | null {
  const [drawn, setDrawn] = useState<Flight | null>(null);

  useEffect(() => {
    const root = document.documentElement;
    let press: Press | null = null;
    let flight: Flight | null = null;
    /** Armed when a drag ends: the click the release produces is not a second click. */
    let swallowClick = false;

    function stop(): void {
      const ending = flight;
      flight = null;
      press = null;
      if (ending !== null) {
        root.removeAttribute("data-dragging");
        ending.option.removeAttribute(DRAG_SOURCE_ATTRIBUTE);
        try {
          if (ending.option.hasPointerCapture(ending.pointerId)) ending.option.releasePointerCapture(ending.pointerId);
        } catch {
          // jsdom has no pointer capture.
        }
        swallowClick = true;
      }
      setDrawn(null);
    }

    function onPointerDown(event: PointerEvent): void {
      if (flight !== null) {
        if (event.pointerId !== flight.pointerId) return;
        stop();
      }
      swallowClick = false;
      press = null;
      if (event.button !== 0) return;
      const option = optionAt(event.target);
      if (option === null || !readSettings().dragToPlay) return;
      press = { pointerId: event.pointerId, start: { x: event.clientX, y: event.clientY }, option };
      // Never preventDefault: below the threshold this press is a click.
    }

    function onPointerMove(event: PointerEvent): void {
      const pointer = { x: event.clientX, y: event.clientY };
      if (flight !== null) {
        if (event.pointerId !== flight.pointerId) return;
        flight = { ...flight, pointer, valid: !overPanel(flight.option, pointer) };
        setDrawn(flight);
        return;
      }
      if (press === null || event.pointerId !== press.pointerId) return;
      if (Math.hypot(pointer.x - press.start.x, pointer.y - press.start.y) < DRAG_THRESHOLD_PX) return;
      const { option } = press;
      press = null;
      if (!option.isConnected) return;
      root.setAttribute("data-dragging", "option");
      option.setAttribute(DRAG_SOURCE_ATTRIBUTE, "true");
      try {
        option.setPointerCapture(event.pointerId);
      } catch {
        // jsdom has no pointer capture, and a pointer that is already gone cannot be captured.
      }
      flight = {
        pointerId: event.pointerId,
        option,
        pointer,
        touch: event.pointerType === "touch",
        valid: !overPanel(option, pointer),
      };
      setDrawn(flight);
    }

    function onPointerUp(event: PointerEvent): void {
      if (flight === null) {
        if (press !== null && event.pointerId === press.pointerId) press = null;
        return;
      }
      if (event.pointerId !== flight.pointerId) return;
      const { option } = flight;
      const drop = !overPanel(option, { x: event.clientX, y: event.clientY }) && option.isConnected;
      stop();
      // The drop is the option's own click, sent before the swallow takes the release's click.
      if (drop) {
        swallowClick = false;
        option.click();
        swallowClick = true;
      }
    }

    function cancel(): void {
      if (flight === null) {
        press = null;
        return;
      }
      stop();
    }

    function onPointerCancel(event: PointerEvent): void {
      if (flight !== null && event.pointerId !== flight.pointerId) return;
      cancel();
    }

    function onKeyDown(event: KeyboardEvent): void {
      swallowClick = false;
      if (event.key === "Escape" && flight !== null) {
        event.stopPropagation();
        cancel();
      }
    }

    function onContextMenu(event: MouseEvent): void {
      if (flight === null) return;
      event.preventDefault();
      cancel();
    }

    function onClickCapture(event: MouseEvent): void {
      if (!swallowClick) return;
      swallowClick = false;
      event.stopPropagation();
      event.preventDefault();
    }

    /** A native drag of an image inside an option card would send `pointercancel` mid-drag. */
    function onDragStart(event: DragEvent): void {
      if (press === null && flight === null) return;
      if (optionAt(event.target) === null) return;
      event.preventDefault();
    }

    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("pointermove", onPointerMove, true);
    window.addEventListener("pointerup", onPointerUp, true);
    window.addEventListener("pointercancel", onPointerCancel, true);
    window.addEventListener("click", onClickCapture, true);
    window.addEventListener("dragstart", onDragStart, true);
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("contextmenu", onContextMenu);
    window.addEventListener("blur", cancel);

    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("pointermove", onPointerMove, true);
      window.removeEventListener("pointerup", onPointerUp, true);
      window.removeEventListener("pointercancel", onPointerCancel, true);
      window.removeEventListener("click", onClickCapture, true);
      window.removeEventListener("dragstart", onDragStart, true);
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("contextmenu", onContextMenu);
      window.removeEventListener("blur", cancel);
      if (flight !== null) {
        flight.option.removeAttribute(DRAG_SOURCE_ATTRIBUTE);
        root.removeAttribute("data-dragging");
      }
      flight = null;
      press = null;
    };
  }, []);

  if (drawn === null) return null;
  const style: CSSProperties = { left: drawn.pointer.x, top: drawn.pointer.y };
  return (
    <div className="drag-layer" data-testid="drag-layer" data-kind="option" aria-hidden="true" style={{ pointerEvents: "none" }}>
      <div
        className="drag-ghost drag-ghost--option"
        data-testid="drag-option-ghost"
        data-option={drawn.option.getAttribute("data-testid") ?? undefined}
        data-pointer={drawn.touch ? "touch" : "mouse"}
        data-valid={drawn.valid ? "true" : "false"}
        style={style}
      >
        <span className="drag-ghost-name">{nameOf(drawn.option)}</span>
      </div>
    </div>
  );
}
