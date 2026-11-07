// The card a drop has just played, while the board catches up (docs/polish/7-mobile-ux.md S9).
//
// A drop sends its `play` at once, but the board keeps showing the view from before it until the
// animation runner has played the play's events (BUILD M5-T4, Game.tsx); the card going back into
// the fan meanwhile read as a refused play. So DragLayer names the card here and clears the name once
// the board shows a newer view; Hand.tsx reads the name to take that card out of the fan meanwhile.
//
// Module state, because DragLayer and Hand are siblings under Game and neither owns the other. It
// is view state only: nothing here is sent or decides a rule.

import { useSyncExternalStore } from "react";

export type DropPoint = { x: number; y: number };

let landing: string | null = null;
let released: DropPoint | null = null;
const listeners = new Set<() => void>();

export function currentLanding(): string | null {
  return landing;
}

/**
 * Name the card a drop has just played, or clear it with null. Notifies only on a change.
 * #3347: `point` is where the pointer let go, so the fx layer's stand-in flies on from there
 * (`dropPointOfTestid`) rather than from the card's place in the fan.
 */
export function setLanding(instanceId: string | null, point?: DropPoint): void {
  released = instanceId === null || point === undefined ? null : { x: point.x, y: point.y };
  if (landing === instanceId) return;
  landing = instanceId;
  for (const listener of [...listeners]) listener();
}

/** #3347: where the drag of the card the board is landing let go, when `testid` is that card's hand slot; else null. */
export function dropPointOfTestid(testid: string): DropPoint | null {
  if (landing === null || released === null || testid !== `hand-card-${landing}`) return null;
  return { x: released.x, y: released.y };
}

export function subscribeLanding(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useLanding(): string | null {
  return useSyncExternalStore(subscribeLanding, currentLanding, currentLanding);
}
