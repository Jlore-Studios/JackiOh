// The replay player's step cache (#511): pages fetched once, however a step is reached; a short page
// followed by the next; turn moves found across pages; a failure passed on as the source's; and the
// page after the shown step fetched ahead.

import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";
import type { ReplayStep } from "@jackioh/engine";
import { describe, expect, it } from "vitest";

import { baseView } from "../test/fixtures.ts";
import { fakeReplaySource } from "../test/replay.ts";
import { ReplayFailure } from "./source.ts";
import type { ReplaySource } from "./source.ts";
import { REPLAY_GIF_MAX_STEPS } from "./config.ts";
import { clampClip, createStepCache, loadClip, turnRange } from "./steps.ts";

/** Forty steps, six to a turn: the turns start at 0, 6, 12, …, 36, and a page boundary falls inside a turn. */
const STEPS_PER_TURN = 6;
const STEPS: ReplayStep[] = Array.from({ length: 40 }, (_, step) => {
  const turn = Math.floor(step / STEPS_PER_TURN);
  return { step, turn, view: baseView({ turn }) };
});

/** A source whose pages hold at most `size` steps, as the engine's do when a page ends early. */
function shortPages(size: number): ReplaySource & { calls: number[] } {
  const calls: number[] = [];
  return {
    total: STEPS.length,
    calls,
    page(from) {
      calls.push(from);
      return Promise.resolve(STEPS.slice(from, from + size));
    },
  };
}

describe("the replay's step cache", () => {
  it("load serves a held step without asking again", async () => {
    const source = fakeReplaySource(STEPS);
    const cache = createStepCache(source);
    expect(cache.total).toBe(STEPS.length);
    expect(cache.get(3)).toBeUndefined();
    expect((await cache.load(3)).step).toBe(3);
    expect((await cache.load(5)).step).toBe(5);
    expect(cache.get(5)?.step).toBe(5);
    expect(source.calls).toEqual([0]);
  });

  it("asks once for a page wanted twice before it arrives", async () => {
    const source = fakeReplaySource(STEPS);
    const cache = createStepCache(source);
    await Promise.all([cache.load(2), cache.load(9)]);
    expect(source.calls).toEqual([0]);
  });

  it("a page ends at the step wanted, so a jump back fills the steps before it", async () => {
    const source = fakeReplaySource(STEPS);
    const cache = createStepCache(source);
    const wanted = 30;
    await cache.load(wanted);
    expect(source.calls).toEqual([wanted - REPLAY_PAGE_STEPS + 1]);
    expect(cache.get(wanted - 1)).toBeDefined();
    expect(cache.get(wanted - REPLAY_PAGE_STEPS)).toBeUndefined();
  });

  it("a short page is followed by the next page until the step is held", async () => {
    const source = shortPages(5);
    const cache = createStepCache(source);
    expect((await cache.load(12)).step).toBe(12);
    expect(source.calls).toEqual([0, 5, 10]);
  });

  it("refuses a step outside the replay", async () => {
    const cache = createStepCache(fakeReplaySource(STEPS));
    for (const index of [-1, STEPS.length, 1.5]) {
      await expect(cache.load(index)).rejects.toMatchObject({ name: "ReplayFailure", reason: "error" });
    }
  });

  it("nextTurn and previousTurn find the turns' first steps", async () => {
    const cache = createStepCache(fakeReplaySource(STEPS));
    // From mid-turn: forward is the next turn's first step, back is this turn's own.
    expect(await cache.nextTurn(8)).toBe(12);
    expect(await cache.previousTurn(8)).toBe(6);
    // From a turn's first step: back is the turn before's first step.
    expect(await cache.nextTurn(6)).toBe(12);
    expect(await cache.previousTurn(6)).toBe(0);
    expect(await cache.previousTurn(12)).toBe(6);
    // From step 0, and the step after it, which is still the first turn.
    expect(await cache.previousTurn(0)).toBe(0);
    expect(await cache.previousTurn(1)).toBe(0);
    expect(await cache.nextTurn(0)).toBe(STEPS_PER_TURN);
    // In the last turn there is no next one: the last step.
    expect(await cache.nextTurn(37)).toBe(STEPS.length - 1);
    expect(await cache.nextTurn(STEPS.length - 1)).toBe(STEPS.length - 1);
    expect(await cache.previousTurn(STEPS.length - 1)).toBe(36);
  });

  it("a page's failure rejects load with the source's ReplayFailure", async () => {
    const failure = new ReplayFailure("expired");
    const cache = createStepCache(fakeReplaySource(STEPS, { failFrom: 0, failure }));
    await expect(cache.load(0)).rejects.toBe(failure);
    // Not held, so asking again asks the source again.
    await expect(cache.load(0)).rejects.toBe(failure);

    const odd: ReplaySource = { total: STEPS.length, page: () => Promise.reject(new Error("the network fell over")) };
    await expect(createStepCache(odd).load(0)).rejects.toMatchObject({ reason: "error", message: "the network fell over" });

    // A page that leaves out its own first step would be asked for for ever.
    const skipping: ReplaySource = { total: STEPS.length, page: (from) => Promise.resolve(STEPS.slice(from + 1, from + 3)) };
    await expect(createStepCache(skipping).load(0)).rejects.toMatchObject({ name: "ReplayFailure", reason: "error" });
  });

  it("prefetch asks for the page after the shown step", async () => {
    const source = fakeReplaySource(STEPS);
    const cache = createStepCache(source);
    await cache.load(0);
    cache.prefetch(0);
    cache.prefetch(0);
    expect(source.calls, "once, however often it is asked").toEqual([0, REPLAY_PAGE_STEPS]);
    await cache.load(REPLAY_PAGE_STEPS);
    expect(source.calls, "the prefetched page is the one load wanted").toEqual([0, REPLAY_PAGE_STEPS]);
    // Nothing unheld within reach of step 0 any more.
    cache.prefetch(0);
    expect(source.calls).toEqual([0, REPLAY_PAGE_STEPS]);
    // Past the last step there is nothing to fetch.
    cache.prefetch(STEPS.length - 1);
    expect(source.calls).toEqual([0, REPLAY_PAGE_STEPS]);
  });

  it("a prefetch that fails is left for load to report", async () => {
    const failure = new ReplayFailure("rules_changed");
    const source = fakeReplaySource(STEPS, { failFrom: REPLAY_PAGE_STEPS, failure });
    const cache = createStepCache(source);
    await cache.load(0);
    cache.prefetch(0);
    await Promise.resolve();
    await expect(cache.load(REPLAY_PAGE_STEPS)).rejects.toBe(failure);
  });
});

