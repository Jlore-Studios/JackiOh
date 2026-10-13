// The replay player's numbers (#511). The step and page sizes are the engine's (R768); only the
// player's own pacing is stated here, as CLAUDE.md rule 9 asks.

import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";

/**
 * Play's pause after the board has settled, at the table's speed 1: the next step starts this long
 * after the last one's animations and voice line end. The effects speed (R201) scales it.
 */
export const REPLAY_PLAY_GAP_MS = 700;

/** How many steps past the shown one are fetched ahead, so Play never waits on a page: one page's worth. */
export const REPLAY_PREFETCH_STEPS = REPLAY_PAGE_STEPS;
