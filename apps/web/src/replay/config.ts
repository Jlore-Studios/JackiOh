// The replay player's numbers (#511). The step and page sizes are the engine's (R768); only the
// player's own pacing is stated here, as CLAUDE.md rule 9 asks.

import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";

/** Play's pause after the board's animations and voice line end, at speed 1; the effects speed (R201) scales it. */
export const REPLAY_PLAY_GAP_MS = 700;

/** How many steps past the shown one are fetched ahead, so Play never waits on a page. */
export const REPLAY_PREFETCH_STEPS = REPLAY_PAGE_STEPS;

/**
 * R768000: a finished match's result is written just after its last view is sent, so its replay can
 * answer "not finished" for a moment; the result panel asks this many times in all, this many ms apart.
 */
export const REPLAY_OPEN_ATTEMPTS = 5;
export const REPLAY_OPEN_RETRY_MS = 1000;

// Sharing a replay as a GIF (#513, R513000). The export is drawn on the client from a clip's views
// alone, so its numbers are the client's own.

/** A GIF's frame clock: frames per second for an animation entry's pulse. */
export const REPLAY_GIF_FPS = 10;
/** The slower clock a clip over the byte cap is tried at last: half the frames per second. */
export const REPLAY_GIF_FALLBACK_FPS = 5;
/** How long a step's board is held after its entries have played. */
export const REPLAY_GIF_STEP_HOLD_MS = 1000;
/** How long each animation entry of a step pulses on the board. */
export const REPLAY_GIF_ENTRY_HOLD_MS = 400;
export const REPLAY_GIF_MAX_STEPS = 40;
export const REPLAY_GIF_MAX_SECONDS = 30;
/** A GIF over this many bytes is drawn smaller, then slower, then refused. */
export const REPLAY_GIF_MAX_BYTES = 8 * 1024 * 1024;
/** A download's object URL is let go this long after the click. */
export const REPLAY_GIF_REVOKE_MS = 60_000;

/** The two shapes a GIF is drawn in, in pixels: wide for a screen, square for a feed. */
export const REPLAY_GIF_SIZES = {
  landscape: { width: 800, height: 450 },
  square: { width: 540, height: 540 },
} as const;

export type ReplayGifPreset = keyof typeof REPLAY_GIF_SIZES;

// Opening a downloaded replay file (#1160, R1141000). The file is untrusted input, so it is held to
// named caps before anything of it is kept.

/** A replay file over this many bytes is refused unread: a full game's views are single-digit megabytes. */
export const REPLAY_JSON_MAX_BYTES = 32 * 1024 * 1024;
/** A replay file of more steps than this is refused; the turn cap (`TURN_CAP_PLAYER_TURNS`, 60) keeps a real game to the low thousands. */
export const REPLAY_JSON_MAX_STEPS = 5000;