describe("the replay's clips", () => {
  it("clampClip orders the ends and caps the clip at REPLAY_GIF_MAX_STEPS", () => {
    expect(clampClip(3, 7)).toEqual([3, 7]);
    expect(clampClip(7, 3)).toEqual([3, 7]);
    expect(clampClip(5, 5)).toEqual([5, 5]);
    expect(clampClip(2, 2 + REPLAY_GIF_MAX_STEPS - 1)).toEqual([2, 2 + REPLAY_GIF_MAX_STEPS - 1]);
    expect(clampClip(2, 1000)).toEqual([2, 2 + REPLAY_GIF_MAX_STEPS - 1]);
    expect(clampClip(1000, 2)).toEqual([2, 2 + REPLAY_GIF_MAX_STEPS - 1]);
  });

  it("turnRange gives the shown turn's first and last steps", async () => {
    const cache = createStepCache(fakeReplaySource(STEPS));
    expect(await turnRange(cache, 8)).toEqual([6, 11]);
    expect(await turnRange(cache, 6)).toEqual([6, 11]);
    expect(await turnRange(cache, 11)).toEqual([6, 11]);
    expect(await turnRange(cache, 0)).toEqual([0, 5]);
    // The last turn has no step after it.
    expect(await turnRange(cache, STEPS.length - 1)).toEqual([36, STEPS.length - 1]);
  });

  it("turnRange caps a turn longer than the clip cap", async () => {
    const long: ReplayStep[] = Array.from({ length: REPLAY_GIF_MAX_STEPS + 10 }, (_, step) => ({ step, turn: 1, view: baseView({ turn: 1 }) }));
    const [first, last] = await turnRange(createStepCache(fakeReplaySource(long)), 5);
    expect([first, last]).toEqual([0, REPLAY_GIF_MAX_STEPS - 1]);
  });

  it("loadClip loads each step from first to last", async () => {
    const cache = createStepCache(fakeReplaySource(STEPS));
    const clip = await loadClip(cache, 4, 9);
    expect(clip.map((step) => step.step)).toEqual([4, 5, 6, 7, 8, 9]);
  });
});
