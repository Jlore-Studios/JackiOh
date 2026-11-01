// The web's tests too slow or statistical for every pull request (a sweep's tail past its first game, and
// the checks only a whole sweep makes): super.yml's `web-extra` job runs them daily with JACKIOH_DAILY=1,
// as `JACKIOH_DAILY=1 pnpm --dir apps/web exec vitest run <file>` does locally.

import { it } from "vitest";

export const DAILY = process.env.JACKIOH_DAILY === "1";

/** `it`, run only in the daily run. */
export const daily = it.runIf(DAILY);

/** A sweep of `count` games in the daily run, its first game alone otherwise. */
export const sweep = (count: number): number => (DAILY ? count : 1);
