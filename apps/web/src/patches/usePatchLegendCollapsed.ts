// R3012000: whether the Patch notes legend is on the phone layout, where it collapses into a drawer,
// followed live. patches.css folds the same width (`PATCH_LEGEND_PHONE_QUERY`); the hook only says which
// layout is showing and decides nothing (CLAUDE.md rule 7). A host without `matchMedia` reads false.
// It caches no `MediaQueryList`: a test swaps `window.matchMedia` between renders (game/mediaQuery.ts).

import { useSyncExternalStore } from "react";

import { queryList, subscribeTo } from "../game/mediaQuery.ts";
import { PATCH_LEGEND_PHONE_QUERY } from "./constants.ts";

function subscribe(onChange: () => void): () => void {
  return subscribeTo(PATCH_LEGEND_PHONE_QUERY, onChange);
}

function isCollapsed(): boolean {
  return queryList(PATCH_LEGEND_PHONE_QUERY)?.matches === true;
}

/** Whether the legend is collapsed into a drawer now, kept current by the media query's `change` event. */
export function usePatchLegendCollapsed(): boolean {
  return useSyncExternalStore(subscribe, isCollapsed, () => false);
}
