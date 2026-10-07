// The emote client's numbers (CLAUDE.md rule 9): bubble span, the emoji's own 2s show, the menu's
// grey-out tick and its distance from the screen's edge, and the voice-line duration estimate. The
// SEND limits (1.5s cooldown, 5 per 20s)
// are NOT here — they live in the wire layer (`src/wire/emotes.ts`, `@jackioh/shared`, beside
// `crates/engine/src/wire/emotes.rs`) because the server enforces them too (R643).

/** An emoji sticker's whole show: pop, bounce, hold and fade (issue §4, "2s total"). */
export const EMOTE_EMOJI_MS = 2000;

/** A voice-line bubble stays at least this long, whatever the line is (issue §3, "minimum 2s"). */
export const EMOTE_BUBBLE_MIN_MS = 2000;
/** …and never longer (issue §3, "maximum 4s"). */
export const EMOTE_BUBBLE_MAX_MS = 4000;

/** How often an open menu re-reads the limiter, so its greyed items' wait counts down. */
export const EMOTE_MENU_TICK_MS = 250;

/** An open emote menu keeps at least this far inside the viewport's left and right edges (#219). */
export const EMOTE_MENU_EDGE_PX = 8;

/**
 * The persona rate a `say` voice speaks at is words per minute; a SAPI persona's is a percent of
 * baseline, which SAPI itself documents at ~180 wpm. Felinors' echo runs 120ms past the base
 * voice (issue §3's mix), which the bubble holds for.
 */
export const SAPI_BASELINE_WPM = 180;
export const FELINORS_ECHO_TAIL_MS = 120;
