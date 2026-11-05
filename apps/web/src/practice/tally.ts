// R639, R2777003: which practice games the player's own statistics (stats/useGameStats.ts) count.

import type { Difficulty } from "@jackioh/engine/config";

import { PRACTICE_TALLY_DIFFICULTY } from "./config.ts";

/** What of a practice game's setup decides whether it is tallied. */
export type TalliedSetup = {
  difficulty: Difficulty;
  lesson?: unknown;
  sandbox?: boolean;
};

/**
 * Whether the player's own statistics count this practice game: R639's, a tutorial lesson aside; R923000: not a
 * sandbox game, which counts for nothing; and R2777003: only a game
 * against Easy, the AI on a person's resources, so no Medium or Hard game moves the player's win rates.
 */
export function talliesPractice(setup: TalliedSetup): boolean {
  return (
    setup.lesson === undefined &&
    setup.sandbox !== true &&
    setup.difficulty === PRACTICE_TALLY_DIFFICULTY
  );
}
