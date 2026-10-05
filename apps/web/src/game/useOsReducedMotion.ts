// #258: the OS "reduce motion" preference, followed live.
//
// `prefersReducedMotion()` (animations.ts) reads the media query once. The board builds its
// animation queue from that answer, so without a subscription a player who turns on the OS setting
// mid-game kept full motion until the next page load. This hook re-renders its caller whenever the
// query flips, which is what makes Game.tsx rebuild the queue (as the panel's own switch already
// does). A host without `matchMedia`, or whose query cannot `addEventListener`, reads false or the
// first answer and never changes.

import { useSyncExternalStore } from "react";

import { REDUCED_MOTION_QUERY, prefersReducedMotion } from "./animations.ts";

function queryList(): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  try {
    return window.matchMedia(REDUCED_MOTION_QUERY);
  } catch {
    return null;
  }
}

/** Calls `onChange` each time the OS preference flips; returns the unsubscribe. */
export function subscribeOsReducedMotion(onChange: () => void): () => void {
  const query = queryList();
  if (query === null || typeof query.addEventListener !== "function") return () => undefined;
  query.addEventListener("change", onChange);
  return () => {
    query.removeEventListener("change", onChange);
  };
}

/** Whether the OS asks for reduced motion now, kept current by the media query's `change` event. */
export function useOsReducedMotion(): boolean {
  return useSyncExternalStore(subscribeOsReducedMotion, prefersReducedMotion, () => false);
}
