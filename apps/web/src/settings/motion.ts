// Reduce motion, read at each call: the media query, the settings panel's switch, the effects
// store's `motion: "reduce"`. It lives here so the router (`net/navigate.ts`, R2117000), which the
// Vite config loads, can read it without the animation table's imports.

import { getFxSettings, type FxSettings } from "../fx/settings.ts";
import { readSettings as readPanelSettings } from "./store.ts";

export const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

/** Reads the media query, tolerating a non-browser host and a host without `matchMedia`. */
export function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  const mm = (window as Window & { matchMedia?: (q: string) => MediaQueryList }).matchMedia;
  if (typeof mm !== "function") return false;
  try {
    return mm.call(window, REDUCED_MOTION_QUERY).matches === true;
  } catch {
    return false;
  }
}

/**
 * `prefersReducedMotion()` OR the panel's "Reduce motion" switch OR the effects store's `motion: "reduce"`
 * (`settings` defaults to `getFxSettings()`). Either setting behaves exactly like the media query (R200, R201).
 */
export function reducedMotionNow(settings?: Pick<FxSettings, "motion">): boolean {
  return (
    prefersReducedMotion() ||
    readPanelSettings().reduceMotion ||
    (settings ?? getFxSettings()).motion === "reduce"
  );
}
