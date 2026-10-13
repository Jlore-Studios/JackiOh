// The steps the replay player has been given so far (#511): pages of a `ReplaySource`, held by step
// number, asked for only when a step is wanted and fetched ahead of the one shown. The player reads
// every step through this and no source directly, so a page is fetched once however it is reached.

import type { ReplayStep } from "@jackioh/engine";
import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";

import { REPLAY_GIF_MAX_STEPS, REPLAY_PREFETCH_STEPS } from "./config.ts";
import { ReplayFailure, replayFailureOf } from "./source.ts";
import type { ReplaySource } from "./source.ts";

export type StepCache = {
  readonly total: number;
  /** The step, when a page has brought it. */
  get(index: number): ReplayStep | undefined;
  /** The step, fetching the page that holds it; rejects with a `ReplayFailure`. */
  load(index: number): Promise<ReplayStep>;
  /** Starts the page after the shown step when its steps are not held yet. A failure is left for `load` to report. */
  prefetch(index: number): void;
  /** The first step of the next turn after `index`'s, or the last step when none follows. */
  nextTurn(index: number): Promise<number>;
  /** The first step of the turn `index` is in; of the turn before when it is a turn's first step; 0 at the start. */
  previousTurn(index: number): Promise<number>;
};

export function createStepCache(source: ReplaySource): StepCache {
  const { total } = source;
  const held = new Map<number, ReplayStep>();
  const flying = new Map<number, Promise<void>>();

  /** Asks the source for the page at `from`, once however many ask while it is on its way. */
  function fetchFrom(from: number): Promise<void> {
    const asked = flying.get(from);
    if (asked !== undefined) return asked;
    const run = source
      .page(from, REPLAY_PAGE_STEPS)
      .then((page) => {
        for (const step of page) held.set(step.step, step);
        // A page that skips its own first step would be asked for again for ever.
        if (!held.has(from)) throw new ReplayFailure("error", `the replay's page at step ${String(from)} did not hold it`);
      })
      .catch((cause: unknown) => {
        throw replayFailureOf(cause);
      })
      .finally(() => {
        flying.delete(from);
      });
    flying.set(from, run);
    return run;
  }

  async function load(index: number): Promise<ReplayStep> {
    if (!Number.isInteger(index) || index < 0 || index >= total) {
      throw new ReplayFailure("error", `no step ${String(index)} in a replay of ${String(total)} steps`);
    }
    for (;;) {
      const step = held.get(index);
      if (step !== undefined) return step;
      // The page ends at the step wanted, so a jump back fills the steps before it too; a short page
      // leaves a gap, which the next pass starts after.
      let from = Math.max(0, index - REPLAY_PAGE_STEPS + 1);
      while (from < index && held.has(from)) from += 1;
      await fetchFrom(from);
    }
  }

  async function turnOf(index: number): Promise<number> {
    return (await load(index)).turn;
  }

  return {
    total,
    get: (index) => held.get(index),
    load,
    prefetch(index) {
      const last = Math.min(total - 1, index + REPLAY_PREFETCH_STEPS);
      for (let next = index + 1; next <= last; next += 1) {
        if (held.has(next)) continue;
        void fetchFrom(next).catch(() => undefined);
        return;
      }
    },
    async nextTurn(index) {
      const turn = await turnOf(index);
      for (let next = index + 1; next < total; next += 1) {
        if ((await turnOf(next)) !== turn) return next;
      }
      return Math.max(0, total - 1);
    },
    async previousTurn(index) {
      if (index <= 0) return 0;
      const turn = await turnOf(index - 1);
      let first = index - 1;
      while (first > 0 && (await turnOf(first - 1)) === turn) first -= 1;
      return first;
    },
  };
}

/** A clip's first and last step, in order, the last no further than `REPLAY_GIF_MAX_STEPS` steps on (#513). */
export function clampClip(a: number, b: number): [number, number] {
  const lo = Math.min(a, b);
  const hi = Math.max(a, b);
  return [lo, Math.min(hi, lo + REPLAY_GIF_MAX_STEPS - 1)];
}

/** The first and last step of the turn `index` is in, the clip clamped to its cap (#513). */
export async function turnRange(cache: StepCache, index: number): Promise<[number, number]> {
  const turn = (await cache.load(index)).turn;
  let first = index;
  while (first > 0 && (await cache.load(first - 1)).turn === turn) first -= 1;
  let last = index;
  while (last < cache.total - 1 && (await cache.load(last + 1)).turn === turn) last += 1;
  return clampClip(first, last);
}

/** Steps `from` to `to`, both included, loading the pages that hold them (#513). */
export async function loadClip(cache: StepCache, from: number, to: number): Promise<ReplayStep[]> {
  const steps: ReplayStep[] = [];
  for (let index = from; index <= to; index += 1) steps.push(await cache.load(index));
  return steps;
}
