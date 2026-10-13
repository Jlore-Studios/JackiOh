// The card a drop has just played, while the board catches up (docs/polish/7-mobile-ux.md S9).
//
// A drop sends its `play` at once, but the board keeps showing the view from before it until the
// animation runner has played the play's events (BUILD M5-T4, Game.tsx), about three quarters of a
// second; the card going back into the fan meanwhile read as a refused play. So DragLayer names the
// card here, draws it where it was dropped, and clears the name once the board shows a newer view.
// Hand.tsx reads the name to take that card out of the fan meanwhile.
//
// Module state, because DragLayer and Hand are siblings under Game and neither owns the other. It
// is view state only: nothing here is sent or decides a rule.

import { useSyncExternalStore } from "react";

let landing: string | null = null;
const listeners = new Set<() => void>();

/** The instance id of the card a drop has just played, or null. */
export function currentLanding(): string | null {
  return landing;
}

/** Name the card a drop has just played, or clear it with null. Notifies only on a change. */
export function setLanding(instanceId: string | null): void {
  if (landing === instanceId) return;
  landing = instanceId;
  for (const listener of [...listeners]) listener();
}

export function subscribeLanding(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The card a drop has just played, re-rendering the caller when it changes. */
export function useLanding(): string | null {
  return useSyncExternalStore(subscribeLanding, currentLanding, currentLanding);
}
