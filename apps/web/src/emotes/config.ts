// The emote client's numbers (CLAUDE.md rule 9). The send limits are in the wire layer
// (`src/wire/emotes.ts`), since the server enforces them too (R643).

/** An emoji sticker's whole show: pop, bounce, hold and fade (issue §4). */
export const EMOTE_EMOJI_MS = 2000;

/** A voice-line bubble stays at least this long, whatever the line is (issue §3). */
export const EMOTE_BUBBLE_MIN_MS = 2000;
/** …and never longer. */
export const EMOTE_BUBBLE_MAX_MS = 4000;

/** How often an open menu re-reads the limiter, so its greyed items' wait counts down. */
export const EMOTE_MENU_TICK_MS = 250;

/** An open emote menu's least gap to the viewport's left and right edges (#219). */
export const EMOTE_MENU_EDGE_PX = 8;

/**
 * A `say` persona's rate is words per minute; a SAPI persona's is a percent of baseline, which SAPI
 * documents at ~180 wpm. Felinors' echo runs past the base voice (issue §3's mix); the bubble holds for it.
 */
export const SAPI_BASELINE_WPM = 180;
export const FELINORS_ECHO_TAIL_MS = 120;

/** The portrait's squash and glint when its menu or inspect view opens (R1331). */
export const PORTRAIT_REACT_MS = 320;

/** How many motes drift over each portrait's oval (R1332). */
export const PORTRAIT_MOTE_COUNT = 6;
/** One mote's drift, from rising in the oval's lower half to fading at its top. */
export const PORTRAIT_MOTE_DRIFT_MS = 6000;
/** One breath of a portrait's light, dim to bright. */
export const PORTRAIT_BREATH_MS = 4200;
/** A mote starts at least this many percent of the oval inside its edge, on each axis. */
export const PORTRAIT_MOTE_EDGE_PCT = 14;
/** The smallest mote, in percent of the oval's width. */
export const PORTRAIT_MOTE_MIN_PCT = 5;
export const PORTRAIT_MOTE_MAX_PCT = 10;
