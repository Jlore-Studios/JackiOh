// #3347: a deck riffles once per action, after its last shuffle: marked on one entry, which it lengthens.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, createAnimationQueue, isShuffle, lastShuffleOf, planEntries, riffleEntries, scaleForSpeed } from "./animations.ts";
import { RIFFLE_MS } from "./config.ts";
import { baseView } from "../test/fixtures.ts";

const view = baseView();
const shuffledIn = (player: "p1" | "p2", n: number): GameEvent => ({ type: "shuffledIn", player, instanceId: `s${String(n)}`, defId: "core-070", position: n });
const traded: GameEvent = { type: "traded", player: "p1" };
const drawn: GameEvent = { type: "drawn", player: "p1", instanceId: "cY", defId: "core-055" };
const played: GameEvent[] = [
  { type: "cardPlayed", player: "p1", instanceId: "c1", defId: "core-002", costPaid: 1 },
  { type: "summoned", player: "p1", instanceId: "c1", defId: "core-002", row: "units", lane: 2 },
];
const FULL = { reduced: false, speed: 1 } as const;
const riffled = (events: GameEvent[], options: { reduced: boolean; speed: number } = FULL) =>
  riffleEntries(planEntries(events, view, options.reduced), options);

describe("#3347 the riffle", () => {
  it("#3347 a Trade riffles once, on the shuffledIn's entry", () => {
    const entries = riffled([traded, shuffledIn("p1", 1), drawn]);
    const marked = entries.filter((entry) => entry.riffle !== undefined);
    expect(marked).toHaveLength(1);
    expect(marked[0]?.events.map((event) => event.type)).toEqual(["shuffledIn"]);
    expect(marked[0]?.riffle?.sides).toEqual(["you"]);
    expect(marked[0]?.durationMs).toBe(ANIMATIONS.shuffledIn.durationMs + RIFFLE_MS);
    expect(marked[0]?.riffle).toEqual({ sides: ["you"], delayMs: ANIMATIONS.shuffledIn.durationMs, ms: RIFFLE_MS });
  });

  it("#3347 three shuffledIn in one action riffle once, on the last", () => {
    const entries = riffled([shuffledIn("p1", 1), shuffledIn("p1", 2), shuffledIn("p1", 3)]);
    expect(entries).toHaveLength(3);
    expect(entries.map((entry) => entry.riffle !== undefined)).toEqual([false, false, true]);
    expect(entries[0]?.durationMs).toBe(ANIMATIONS.shuffledIn.durationMs);
  });

  it("#3347 a mulligan shuffling both seats riffles once, on both decks, you first", () => {
    const entries = riffled([shuffledIn("p2", 1), shuffledIn("p1", 2), shuffledIn("p2", 3)]);
    const marked = entries.filter((entry) => entry.riffle !== undefined);
    expect(marked).toHaveLength(1);
    expect(marked[0]?.riffle?.sides).toEqual(["you", "opponent"]);
  });

  it("#3347 no shuffle, no riffle, and the entries come back as they were", () => {
    const planned = planEntries([drawn, ...played], view, false);
    const entries = riffleEntries(planned, FULL);
    expect(entries).toEqual(planned);
    expect(entries.every((entry) => entry.riffle === undefined)).toBe(true);
  });

  it("#3347 reduced motion marks ms 0 and keeps the entry's duration", () => {
    const planned = planEntries([shuffledIn("p1", 1)], view, true);
    const [entry] = riffleEntries(planned, { reduced: true, speed: 1 });
    expect(entry?.riffle?.ms).toBe(0);
    expect(entry?.durationMs).toBe(planned[0]?.durationMs);
  });

  it("#3347 speed 2 halves riffle.ms", () => {
    const [entry] = riffled([shuffledIn("p1", 1)], { reduced: false, speed: 2 });
    expect(entry?.riffle?.ms).toBe(RIFFLE_MS / 2);
  });

  it("#3347 a hand play is one 400 ms entry, the riffle changing nothing about it", () => {
    const entries = riffled(played);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.durationMs).toBe(ANIMATIONS.cardPlayed.durationMs);
    expect(entries[0]?.riffle).toBeUndefined();
  });

  it("#3347 isShuffle and lastShuffleOf name only a shuffledIn or a traded", () => {
    expect(isShuffle(traded)).toBe(true);
    expect(isShuffle(shuffledIn("p1", 1))).toBe(true);
    expect(isShuffle(drawn)).toBe(false);
    const last = shuffledIn("p1", 2);
    expect(lastShuffleOf([shuffledIn("p1", 1), last, drawn])).toBe(last);
    expect(lastShuffleOf([drawn])).toBeNull();
  });

  it("#3347 the queue takes the riffle's time into the entry, scaled by the speed", () => {
    const timers: number[] = [];
    const queue = createAnimationQueue({
      reducedMotion: false,
      schedule: (_run, ms) => void timers.push(ms),
      settings: () => ({ speed: 2, motion: "system", intensity: "normal" }),
    });
    queue.enqueue([shuffledIn("p1", 1)], view);
    expect(queue.inFlight()?.riffle?.ms).toBe(RIFFLE_MS / 2);
    expect(timers).toEqual([scaleForSpeed(ANIMATIONS.shuffledIn.durationMs, 2) + RIFFLE_MS / 2]);
  });

  it("#3347 with the effects off the riffle takes no time, so the entry is as long as it was", () => {
    const timers: number[] = [];
    const queue = createAnimationQueue({
      reducedMotion: false,
      schedule: (_run, ms) => void timers.push(ms),
      settings: () => ({ speed: 1, motion: "system", intensity: "off" }),
    });
    queue.enqueue([shuffledIn("p1", 1)], view);
    expect(queue.inFlight()?.riffle?.ms).toBe(0);
    expect(timers).toEqual([ANIMATIONS.shuffledIn.durationMs]);
  });
});
