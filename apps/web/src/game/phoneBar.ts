// #2645: whether the board is on one of its two phone layouts, followed live.
//
// board.css keeps the control bar to one row there, so Offer draw, Concede and Friends sit in the match
// menu (`MatchMenu.tsx`) instead of beside End turn. That is a choice of which controls are drawn, so
// the hook reads the same media query the stylesheet does (`PHONE_BAR_QUERY`) and re-renders its
// caller whenever it flips. It only says which layout is showing: every control decides nothing
// (CLAUDE.md rule 7). A host without `matchMedia` reads false.
//
// #2932: `usePhonePortrait` is the same for the portrait block alone (`PHONE_PORTRAIT_QUERY`): on a
// phone held upright a field card's Activate control and keyword marks hang off its corner
// (`FieldBadges.tsx`) and the match bar rides the top strip (`Game.tsx`). Neither hook caches its
// `MediaQueryList`: a test swaps `window.matchMedia` between renders.

import { useSyncExternalStore } from "react";

import { PHONE_BAR_QUERY, PHONE_PORTRAIT_QUERY } from "./config.ts";

function queryList(text: string): MediaQueryList | null {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return null;
  try {
    return window.matchMedia(text);
  } catch {
    return null;
  }
}

function subscribeTo(text: string, onChange: () => void): () => void {
  const query = queryList(text);
  if (query === null || typeof query.addEventListener !== "function") return () => undefined;
  query.addEventListener("change", onChange);
  return () => {
    query.removeEventListener("change", onChange);
  };
}

function subscribe(onChange: () => void): () => void {
  return subscribeTo(PHONE_BAR_QUERY, onChange);
}

function isPhone(): boolean {
  return queryList(PHONE_BAR_QUERY)?.matches === true;
}

function subscribePortrait(onChange: () => void): () => void {
  return subscribeTo(PHONE_PORTRAIT_QUERY, onChange);
}

function isPortrait(): boolean {
  return queryList(PHONE_PORTRAIT_QUERY)?.matches === true;
}

/** Whether the board is on a phone layout now, kept current by the media query's `change` event. */
export function usePhoneBar(): boolean {
  return useSyncExternalStore(subscribe, isPhone, () => false);
}

/** #2932: whether the board is on the phone portrait layout now (`max-width: 600px`), kept current by the media query's `change` event. */
export function usePhonePortrait(): boolean {
  return useSyncExternalStore(subscribePortrait, isPortrait, () => false);
}
