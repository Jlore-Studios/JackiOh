// The Infinite Assault screen's client-owned names and numbers (CLAUDE.md rule 9). The mode's
// levels, Commanders, checkpoints and history length come from `assaultConstants()` instead.

/** The release flips this on; before then the route exists but no player-facing entry points do. */
export let ASSAULT_OPEN = false;

/** Test seam for both sides of the release gate. */
export function setAssaultOpenForTests(open: boolean): void {
  ASSAULT_OPEN = open;
}

/** The open attempt's `AssaultFile` and part D's kept fight. */
export const ASSAULT_ATTEMPT_KEY = "jackioh.assault.attempt";
/** The records of the days this device keeps. */
export const ASSAULT_RECORDS_KEY = "jackioh.assault.records";
/** The shape of both device values. Another version reads as nothing stored. */
export const ASSAULT_STORAGE_VERSION = 1;
/** The UTC date format `assaultDay` accepts. */
export const ASSAULT_DATE_FORMAT = "YYYY-MM-DD";
/** Levels start at one. */
export const ASSAULT_FIRST_LEVEL = 1;
/** Attempts start at one and move by one. */
export const ASSAULT_FIRST_ATTEMPT = 1;
export const ASSAULT_ATTEMPT_STEP = 1;
/** A Commander shows its two signature cards. */
export const ASSAULT_COMMANDER_CARDS_SHOWN = 2;

/** UTC-day countdown units and its clock's two-digit minute and second fields. */
export const ASSAULT_SECONDS_PER_MINUTE = 60;
export const ASSAULT_MINUTES_PER_HOUR = 60;
export const ASSAULT_HOURS_PER_DAY = 24;
export const ASSAULT_MILLISECONDS_PER_SECOND = 1000;
export const ASSAULT_CLOCK_DIGITS = 2;

/** The five non-Token rarities in an `AssaultLevel.rarities` tuple. */
export const ASSAULT_RARITIES = ["Common", "Rare", "Epic", "Legendary", "Mythic"] as const;
