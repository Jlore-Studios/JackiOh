// #258: the OS "reduce motion" preference, followed live.
//
// `prefersReducedMotion()` (animations.ts) reads the media query once. The board builds its
// animation queue from that answer, so without a subscription a player who turns on the OS setting
// mid-game kept full motion until the next page load. This hook re-renders its caller whenever the
// query flips, which is what makes Game.tsx rebuild the queue.

import { useSyncExternalStore } from "react";

import { REDUCED_MOTION_QUERY, prefersReducedMotion } from "./animations.ts";
import { subscribeTo } from "./mediaQuery.ts";

export function subscribeOsReducedMotion(onChange: () => void): () => void {
  return subscribeTo(REDUCED_MOTION_QUERY, onChange);
}

/** Whether the OS asks for reduced motion now, kept current by the media query's `change` event. */
export function useOsReducedMotion(): boolean {
  return useSyncExternalStore(subscribeOsReducedMotion, prefersReducedMotion, () => false);
}
