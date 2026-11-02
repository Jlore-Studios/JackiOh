// The dungeon run's page and its device store (DR14 part D, #2705): the keys, the version and the
// counts this part states (CLAUDE.md rule 9). The run's own numbers (the slots, the artifact limits,
// the level costs, the seed's length) are `crates/run`'s, which the client reads through
// `wasm/index.ts` (`runConstants`, `runLevelCost`, `runSeed`) and states none of.

/** DR14: where the Sandbox keeps the player's unlocks and Embers on this device. */
export const DUNGEON_PROGRESS_KEY = "jackioh.dungeon.progress";
/** DR14: the run in progress, as its `RunFile`, which `runFold` replays on load (B19). */
export const DUNGEON_RUN_KEY = "jackioh.dungeon.run";
/** DR14: every finished run's `RunRecord`, oldest first, which the stats page reads. */
export const DUNGEON_HISTORY_KEY = "jackioh.dungeon.history";

/** The shape of what the three keys hold; a record of another version reads as nothing stored. */
export const DUNGEON_STORAGE_VERSION = 1;

/** DR3: the starting deck's level before any is bought (`RunUnlocks.level` counts from 1). */
export const DUNGEON_FIRST_LEVEL = 1;
/** DR3: one purchase raises the starting deck by one level. */
export const DUNGEON_LEVEL_STEP = 1;

/** DR2: the day's first daily run is run number 1; each later one appends the next number. */
export const DUNGEON_FIRST_RUN = 1;
/** DR2: a started daily run moves the day's number on by one. */
export const DUNGEON_RUN_STEP = 1;

/** DR9: a run's row is shown counting from 1, as the map numbers its rows; the state counts from 0. */
export const DUNGEON_ROW_OFFSET = 1;

/** A run's elapsed time is never negative, whatever the clock did between its start and its end. */
export const DUNGEON_ELAPSED_MIN_MS = 0;

/** B15: the date a daily seed hashes is written `YYYY-MM-DD`, as `runDailySeed` reads it. */
export const DUNGEON_DATE_FORMAT = "YYYY-MM-DD";
