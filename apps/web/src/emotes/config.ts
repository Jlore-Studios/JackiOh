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

/** The board portrait's reaction to opening its inspect view: the squash and the glint (R1331). */
export const PORTRAIT_REACT_MS = 320;

/** How many motes drift over each portrait's oval (R1332). */
export const PORTRAIT_MOTE_COUNT = 6;
/** One mote's drift, from rising out of the oval's lower half to fading at its top (R1332). */
export const PORTRAIT_MOTE_DRIFT_MS = 6000;
/** One breath of a portrait's light, dim to bright (R1332). */
export const PORTRAIT_BREATH_MS = 4200;
/** A mote starts at least this many percent of the oval inside its edge, on each axis (R1332). */
export const PORTRAIT_MOTE_EDGE_PCT = 14;
/** The smallest mote, in percent of the oval's width (R1332). */
export const PORTRAIT_MOTE_MIN_PCT = 5;
/** The largest mote, in percent of the oval's width (R1332). */
export const PORTRAIT_MOTE_MAX_PCT = 10;
