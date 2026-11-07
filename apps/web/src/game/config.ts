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

/**
 * #3341: each board layout's card height, `clamp(minPx, (100cqh - fixedPx) / rows, maxPx)`: `fixedPx`
 * is what the layout spends on everything but cards (gaps, seats, controls, padding), `rows` how many
 * card heights it stacks (the field's four rows and the hand's share). Mirrored in `game/board.css`
 * (`--card-h`); `game/cardSizes.test.ts` holds the two together. The desktop also keeps ten hand
 * cards side by side (B30): `(100cqw - widthFixedPx) / widthRows`, the width the 250px sidebar,
 * the column gap and nine `--gap`s leave. `base` is the one-column board of 601–760px, `tablet` the
 * upright tablet, `phone` the portrait phone and `landscape` the phone on its side.
 */
export const BOARD_CARD_BUDGET = {
  desktop: { minPx: 56, fixedPx: 203, rows: 5.48, maxPx: 119, widthFixedPx: 316, widthRows: 10.66 },
  tablet: { minPx: 60, fixedPx: 355, rows: 5.3, maxPx: 113 },
  base: { minPx: 60, fixedPx: 330, rows: 5.88, maxPx: 112 },
  phone: { minPx: 44, fixedPx: 330, rows: 5.24, maxPx: 94 },
  landscape: { minPx: 40, fixedPx: 28, rows: 4.73, maxPx: 72 },
} as const;

/**
 * #3341: how much bigger a lifted hand card is drawn than at rest, per layout (`--hand-lift-scale`
 * in `game/board.css`; `game/cardSizes.test.ts` holds the two together).
 */
export const HAND_LIFT_SCALE = { desktop: 1.22, tablet: 1.14, phone: 1.9, landscape: 1.7 } as const;

/**
 * #3341: the share of the hand card's height a lifted card rises (`--hand-lift-rise` in
 * `game/board.css`). The phone on its side has no height to spare, so its lift stays put.
 */
export const HAND_LIFT_RISE = { desktop: 0.1, tablet: 0.12, phone: 0.4 } as const;
