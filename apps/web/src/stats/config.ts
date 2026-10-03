// Every number and key the player statistics state (CLAUDE.md rule 9; SPEC R639). These are the
// browser's own: the rules numbers are the engine's, and a game record's (SPEC §9.11, R376) is the
// server's. Player statistics sit on the device and nowhere else.

/** localStorage: `{ v: 1, games, wins, losses, draws, cards }` (stats/store.ts), read and written inside try/catch. */
export const PLAYER_STATS_KEY = "jackioh.stats.v1";

/** The stored shape's version; anything else reads as no statistics. */
export const PLAYER_STATS_VERSION = 1;

/**
 * R639: the games a device has logged before the homescreen rotates through every set. Below it the
 * fan keeps its fixed deal of Core cards (R374), which is what a new player is learning.
 */
export const ROTATION_MIN_GAMES = 10;

/** R639: how long the fan holds a card before the next slot swaps for a freshly dealt one, in ms. */
export const ROTATION_INTERVAL_MS = 7000;

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

/** R639: how many cards each "favourite" list on the statistics card shows. */
export const STATS_TOP_CARDS = 3;

/**
 * R640: live ranked and unranked games required on a patch before public card statistics
 * strictly ignore AI development games.
 */
export const PUBLIC_STATS_MIN_LIVE_GAMES = 1000;

/**
 * R640: minimum sample of games a card must appear in to display a win rate percentage
 * instead of "not enough games".
 */
export const CARD_STATS_MIN_SAMPLE = 20;

