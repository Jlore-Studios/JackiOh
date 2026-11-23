// Issue #3734 (R3734001): a pool card is dragged into the deck with pointer events, so a finger works as a mouse
// does; the browser's HTML5 drag never fires on touch. `game/drag/OptionDrag.tsx` is the model: window listeners
// in the capture phase, a press that stays a click below DRAG_THRESHOLD_PX, a ghost that follows the pointer.
//
// A press on a pool card of the deck editor (`.db-card[data-legal]`, which the read-only Almanac's cards lack) that
// travels the threshold lifts it. A mouse or a pen lifts in any direction; a touch lifts only when it moves more across than down
// (deckbuilder.css leaves `touch-action: pan-y` on the card, so a vertical start is the browser's scroll, which
// sends `pointercancel`). Released over the deck (`[data-testid="deck-drop"]`, by its box) the card is added, as the
// "+" adds it, so a card the deck would not take is refused and the refusal reported, as for a tap, and as the browser's
// drag did before it; released anywhere else, on Escape, on a cancel or on
// a blur, nothing happens. Nothing here reads a rule (CLAUDE.md rule 7): whether the deck takes a card is
// the editor's `add`, and the validator's the last word.

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import { closeInspect } from "../../cards/index.ts";
import { DRAG_THRESHOLD_PX } from "../drag/model.ts";
import { CARD_POOL, DECK_DRAG_GHOST, DECK_DROP } from "./testids.ts";

const POOL = `[data-testid="${CARD_POOL}"]`;
const CARD = ".db-card[data-legal]";
const DROP = `[data-testid="${DECK_DROP}"]`;

type Point = { x: number; y: number };
type Press = { pointerId: number; start: Point; card: HTMLElement; cardId: string };
type Flight = { pointerId: number; cardId: string; pointer: Point; touch: boolean };

type DeckDragProps = {
  /** Adds the card to the open deck: the editor's own `add`, the same a tap on "+" calls. */
  onDrop: (cardId: string) => void;
  /** The words the ghost carries; the card's id when absent. */
  nameOf?: (cardId: string) => string;
};

/** The deck editor's pool card a press landed on. */
function cardAt(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof Element)) return null;
  if (target.closest(POOL) === null) return null;
  const card = target.closest(CARD);
  return card instanceof HTMLElement && card.dataset.card !== undefined ? card : null;
}

/** The deck's drop target, when it is drawn (a box of no size is hidden). */
function dropTarget(): HTMLElement | null {
  const drop = document.querySelector(DROP);
  return drop instanceof HTMLElement ? drop : null;
}

/** Over the deck, by its box: the ghost is under the pointer, so a hit test would see the ghost. */
function overDeck(drop: HTMLElement | null, point: Point): boolean {
  if (drop === null) return false;
  const rect = drop.getBoundingClientRect();
  if (!(rect.width > 0 && rect.height > 0)) return false;
  return point.x >= rect.left && point.x <= rect.right && point.y >= rect.top && point.y <= rect.bottom;
}

