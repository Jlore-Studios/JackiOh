// Tutorial numbers and keys obey CLAUDE.md rule 9; the opponent handicap remains engine-owned
// (`AI_TUTORIAL` in crates/engine/src/config.rs, R290).

import { PRACTICE_SHOWCASE_HOLD_MAX_MS } from "../practice/config.ts";

/** Retire stale non-final steps so a passed moment cannot strand the lesson. */
export const TUTORIAL_STEP_TURNS_MAX = 2;

/** `progress.ts` reads and writes this localStorage record in try/catch. */
export const TUTORIAL_PROGRESS_KEY = "jackioh.tutorial.v1";

/** Any other stored version means no progress. */
export const TUTORIAL_PROGRESS_VERSION = 1;

export const COACH_RING_PAD_PX = 6;

export const COACH_BUBBLE_GAP_PX = 12;

export const COACH_VIEWPORT_MARGIN_PX = 8;

/** Re-measure animated or hovered cards, which need not emit a resize. */
export const COACH_TRACK_INTERVAL_MS = 250;

/** Dock on phone layouts so the bubble cannot cover board cards (docs/polish/7-mobile-ux.md S7). */
export const COACH_DOCK_QUERY = "(max-width: 600px), (orientation: landscape) and (max-height: 500px)";

/** Preserve room for a floating bubble's title and actions. */
export const COACH_BUBBLE_MIN_HEIGHT_PX = 96;

/** Match the practice showcase cap so an uncleared mark cannot freeze the coach. */
export const COACH_SHOWCASE_WAIT_MAX_MS = PRACTICE_SHOWCASE_HOLD_MAX_MS;
