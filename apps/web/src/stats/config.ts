// Every number and key the player statistics state (CLAUDE.md rule 9; SPEC R639). These are the
// browser's own: the rules numbers are the engine's, and a game record's (SPEC §9.11, R376) is the
// server's. Player statistics sit on the device and nowhere else.

/** localStorage: `{ v: 1, games, wins, losses, draws, cards }` (stats/store.ts), read and written inside try/catch. */
export const PLAYER_STATS_KEY = "jackioh.stats.v1";

/** The stored shape's version; anything else reads as no statistics. */
export const PLAYER_STATS_VERSION = 1;

/**
 * R639: the games a device has logged before the homescreen deals and swaps from every set. Below
 * it the fan deals Core cards (R374), which is what a new player is learning, and swaps among them
 * (R654).
 */
export const ROTATION_MIN_GAMES = 10;

/** R639: how long the fan holds a card before the next slot swaps for a freshly dealt one, in ms. */
export const ROTATION_INTERVAL_MS = 7000;

/** R654: how long a swap takes, in ms: the card going out fizzles away while the new one fades in over it. landing.tsx hands it to landing.css as `--fan-swap`. */
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

/** R639: how many cards each "favourite" list on the statistics card shows. */
export const STATS_TOP_CARDS = 3;
