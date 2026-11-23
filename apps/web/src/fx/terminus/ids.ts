// R3338000: the Terminus cards whose set pieces fx/terminus/ draws, by their public ids. Presentation
// only (CLAUDE.md rule 7): no planner reads which of them a hidden card is (R202).

/** Stinkyscape #4 Olivian Expanse, which Discovers a Terminus card. */
export const OLIVIAN_EXPANSE = "stinkyscape-004";
/** Core #100 Ceaseless Void, tagged Terminus (R1837000). */
export const CEASELESS_VOID = "core-100";
/** The set's six Terminus tokens (R1857002). */
export const FAREWELL = "stinkyscape-004-1";
export const KYS_LAST_EXAM = "stinkyscape-004-2";
export const EVENT_HORIZON = "stinkyscape-004-3";
export const LAST_WORD = "stinkyscape-004-4";
export const FINAL_DESTINATION = "stinkyscape-004-5";
export const BIG_CRUNCH = "stinkyscape-004-6";
/** Stinkyscape #154 Dice of Doom, which adds every Terminus card on a 67. */
export const DICE_OF_DOOM = "stinkyscape-154";

/**
 * The first seven Terminus cards, in catalog order, the pool's order (`query::terminus_cards`): the stars
 * Olivian Expanse's rift circles and Dice of Doom's jackpot lights, one each (never the pick).
 */
export const TERMINUS_IDS: readonly string[] = [CEASELESS_VOID, FAREWELL, KYS_LAST_EXAM, EVENT_HORIZON, LAST_WORD, FINAL_DESTINATION, BIG_CRUNCH];

/**
 * Stinkyscape+'s Terminus cards, in catalog order (#3596). Olivian Expanse and Dice of Doom draw them too
 * since #3869 (R3869000), but none has a set piece of its own yet, so the rift and the jackpot light no
 * star for them.
 */
export const STINKYSCAPE_PLUS_TERMINUS_IDS: readonly string[] = [
  "stinkyscapeplus-077",
  "stinkyscapeplus-078",
  "stinkyscapeplus-079",
  "stinkyscapeplus-080",
  "stinkyscapeplus-089",
  "stinkyscapeplus-t-tears-of-the-kwzki-tree",
  "stinkyscapeplus-t-supernova",
  "stinkyscapeplus-t-alnilam",
  "stinkyscapeplus-t-the-enigmata",
];

/** The cards that owe their caster a turn, whose opening plays their follow-up (Event Horizon's, Last Word's). */
export const TURN_FOLLOWS: ReadonlySet<string> = new Set([EVENT_HORIZON, LAST_WORD]);
