// Every number the board's client layers state that has no module of its own (CLAUDE.md rule 9);
// the rules numbers are the engine's.

import { PHONE_SHEET_QUERY, over, upTo } from "../breakpoints.ts";

/**
 * R745: the most lines the game log draws for one viewer, its window's included; past it the
 * oldest drop off. Planning measured greedy-vs-greedy games of 9 to 31 turns at 118 to 580 lines.
 */
export const LOG_HISTORY_LIMIT = 1000;

/**
 * R1297000: how many particles a backdrop (`game/Backdrop.tsx`) draws: the most any field has, so
 * no backdrop grows past it and a still backdrop under reduced motion shows the same set.
 */
export const BACKDROP_PARTICLE_MAX = 18;

/** The share of the backdrop's width each particle steps to the right of the one before, in percent, wrapping at 100. */
export const BACKDROP_PARTICLE_X_STEP_PCT = 37;

/** The share of the backdrop's height each particle steps down from the one before, in percent, wrapping at 100. */
export const BACKDROP_PARTICLE_Y_STEP_PCT = 61;

/** How long after the one before each particle's motion starts, in milliseconds, so nothing moves in step. */
export const BACKDROP_PARTICLE_DELAY_STEP_MS = 430;

/**
 * R2388000: the result is written just after the view showing it, so once it has come a ranked
 * match's ranks read carrying no move yet is asked again: this many reads in all, this far apart.
 */
export const MATCH_RANKS_READ_ATTEMPTS = 5;
export const MATCH_RANKS_RETRY_MS = 1000;

/** #2645: board.css's two phone layouts, where the control bar keeps to one row. */
export const PHONE_BAR_QUERY = PHONE_SHEET_QUERY;

/** #2932: board.css's phone portrait block (`@media (max-width: 600px)`), where field cards keep a card shape and the match bar rides the top strip. */
export const PHONE_PORTRAIT_QUERY = upTo("phone");

/** #2877: the desktop layout's media query, where the hero tray shows the most chips. */
export const HERO_DESKTOP_QUERY = over("tablet");
/** #2877: the tablet layout's media query (761–1024px wide), between the phone and desktop layouts. */
export const HERO_TABLET_QUERY = `${over("narrow")} and ${upTo("tablet")}`;

/** #2877: how many entries the hero tray shows on a desktop layout before folding the rest behind `+N`. */
export const HERO_TRAY_DESKTOP_COUNT = 4;
/** #2877: how many entries the hero tray shows on a tablet layout before folding the rest behind `+N`. */
export const HERO_TRAY_TABLET_COUNT = 3;
/** #2877: how many entries the hero tray shows on a phone layout before folding the rest behind `+N`. */
export const HERO_TRAY_PHONE_COUNT = 2;

/** #3347: how long after the one before each card of the opening hand is dealt, in milliseconds (at Speed 1×). */
export const DEAL_STAGGER_MS = 140;

/** #3347: how long one dealt card takes to fly from its deck to its place in the hand, in milliseconds (at Speed 1×). */
export const DEAL_FLIGHT_MS = 320;

/** #3347: how long a deck's riffle plays, in milliseconds (at Speed 1×): the pile's top cards split and interleave once. */
export const RIFFLE_MS = 520;

/** #3347: how many leaves a deck's riffle draws, split between its two halves. */
export const RIFFLE_LEAVES = 6;

/** #3347: the least a hand card's place must have moved, in pixels, for the fan to spring it there instead of leaving it. */
export const HAND_SPRING_MIN_PX = 2;
