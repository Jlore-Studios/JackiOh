// Every number and key the player statistics state (CLAUDE.md rule 9; SPEC R639). These are the
// browser's own: the rules numbers are the engine's, and a game record's (SPEC §9.11, R376) is the
// server's. Player statistics sit on the device and nowhere else.

import type { MarkColor } from "../cards/marks.ts";

/** localStorage: `{ v: 1, games, wins, losses, draws, cards }` (stats/store.ts), read and written inside try/catch. */
export const PLAYER_STATS_KEY = "jackioh.stats.v1";

/** The stored shape's version; anything else reads as no statistics. */
export const PLAYER_STATS_VERSION = 1;

/**
 * R639: the games a device has logged before the homescreen deals and swaps from every set. Below
 * it the fan deals Core cards (R374), which is what a new player is learning, and swaps among
 * them (R704).
 */
export const ROTATION_MIN_GAMES = 10;

/** R639: how long the fan holds a card before the next slot swaps for a freshly dealt one, in ms. */
export const ROTATION_INTERVAL_MS = 7000;

/** R704: how long a swap takes, in ms: the card going out fizzles away while the new one fades in over it. landing.tsx hands it to landing.css as `--fan-swap`. */
export const ROTATION_SWAP_MS = 1200;

/**
 * R639: a card whose name and rules text fit their boxes at the largest size (`fit.ts`'s two shortest
 * length tiers) is drawn this many times as readily as one that needs the text shrunk. The others
 * keep a weight above 0: they are less likely, never excluded.
 */
export const FEATURE_WEIGHT_PLAIN = 4;

/** R639: the weight of a card whose text needs shrinking to fit. */
export const FEATURE_WEIGHT_DENSE = 1;

/** R639: the longest length tier (`fit.ts`) a name or a rules text may be in and still count as fitting without a shrink. */
export const FEATURE_PLAIN_MAX_TIER = "m";

/**
 * R1017002: a hero portrait's full-art card (cards/fullArt.ts) is drawn this many times as readily
 * as it would be otherwise, in the fan's deal and in its swaps, while the Full-art cards setting is on.
 */
export const FULL_ART_WEIGHT_FACTOR = 2;

/** R1584000: how long the floor's showcase shows one stat before the next, in ms. Everything one stat plays fits inside it. */
export const SHOWCASE_INTERVAL_MS = 6000;

/** R1584000: how long a stat's card takes to come in, and to come back after a death, in ms. landing.tsx hands it to landing.css as `--showcase-lead`. */
export const SHOWCASE_LEAD_MS = 1200;

/** R1584000: how long the card waits after each animation before the next one, and before it rests, in ms. */
export const SHOWCASE_BEAT_GAP_MS = 900;

/** R1743000: the plays (games, times faced) a card needs before a per-play stat can name it, so one game does not win it. */
export const SHOWCASE_MIN_PLAYS = 5;

/** R2649000: the engine's `FROZEN_MARK` (`config.rs`, R1174000): `marked` with `added: true` as a freeze lands, `added: false` as it thaws. */
export const FROZEN_MARK = "frozen";

/** R2649000: the engine's `FROZEN_MARK` color key (`config.rs`): the frost, as the stage's `freeze` and `thaw` draw it. */
export const FROZEN_MARK_COLOR: MarkColor = "cyan";

/** R2649000: Classic #20 The Power to Punish's mark (R437), the unit marked for death, as the stage's `doom` draws it. */
export const DESTROY_MARK = "destroy";

/** R2649000: Classic #20's mark color: #50's aura, in red (R437). */
export const DESTROY_MARK_COLOR: MarkColor = "red";

/** R1583000: the games the device must have logged before it names a Pocket Pick. */
export const POCKET_PICK_MIN_GAMES = 10;

/** R1583000: the plays of a card the device must have logged before it can be the Pocket Pick, so one game does not win it. */
export const POCKET_PICK_MIN_PLAYS = 3;

/**
 * R654: minimum sample of games a card must appear in to display a win rate percentage
 * instead of "not enough games".
 */
export const CARD_STATS_MIN_SAMPLE = 20;


/** R654: how long the players tab waits after the last change to its search before it asks the server, in ms. */
export const PLAYER_SEARCH_DEBOUNCE_MS = 300;

/** R1966000: how many standard errors from 50% a card's win rate must be for the Sets tab to count it strong or weak. */
export const SET_STATS_OUTLIER_Z = 3;

/** R1966000: how many of the strongest cards, and of the weakest, the Sets tab lists. */
export const SET_STATS_CARDS_LISTED = 10;

/** R1966000: the decimal places of a set's win rates and their change, which differ by tenths of a point, and of a card's standard errors from 50%. */
export const SET_STATS_DIGITS = 1;

/** R1973002: the games, live and AI, a patch needs before the patch picker offers it. */
export const STATS_PATCH_MIN_GAMES = 1000;

/** R1973002: how many of the newest patches with `STATS_PATCH_MIN_GAMES` games the patch picker offers. */
export const STATS_PATCHES_OFFERED = 5;
