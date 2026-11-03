// A dungeon run's fight screen (DR14 part E, #2706): the numbers it states (CLAUDE.md rule 9).

import type { Difficulty } from "@jackioh/engine/config";

import { MS_PER_SECOND } from "../../game/clockConstants.ts";

/** B24: how often the run's timer is read again; it shows whole seconds. */
export const FIGHT_TIMER_TICK_MS = MS_PER_SECOND;

/** B24: the timer reads minutes and seconds. */
export const SECONDS_PER_MINUTE = 60;

/** B24: the seconds are always written with this many digits ("1:05"). */
export const CLOCK_DIGITS = 2;

/**
 * R2706000: the difficulty a fight's start names. `PracticeStartConfig` needs one, but a config with
 * `run` plays the setup's handicaps, so nothing reads it; Easy is the run's own base (DR5).
 */
export const FIGHT_DIFFICULTY: Difficulty = "easy";
