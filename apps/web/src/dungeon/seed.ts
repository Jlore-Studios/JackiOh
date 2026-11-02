// The run's seed as the start screen chooses it (DR2, B15): the day's, a random one or one typed.
// The seed's letters are `crates/run`'s (`runSeed`, `runDailySeed`); the client only asks for them.

import { runSeed } from "../wasm/index.ts";
import { DUNGEON_DATE_FORMAT } from "./config.ts";

/**
 * DR2: the date the daily run hashes, `YYYY-MM-DD`. It is the UTC day, so everyone in the world is
 * dealt the same daily run whatever their clock's zone.
 */
export function todayDate(now: Date = new Date()): string {
  return now.toISOString().slice(0, DUNGEON_DATE_FORMAT.length);
}

/** DR2: how many capital letters a seed has, as the run's own seeds show it. */
export function seedLength(): number {
  return runSeed("length").length;
}

/** B15: a typed seed is capital letters, as many as a seed has. */
export function isTypedSeed(text: string): boolean {
  return /^[A-Z]+$/u.test(text) && text.length === seedLength();
}

/** DR2: a seed drawn from a random number, for a run that is not the daily one. */
export function randomSeed(): string {
  return runSeed(String(Math.random()));
}
