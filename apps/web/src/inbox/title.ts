// R4039001: the tab's title counts what waits on the player while the tab is hidden, "(3) Collection · JackiOh",
// so a player in another tab sees it; it is put back as the tab shows. A board's title says whose turn it is
// and the Daily screen's says "Your move (n)" (`routes/match.tsx`, `daily/DailyRoute.tsx`), so neither gains a
// count. The count is the bell's (`InboxBell.tsx`): what the server said is waiting.

import { useEffect } from "react";

import { currentPath, isBoardPath, paths } from "../net/navigate.ts";

/** `base` with the count before it, or `base` alone at zero. */
export function titleWithCount(base: string, count: number): string {
  return count > 0 ? `(${String(count)}) ${base}` : base;
}

/**
 * The screen's title and the one with the count, while the count is on. One for the document, so two bells
 * mounted at once (the top bar's and the landing's) put the count on once.
 */
let held: { base: string; counted: string } | null = null;

/** Whether the screen at `path` keeps its own title. */
function ownTitle(path: string): boolean {
  return isBoardPath(path) || path === paths.daily;
}

/** While the tab is hidden, the title carries `count` (0 shows none); the screen's own title comes back as it shows. */
export function useHiddenTitleCount(count: number): void {
  useEffect(() => {
    const restore = (): void => {
      // A screen that set its own title meanwhile keeps it.
      if (held !== null && document.title === held.counted) document.title = held.base;
      held = null;
    };
    const apply = (): void => {
      if (document.visibilityState !== "hidden" || count === 0 || ownTitle(currentPath())) {
        restore();
        return;
      }
      if (held !== null && document.title === held.counted) return;
      const base = document.title;
      held = { base, counted: titleWithCount(base, count) };
      document.title = held.counted;
    };
    apply();
    document.addEventListener("visibilitychange", apply);
    return () => {
      document.removeEventListener("visibilitychange", apply);
      restore();
    };
  }, [count]);
}