export default function DeckDrag({ onDrop, nameOf }: DeckDragProps): ReactElement | null {
  const [drawn, setDrawn] = useState<Flight | null>(null);
  // `onDrop` is the editor's `add`, which changes with every card the deck takes: the listeners call the latest,
  // so a drag in flight is never torn down by a change of it.
  const latest = useRef(onDrop);
  useLayoutEffect(() => {
    latest.current = onDrop;
  });

  useEffect(() => {
    const root = document.documentElement;
    let press: Press | null = null;
    let flight: Flight | null = null;
    let target: HTMLElement | null = null;
    /** Armed when a drag ends: the click the release produces is not a tap on the card. */
    let swallowClick = false;

    function mark(point: Point | null): void {
      const over = point !== null && overDeck(target, point);
      if (over) target?.setAttribute("data-drop", "over");
      else target?.removeAttribute("data-drop");
    }

    function stop(): void {
      const ending = flight;
      flight = null;
      press = null;
      if (ending !== null) {
        root.removeAttribute("data-dragging");
        mark(null);
        target = null;
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
      const card = cardAt(event.target);
      if (card === null) return;
      press = { pointerId: event.pointerId, start: { x: event.clientX, y: event.clientY }, card, cardId: card.dataset.card ?? "" };
      // Never preventDefault: below the threshold this press is a click.
    }

    function onPointerMove(event: PointerEvent): void {
      const pointer = { x: event.clientX, y: event.clientY };
      if (flight !== null) {
        if (event.pointerId !== flight.pointerId) return;
        flight = { ...flight, pointer };
        mark(pointer);
        setDrawn(flight);
        return;
      }
      if (press === null || event.pointerId !== press.pointerId) return;
      const across = Math.abs(pointer.x - press.start.x);
      const down = Math.abs(pointer.y - press.start.y);
      if (Math.hypot(across, down) < DRAG_THRESHOLD_PX) return;
      const touch = event.pointerType === "touch";
      const { card, cardId } = press;
      // A touch that starts more down than across is a scroll of the pool, the browser's.
      if (touch && across <= down) {
        press = null;
        return;
      }
      press = null;
      if (!card.isConnected) return;
      // A drag is not an inspection: any preview still open would sit over the drop target.
      closeInspect();
      root.setAttribute("data-dragging", "deck");
      try {
        card.setPointerCapture(event.pointerId);
      } catch {
        // jsdom has no pointer capture, and a pointer that is already gone cannot be captured.
      }
      target = dropTarget();
      flight = { pointerId: event.pointerId, cardId, pointer, touch };
      mark(pointer);
      setDrawn(flight);
    }

    function onPointerUp(event: PointerEvent): void {
      if (flight === null) {
        if (press !== null && event.pointerId === press.pointerId) press = null;
        return;
      }
      if (event.pointerId !== flight.pointerId) return;
      const { cardId } = flight;
      const drop = overDeck(target, { x: event.clientX, y: event.clientY });
      stop();
      if (drop) latest.current(cardId);
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

    function onClickCapture(event: MouseEvent): void {
      if (!swallowClick) return;
      swallowClick = false;
      event.stopPropagation();
      event.preventDefault();
    }

    /** A native drag of the card's art would send `pointercancel` mid-drag. */
    function onDragStart(event: DragEvent): void {
      if (cardAt(event.target) === null) return;
      event.preventDefault();
    }

    /** A finger that has lifted a card must not pan the pool under it. */
    function onTouchMove(event: TouchEvent): void {
      if (flight !== null && event.cancelable) event.preventDefault();
    }

    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("pointermove", onPointerMove, true);
    window.addEventListener("pointerup", onPointerUp, true);
    window.addEventListener("pointercancel", onPointerCancel, true);
    window.addEventListener("click", onClickCapture, true);
    window.addEventListener("dragstart", onDragStart, true);
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("touchmove", onTouchMove, { capture: true, passive: false });
    window.addEventListener("blur", cancel);

    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("pointermove", onPointerMove, true);
      window.removeEventListener("pointerup", onPointerUp, true);
      window.removeEventListener("pointercancel", onPointerCancel, true);
      window.removeEventListener("click", onClickCapture, true);
      window.removeEventListener("dragstart", onDragStart, true);
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("touchmove", onTouchMove, true);
      window.removeEventListener("blur", cancel);
      if (flight !== null) {
        root.removeAttribute("data-dragging");
        mark(null);
      }
      flight = null;
      press = null;
    };
  }, []);

  if (drawn === null) return null;
  const style: CSSProperties = { left: drawn.pointer.x, top: drawn.pointer.y };
  return (
    <div
      className="db-drag-ghost"
      data-testid={DECK_DRAG_GHOST}
      data-card={drawn.cardId}
      data-pointer={drawn.touch ? "touch" : "mouse"}
      aria-hidden="true"
      style={style}
    >
      {nameOf === undefined ? drawn.cardId : nameOf(drawn.cardId)}
    </div>
  );
}
