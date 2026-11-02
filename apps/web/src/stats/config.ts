// Every number and key the player statistics state (CLAUDE.md rule 9; SPEC R639), the browser's own: the
// engine owns the rules numbers, the server a game record's (SPEC §9.11, R376). The statistics stay on the device.

import type { MarkColor } from "../cards/marks.ts";

/** localStorage, in stats/store.ts's shape. */
export const PLAYER_STATS_KEY = "jackioh.stats.v1";

/** The stored shape's version; anything else reads as no statistics. */
export const PLAYER_STATS_VERSION = 1;

/** R639: the games logged before the fan deals from every set; until then Core (R374, R704), what a new player learns. */
export const ROTATION_MIN_GAMES = 10;

/** R639: how long the fan holds a card before the next slot swaps, in ms. */
export const ROTATION_INTERVAL_MS = 7000;

/** R704: how long a swap takes, in ms; landing.tsx hands it to landing.css as `--fan-swap`. */
export const ROTATION_SWAP_MS = 1200;

/** R639: a card whose text fits unshrunk (`fit.ts`'s two shortest tiers) is drawn this many times as readily as a dense one. */
export const FEATURE_WEIGHT_PLAIN = 4;

/** R639: the weight of a card whose text needs shrinking. */
export const FEATURE_WEIGHT_DENSE = 1;

/** R639: the longest length tier (`fit.ts`) that still counts as fitting unshrunk. */
export const FEATURE_PLAIN_MAX_TIER = "m";

/**
 * R1017002: a hero portrait's full-art card (cards/fullArt.ts) is drawn this many times as readily in the
 * fan's deal and swaps while the Full-art cards setting is on.
 */
export const FULL_ART_WEIGHT_FACTOR = 2;

/** R1584000: how long the showcase shows one stat, in ms; everything one stat plays fits inside it. */
export const SHOWCASE_INTERVAL_MS = 6000;

/** R1584000: how long a stat's card takes to come in, and back after a death, in ms; landing.tsx hands it to landing.css as `--showcase-lead`. */
export const SHOWCASE_LEAD_MS = 1200;

/** R1584000: the card's wait after each animation, before the next or its rest, in ms. */
export const SHOWCASE_BEAT_GAP_MS = 900;

/** R1743000: a card's plays (games, times faced) before a per-play stat can name it, so one game does not win it. */
export const SHOWCASE_MIN_PLAYS = 5;

/** R2649000: the engine's `FROZEN_MARK` (`config.rs`, R1174000): `marked` with `added: true` as a freeze lands, `added: false` as it thaws. */
export const FROZEN_MARK = "frozen";

/** R2649000: the engine's `FROZEN_MARK` color key (`config.rs`): the frost, as the stage's `freeze` and `thaw` draw it. */
export const FROZEN_MARK_COLOR: MarkColor = "cyan";

/** R2649000: Classic #20 The Power to Punish's mark (R437), the unit marked for death, as the stage's `doom` draws it. */
export const DESTROY_MARK = "destroy";

/** R2649000: Classic #20's mark color: #50's aura, in red (R437). */
export const DESTROY_MARK_COLOR: MarkColor = "red";

/** R1583000: the games logged before the device names a Pocket Pick. */
export const POCKET_PICK_MIN_GAMES = 10;

/** R1583000: a card's plays logged before it can be the Pocket Pick, so one game does not win it. */
export const POCKET_PICK_MIN_PLAYS = 3;

/** R654: the games a card must appear in to show a win rate, not "not enough games". */
export const CARD_STATS_MIN_SAMPLE = 20;


/** R654: the players tab's search debounce, in ms. */
export const PLAYER_SEARCH_DEBOUNCE_MS = 300;

/** R1966000: how many standard errors from 50% a card's win rate must be for the Sets tab to count it strong or weak. */
export const SET_STATS_OUTLIER_Z = 3;

/** R1966000: how many strongest cards, and weakest, the Sets tab lists. */
export const SET_STATS_CARDS_LISTED = 10;

/** R1966000: decimal places of a set's win rates and change (they differ by tenths) and of a card's standard errors from 50%. */
export const SET_STATS_DIGITS = 1;

/** R1973002: the games, live and AI, a patch needs before the patch picker offers it. */
export const STATS_PATCH_MIN_GAMES = 1000;

/** R1973002: how many of the newest patches with enough games the picker offers. */
export const STATS_PATCHES_OFFERED = 5;
