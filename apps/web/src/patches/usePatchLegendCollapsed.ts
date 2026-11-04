// R3012000: whether the Patch notes legend is on the phone layout, where it collapses into a drawer,
// followed live. patches.css folds the same width (`PATCH_LEGEND_PHONE_QUERY`); the hook only says which
// layout is showing and decides nothing (CLAUDE.md rule 7). A host without `matchMedia` reads false.
// It caches no `MediaQueryList`: a test swaps `window.matchMedia` between renders (as game/phoneBar.ts does).

import { useSyncExternalStore } from "react";

import { PATCH_LEGEND_PHONE_QUERY } from "./constants.ts";

function queryList(): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  try {
    return window.matchMedia(PATCH_LEGEND_PHONE_QUERY);
  } catch {
    return null;
  }
}

function subscribe(onChange: () => void): () => void {
  const query = queryList();
  if (query === null || typeof query.addEventListener !== "function") return () => undefined;
  query.addEventListener("change", onChange);
  return () => {
    query.removeEventListener("change", onChange);
  };
}

function isCollapsed(): boolean {
  return queryList()?.matches === true;
}

/** Whether the legend is collapsed into a drawer now, kept current by the media query's `change` event. */
export function usePatchLegendCollapsed(): boolean {
  return useSyncExternalStore(subscribe, isCollapsed, () => false);
}
