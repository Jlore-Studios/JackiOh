// A Marvel Snap-style card wheel (issue #124): the top card stands forward as the primary one, the
// card under it peeks out to the right, the bottom card peeks out to the left, and the viewer
// shuffles through by buttons, scroll, tap, swipe or keyboard.
//
// Generic over what the cards are — a Stack pile (a face over backs), related cards, anything with
// a face or a back — so other displays can reuse it. It enforces nothing and knows no rules: items
// in, an index out.

import { useCallback, useRef, useState, type ReactElement } from "react";

import { CardBack } from "../CardBack.tsx";
import { CardFace } from "../CardFace.tsx";
import type { FaceModel } from "../model.ts";

import "./wheel.css";

/** One card on the wheel: a face, or a back (`face: null`, whose name stays unknown). */
export type WheelItem = {
  /** Stable key for what it shows. */
  key: string;
  face: FaceModel | null;
  /** What a screen reader calls it ("Bigot", "Unknown card"). */
  name: string;
  /** Where it sits, in words ("Top of pile", "Buried", "Bottom of pile"); shown under the primary. */
  note?: string;
};

export type CardWheelProps = {
  /** The cards in order, top (above) first. At least one. */
  items: readonly WheelItem[];
  /** The card standing forward at first. */
  defaultIndex?: number;
  /** What the wheel is, in words: "Stack pile". Names the dialog for assistive tech. */
  ariaLabel: string;
  /** Reports the primary card's index as the viewer shuffles. */
  onIndex?: (index: number) => void;
};

/** Test ids of the wheel. Every id starts with `wheel-`, never `card-` or `inspect-`. */
export const wheelTestid = {
  wheel: "wheel",
  card: "wheel-card",
  prev: "wheel-prev",
  next: "wheel-next",
  position: "wheel-position",
  note: "wheel-note",
} as const;

/** How far a swipe must travel to turn the wheel, in pixels. */
export const WHEEL_SWIPE_PX = 24;

function wrap(index: number, length: number): number {
  return ((index % length) + length) % length;
}

/**
 * Where an item stands beside the primary: 0 under it, -1 peeking left, +1 peeking right, and
 * anything else off the wheel. Past the ends the wheel wraps, so at the top the bottom card peeks
 * left and at the bottom the top card peeks right.
 */
function displayOffset(at: number, index: number, length: number): number {
  const raw = at - index;
  // Two cards are just neighbours; three or more wrap around the ends.
  if (length > 2) {
    if (raw === length - 1) return -1;
    if (raw === 1 - length) return 1;
  }
  return raw;
}

export default function CardWheel({ items, defaultIndex = 0, ariaLabel, onIndex }: CardWheelProps): ReactElement {
  const [index, setIndex] = useState(() => (items.length === 0 ? 0 : wrap(defaultIndex, items.length)));
  const touchX = useRef<number | null>(null);
  const single = items.length <= 1;

  const go = useCallback(
    (next: number) => {
      if (items.length === 0) return;
      const wrapped = wrap(next, items.length);
      if (wrapped !== index) {
        setIndex(wrapped);
        onIndex?.(wrapped);
      }
    },
    [index, items.length, onIndex],
  );

  const current = items[index] ?? items[0];
  return (
    <div
      className="card-wheel"
      data-testid={wheelTestid.wheel}
      role="group"
      aria-roledescription="carousel"
      aria-label={ariaLabel}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft") go(index - 1);
        else if (event.key === "ArrowRight") go(index + 1);
        else if (event.key === "Home") go(0);
        else if (event.key === "End") go(items.length - 1);
        else return;
        event.preventDefault();
      }}
      onWheel={(event) => {
        // A scroll turns the wheel the way a page would go: down for deeper.
        if (event.deltaY === 0 && event.deltaX === 0) return;
        go(index + (event.deltaY + event.deltaX > 0 ? 1 : -1));
      }}
      onTouchStart={(event) => {
        touchX.current = event.touches[0]?.clientX ?? null;
      }}
      onTouchEnd={(event) => {
        const from = touchX.current;
        touchX.current = null;
        if (from === null) return;
        const dx = (event.changedTouches[0]?.clientX ?? from) - from;
        if (Math.abs(dx) >= WHEEL_SWIPE_PX) go(index + (dx < 0 ? 1 : -1));
      }}
    >
      <button type="button" className="card-wheel-turn" data-testid={wheelTestid.prev} aria-label="Previous card" onClick={() => go(index - 1)} disabled={single}>
        ‹
      </button>
      <div className="card-wheel-stage" aria-live="polite">
        {items.map((item, at) => {
          const offset = displayOffset(at, index, items.length);
          if (offset !== 0) {
            const near = Math.abs(offset) === 1;
            return (
              <button
                key={item.key}
                type="button"
                className="card-wheel-card card-wheel-card--peek"
                data-testid={wheelTestid.card}
                data-offset={offset}
                data-face={item.face === null ? "back" : "face"}
                aria-label={`Show ${item.name}`}
                aria-hidden={near ? undefined : true}
                tabIndex={near ? 0 : -1}
                onClick={() => go(at)}
              >
                {item.face === null ? <CardBack /> : <CardFace face={item.face} layout="full" />}
              </button>
            );
          }
          return (
            <div
              key={item.key}
              className="card-wheel-card card-wheel-card--primary"
              data-testid={wheelTestid.card}
              data-offset={offset}
              data-face={item.face === null ? "back" : "face"}
            >
              {item.face === null ? <CardBack /> : <CardFace face={item.face} layout="full" />}
            </div>
          );
        })}
      </div>
      <button type="button" className="card-wheel-turn" data-testid={wheelTestid.next} aria-label="Next card" onClick={() => go(index + 1)} disabled={single}>
        ›
      </button>
      <div className="card-wheel-meta">
        <span className="card-wheel-position" data-testid={wheelTestid.position} data-index={index} data-count={items.length}>
          {`${String(index + 1)} of ${String(items.length)}`}
        </span>
        {current?.note === undefined ? null : (
          <span className="card-wheel-note" data-testid={wheelTestid.note}>
            {current.note}
          </span>
        )}
      </div>
    </div>
  );
}
