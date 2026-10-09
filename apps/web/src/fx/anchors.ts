// Where an effect plays, measured the moment it fires (docs/polish/1-animations.md, S8).
//
// Every anchor resolves against the rendered DOM and nothing caches a rectangle: the board re-lays
// itself out between entries, so a stale box lands in the wrong place. A 0×0 rect (not laid out, and
// every element under jsdom) resolves to null so the director skips the cue.
//
// A hand anchor is the box of the cards it holds plus the slot the next card takes, clipped to the
// strip (the strip's centre is empty table); an empty hand is the strip itself.
//
// `boardShakeSink` writes the individual CSS `translate` and `rotate` properties, which compose with
// the board's `transform` keyframes, and restores the inline values the board carried before.

import type { FxAnchor, FxBox, FxPoint, FxShakeOffset, FxShakeSink, FxVec } from "./types.ts";

const CENTRE: FxPoint = { x: 0.5, y: 0.5 };

/** `CSS.escape` does not exist under jsdom; a testid only needs its quotes and backslashes escaped. */
function attrValue(value: string): string {
  return value.replace(/["\\]/g, "\\$&");
}

/** The element's viewport rectangle, or null when there is no element or it has no size yet. */
function boxOf(element: Element | null | undefined): FxBox | null {
  if (element === null || element === undefined) return null;
  const rect = element.getBoundingClientRect();
  if (rect.width === 0 && rect.height === 0) return null;
  return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
}

const HAND_TESTIDS: ReadonlySet<string> = new Set(["hand-you", "hand-opponent"]);

/** A hand's cards plus the slot after the last one, inside the strip (see the header). */
function handBox(hand: Element): FxBox | null {
  const strip = boxOf(hand);
  if (strip === null) return null;
  const cards: FxBox[] = [];
  for (const card of Array.from(hand.querySelectorAll(".card"))) {
    const box = boxOf(card);
    if (box !== null) cards.push(box);
  }
  const last = cards[cards.length - 1];
  if (last === undefined) return strip;
  const previous = cards[cards.length - 2];
  // The next card lands one step on from the last: the hand's own spacing, or one card's width.
  const step = previous !== undefined && last.x > previous.x ? last.x - previous.x : last.width;
  let left = last.x;
  let top = last.y;
  let right = last.x + last.width + step;
  let bottom = last.y + last.height;
  for (const box of cards) {
    left = Math.min(left, box.x);
    top = Math.min(top, box.y);
    right = Math.max(right, box.x + box.width);
    bottom = Math.max(bottom, box.y + box.height);
  }
  left = Math.max(left, strip.x);
  right = Math.min(right, strip.x + strip.width);
  if (!(right > left)) return strip;
  return { x: left, y: top, width: right - left, height: bottom - top };
}

/**
 * testid → the element's rect (a hand: see the header); crystal → the index-th `.mana-crystal` in
 * `mana-<side>` (else the tray); handCard → the `pick`-th card of `hand-<side>` modulo its cards
 * (else the hand); viewport → a zero-size box at (innerWidth·at.x, innerHeight·at.y).
 */
export function resolveAnchor(anchor: FxAnchor, doc?: Document, win?: Window): FxBox | null {
  if (anchor.kind === "viewport") {
    const view = win ?? window;
    return { x: view.innerWidth * anchor.at.x, y: view.innerHeight * anchor.at.y, width: 0, height: 0 };
  }

  const page = doc ?? document;

  if (anchor.kind === "testid") {
    const element = page.querySelector(`[data-testid="${attrValue(anchor.testid)}"]`);
    if (element !== null && HAND_TESTIDS.has(anchor.testid)) return handBox(element);
    return boxOf(element);
  }

  if (anchor.kind === "handCard") {
    const hand = page.querySelector(`[data-testid="hand-${anchor.side}"]`);
    if (hand === null) return null;
    const cards = hand.querySelectorAll(".card");
    if (cards.length === 0) return handBox(hand);
    const at = ((Math.trunc(anchor.pick) % cards.length) + cards.length) % cards.length;
    return boxOf(cards[at]) ?? handBox(hand);
  }

  const tray = page.querySelector(`[data-testid="mana-${anchor.side}"]`);
  if (tray === null) return null;
  const crystal = tray.querySelectorAll(".mana-crystal")[anchor.index];
  return boxOf(crystal) ?? boxOf(tray);
}

/** The point `at` inside `box`, as fractions of its width and height; the centre by default. */
export function pointIn(box: FxBox, at?: FxPoint): FxVec {
  const p = at ?? CENTRE;
  return { x: box.x + box.width * p.x, y: box.y + box.height * p.y };
}

type SavedInline = { translate: string; rotate: string; translatePriority: string; rotatePriority: string };

function restoreProperty(style: CSSStyleDeclaration, name: string, value: string, priority: string): void {
  if (value === "") style.removeProperty(name);
  else style.setProperty(name, value, priority);
}

/** CSS `translate`/`rotate` on [data-testid="board"] while shaking; restores the prior inline values on clear(). */
export function boardShakeSink(doc?: Document): FxShakeSink {
  let target: HTMLElement | null = null;
  let saved: SavedInline | null = null;

  const restore = (): void => {
    if (target !== null && saved !== null) {
      restoreProperty(target.style, "translate", saved.translate, saved.translatePriority);
      restoreProperty(target.style, "rotate", saved.rotate, saved.rotatePriority);
    }
    target = null;
    saved = null;
  };

  return {
    apply(offset: FxShakeOffset): void {
      const page = doc ?? document;
      const board = page.querySelector<HTMLElement>('[data-testid="board"]');
      if (board === null) {
        restore();
        return;
      }
      if (board !== target) {
        // A new board element (a remount, a hand-over): the old one gets its values back first.
        restore();
        target = board;
        saved = {
          translate: board.style.getPropertyValue("translate"),
          rotate: board.style.getPropertyValue("rotate"),
          translatePriority: board.style.getPropertyPriority("translate"),
          rotatePriority: board.style.getPropertyPriority("rotate"),
        };
      }
      board.style.setProperty("translate", `${offset.x.toFixed(2)}px ${offset.y.toFixed(2)}px`);
      board.style.setProperty("rotate", `${offset.angle.toFixed(3)}deg`);
    },
    clear(): void {
      restore();
    },
  };
}
