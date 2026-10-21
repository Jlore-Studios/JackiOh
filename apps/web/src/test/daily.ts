// The web's tests too slow or too statistical for every pull request: the tail of a many-game sweep,
// whose first game stays per pull request, and the checks only a whole sweep can make. super.yml's
// `web-extra` job runs them once a day with JACKIOH_DAILY=1; locally,
// `JACKIOH_DAILY=1 pnpm --dir apps/web exec vitest run <file>` does the same.

import { it } from "vitest";

/** Whether this is the daily run. */
export const DAILY = process.env.JACKIOH_DAILY === "1";

/** `it`, run only in the daily run: a check that only the whole sweep makes. */
export const daily = it.runIf(DAILY);

/** A sweep of `count` games in the daily run, and its first game in every other run. */
export const sweep = (count: number): number => (DAILY ? count : 1);
