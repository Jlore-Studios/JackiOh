// Every number the board's client layers state that has no module of its own (CLAUDE.md rule 9);
// the rules numbers are the engine's.

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

/** R1297000: the share of the backdrop's width each particle steps to the right of the one before, in percent, wrapping at 100. */
export const BACKDROP_PARTICLE_X_STEP_PCT = 37;

/** R1297000: the share of the backdrop's height each particle steps down from the one before, in percent, wrapping at 100. */
export const BACKDROP_PARTICLE_Y_STEP_PCT = 61;

/** R1297000: how long after the one before each particle's motion starts, in milliseconds, so nothing moves in step. */
export const BACKDROP_PARTICLE_DELAY_STEP_MS = 430;
