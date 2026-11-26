// An Infinite Assault fight's screen (issue #4582): the numbers it states (CLAUDE.md rule 9).

import type { Difficulty } from "@jackioh/engine/config";

/**
 * R4582000: the difficulty a fight's start names. `PracticeStartConfig` needs one, but a config with
 * `assault` plays the setup's handicaps, so nothing reads it; Easy is the Assault's own base.
 */
export const ASSAULT_FIGHT_DIFFICULTY: Difficulty = "easy";
