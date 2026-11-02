// #2645: whether the board is on one of its two phone layouts, followed live.
//
// board.css keeps the control bar to one row there, so Offer draw, Concede and Friends sit in the match
// menu (`MatchMenu.tsx`) instead of beside End turn. That is a choice of which controls are drawn, so
// the hook reads the same media query the stylesheet does (`PHONE_BAR_QUERY`) and re-renders its
// caller whenever it flips. It only says which layout is showing: every control decides nothing
// (CLAUDE.md rule 7). A host without `matchMedia` reads false.

import { useSyncExternalStore } from "react";

import { PHONE_BAR_QUERY } from "./config.ts";

function queryList(): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  try {
    return window.matchMedia(PHONE_BAR_QUERY);
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

function isPhone(): boolean {
  return queryList()?.matches === true;
}

/** Whether the board is on a phone layout now, kept current by the media query's `change` event. */
export function usePhoneBar(): boolean {
  return useSyncExternalStore(subscribe, isPhone, () => false);
}
