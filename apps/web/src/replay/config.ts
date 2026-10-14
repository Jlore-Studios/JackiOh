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

/**
 * R768000: a finished match's result is written just after its last view is sent, so its replay can
 * answer "not finished" for a moment. Opening it from the result panel asks again this many times in
 * all, this many milliseconds apart.
 */
export const REPLAY_OPEN_ATTEMPTS = 5;
export const REPLAY_OPEN_RETRY_MS = 1000;

// Sharing a replay as a GIF (#513, R513000). The export is drawn on the client from a clip's views
// alone, so its numbers are the client's own.

/** A GIF's frame clock: frames per second for an animation entry's pulse. */
export const REPLAY_GIF_FPS = 10;
/** The slower clock a clip over the byte cap is tried at last: half the frames per second. */
export const REPLAY_GIF_FALLBACK_FPS = 5;
/** How long a step's board is held, in milliseconds, after its entries have played. */
export const REPLAY_GIF_STEP_HOLD_MS = 1000;
/** How long each animation entry of a step pulses on the board, in milliseconds. */
export const REPLAY_GIF_ENTRY_HOLD_MS = 400;
/** A clip holds at most this many steps. */
export const REPLAY_GIF_MAX_STEPS = 40;
/** A clip plays for at most this many seconds. */
export const REPLAY_GIF_MAX_SECONDS = 30;
/** A GIF over this many bytes is drawn smaller, then slower, then refused. */
export const REPLAY_GIF_MAX_BYTES = 8 * 1024 * 1024;
/** A download's object URL is let go this many milliseconds after the click. */
export const REPLAY_GIF_REVOKE_MS = 60_000;

/** The two shapes a GIF is drawn in, in pixels: wide for a screen, square for a feed. */
export const REPLAY_GIF_SIZES = {
  landscape: { width: 800, height: 450 },
  square: { width: 540, height: 540 },
} as const;

export type ReplayGifPreset = keyof typeof REPLAY_GIF_SIZES;
