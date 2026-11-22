// #4444: the Friends drawer's chunk, owned by the rail button. The drawer (the friends panel,
// the chat panel, the challenge dialog) arrives the first time Friends opens, prefetched when
// pointer or focus shows interest on the rail. Only code is fetched, never game state
// (CLAUDE.md rule 7). This module never value-imports a panel file, so the entry keeps only
// the button, its badge and its polling.

import { routeChunk } from "../routes/routeChunk.tsx";
import { readSession } from "../net/session.ts";
import { savingData } from "../net/saveData.ts";
import type { FriendsDrawerProps } from "./FriendsDrawer.tsx";

/** The drawer content, drawn at once when its code is in, else through `lazy()`. */
export const friendsDrawerChunk = routeChunk<FriendsDrawerProps>(() => import("./FriendsDrawer.tsx"));

/** The drawer chunk once per page load, as `prefetchRoute` holds each route's. */
let prefetched = false;

/** Test seam: a new page. */
export function resetFriendsDrawerPrefetchForTests(): void {
  prefetched = false;
}

/**
 * #4444: intent on the rail starts the drawer chunk, once per page load. Skipped while the
 * browser asks to save data (#4299), and with no session there is no drawer to open. A
 * failure is the drawer's to report.
 */
export function prefetchFriendsDrawer(): void {
  if (prefetched) return;
  if (savingData()) return;
  if (readSession() === null) return;
  prefetched = true;
  void friendsDrawerChunk.load().catch(() => undefined);
}
