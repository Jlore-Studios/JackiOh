// A hand that springs (#3347): when a card leaves or arrives, the fan reflows and each card that moved
// slides from where it was to where it now sits, on board.css's `translate` transition (an overshoot at
// full card motion), instead of jumping. FLIP: read each slot's layout place after every render, set the
// slot back at its old place with no transition, force a layout, and let go. View state only (CLAUDE.md rule 7).

import { useLayoutEffect, useRef, type RefObject } from "react";

import { HAND_SPRING_MIN_PX } from "./config.ts";

/**
 * `before − after` for each key in both maps whose move is at least `minPx` either way: how far back from
 * its new place a card has to start to look as if it never moved. A key that is new or gone has no
 * place to spring from, and a nudge under `minPx` is rounding, so neither is returned.
 */
export function flipDeltas(
  before: ReadonlyMap<string, number>,
  after: ReadonlyMap<string, number>,
  minPx: number,
): Map<string, number> {
  const deltas = new Map<string, number>();
  for (const [key, left] of after) {
    const was = before.get(key);
    if (was === undefined) continue;
    const delta = was - left;
    if (Math.abs(delta) >= minPx) deltas.set(key, delta);
  }
  return deltas;
}

/**
 * Springs the `.hand-slot`s under `root` after each render. `keys[i]` names slot i (a card's instance id,
 * or a back's index), so no attribute is added to the DOM. The place is `offsetLeft`, the layout's own, which
 * the slot's tilt, lift and spring leave alone, so a hover or a spring in flight is never mistaken for a move.
 */
export function useHandSpring(root: RefObject<HTMLElement | null>, keys: readonly string[]): void {
  const places = useRef<Map<string, number>>(new Map());
  // No dependency list: the hand reflows on any render (a card leaving, arriving, the fan's step changing).
  useLayoutEffect(() => {
    const element = root.current;
    if (element === null) return;
    const slots = Array.from(element.querySelectorAll<HTMLElement>(".hand-slot"));
    const after = new Map<string, number>();
    keys.forEach((key, i) => {
      const slot = slots[i];
      if (slot !== undefined) after.set(key, slot.offsetLeft);
    });
    const moved = flipDeltas(places.current, after, HAND_SPRING_MIN_PX);
    places.current = after;
    if (moved.size === 0) return;
    const sprung: HTMLElement[] = [];
    keys.forEach((key, i) => {
      const slot = slots[i];
      const delta = moved.get(key);
      if (slot === undefined || delta === undefined) return;
      slot.style.transition = "none";
      slot.style.translate = `${String(delta)}px 0`;
      sprung.push(slot);
    });
    // One layout for all of them, then let go: the CSS transition runs each from its old place.
    void element.offsetWidth;
    for (const slot of sprung) {
      slot.style.transition = "";
      slot.style.translate = "";
    }
  });
}
