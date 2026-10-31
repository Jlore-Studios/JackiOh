// Tracks dropped card in flight until animation catches up (docs/polish/7-mobile-ux.md S9, BUILD M5-T4).
// DragLayer sets dropped instanceId so Hand.tsx removes it from the fan before arrival of the new view.

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
