// #2877: which board layout the hero tray is drawn in, followed live.
//
// The tray shows a per-layout allowance of chips (the `HERO_TRAY_*_COUNT` constants beside
// `PHONE_BAR_QUERY` in `config.ts`, CLAUDE.md rule 9), so the hook reads the same media queries
// the stylesheet does and re-renders its caller whenever one flips, in the `phoneBar.ts` pattern.
// It only says which layout is showing: every chip decides nothing (CLAUDE.md rule 7). A host
// without `matchMedia` reads desktop, which shows the most.

import { useSyncExternalStore } from "react";

import {
  HERO_DESKTOP_QUERY,
  HERO_TABLET_QUERY,
  HERO_TRAY_DESKTOP_COUNT,
  HERO_TRAY_PHONE_COUNT,
  HERO_TRAY_TABLET_COUNT,
  PHONE_BAR_QUERY,
} from "./config.ts";

export type HeroLayout = "desktop" | "tablet" | "phone";

/** Which layout the board is on now: phone first (it overlaps the tablet range), then desktop, else tablet. */
function currentLayout(): HeroLayout {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return "desktop";
  try {
    if (window.matchMedia(PHONE_BAR_QUERY).matches === true) return "phone";
    if (window.matchMedia(HERO_DESKTOP_QUERY).matches === true) return "desktop";
    return "tablet";
  } catch {
    return "desktop";
  }
}

function subscribe(onChange: () => void): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return () => undefined;
  const queries = [PHONE_BAR_QUERY, HERO_DESKTOP_QUERY, HERO_TABLET_QUERY];
  const lists: MediaQueryList[] = [];
  for (const query of queries) {
    try {
      lists.push(window.matchMedia(query));
    } catch {
      return () => undefined;
    }
  }
  if (lists.some((list) => typeof list.addEventListener !== "function")) return () => undefined;
  for (const list of lists) list.addEventListener("change", onChange);
  return () => {
    for (const list of lists) list.removeEventListener("change", onChange);
  };
}

/** The board's layout now, kept current by the media queries' `change` events. */
export function useHeroLayout(): HeroLayout {
  return useSyncExternalStore(subscribe, currentLayout, () => "desktop" as HeroLayout);
}

/** How many tray entries the caller's layout shows before the rest fold behind `+N`. */
export function useHeroTrayAllowance(): number {
  const layout = useHeroLayout();
  if (layout === "phone") return HERO_TRAY_PHONE_COUNT;
  if (layout === "desktop") return HERO_TRAY_DESKTOP_COUNT;
  return HERO_TRAY_TABLET_COUNT;
}
