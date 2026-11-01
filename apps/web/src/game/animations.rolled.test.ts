// R2652001: a card's coin flips or die rolls are one `rolled` batch, which the runner plays as one
// entry and the effects layer as one panel (fx/rolls.ts), before the entries of what the batch
// decided. Its time is ROLLED_ENTRY_MS, divided by the effects speed (R201), never squeezed by the
// burst budget, and 0 under reduced motion.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { FX_BANNER_TAIL_MS, FX_INTENSITY_SCALE } from "../fx/constants.ts";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import type { FxCue, FxRollCue } from "../fx/types.ts";
import { fullBoardView } from "../test/fixtures.ts";
import {
  ANIMATIONS,
  MIN_ENTRY_MS,
  ROLLED_ENTRY_MS,
  createAnimationQueue,
  durationFor,
  planEntries,
  scaleForSpeed,
  type AnimationEntry,
  type RunnerSignal,
} from "./animations.ts";

const GARY = "core-004";

function coins(faces: readonly (readonly number[])[], instanceId = "u1"): GameEvent {
  return {
    type: "rolled",
    player: "p1",
    instanceId,
    defId: GARY,
    kind: "coin",
    sides: 2,
    rolls: faces.map((drawn) => ({ faces: [...drawn], kept: Math.max(0, drawn.indexOf(1)) })),
  };
}

const SEVEN = coins([[1], [2], [1], [1], [2], [1], [2]]);
const BUFFED: GameEvent = { type: "buffed", instanceId: "u1", attack: 4, health: 3 };
const RIDER = coins([[2]]);
const GRANTED: GameEvent = { type: "keywordGranted", instanceId: "u1", keyword: { kind: "Rush" } };
/** A hit on the enemy hero from its own source, so no two hits join a volley or a sweep. */
const hit = (n: number): GameEvent => ({ type: "damage", sourceId: `s${String(n)}`, targetId: "hero-p2", amount: 1, combat: false });

function planOne(event: GameEvent, view: PlayerView): FxCue[] {
  const [entry] = planEntries([event], { ...view, events: [event] }, false);
  if (entry === undefined) throw new Error("no entry");
  const env = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
  env.memory.remember(entry.events);
  return planFx(entry, view, env);
}

function fakeClock() {
  const calls: { ms: number; run: () => void }[] = [];
  return {
    schedule: vi.fn((fn: () => void, ms: number) => {
      calls.push({ ms, run: fn });
    }),
    flush(): void {
      for (let guard = 0; guard < 10_000 && calls.length > 0; guard += 1) calls.shift()?.run();
    },
  };
}

/** The entries a queue starts for `events`, in order, at `speed`. */
function started(events: readonly GameEvent[], speed: number, options: { reducedMotion?: boolean; burstBudgetMs?: number } = {}): AnimationEntry[] {
  const clock = fakeClock();
  const queue = createAnimationQueue({
    schedule: clock.schedule,
    reducedMotion: options.reducedMotion ?? false,
    settings: () => ({ speed, motion: "system" }),
    ...(options.burstBudgetMs === undefined ? {} : { burstBudgetMs: options.burstBudgetMs }),
  });
  const seen: AnimationEntry[] = [];
  queue.subscribeSignals((signal: RunnerSignal) => {
    if (signal.kind === "start") seen.push(signal.entry);
  });
  queue.enqueue(events, fullBoardView());
  clock.flush();
  expect(queue.idle()).toBe(true);
  return seen;
}

describe("R2652001 a batch of coins or dice is one entry and one panel", () => {
  it("R2652001 a 7-coin Gary batch is one entry and one roll panel, not seven", () => {
    const view = fullBoardView();
    const entries = planEntries([SEVEN], { ...view, events: [SEVEN] }, false);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.type).toBe("rolled");
    expect(entries[0]?.durationMs).toBe(ROLLED_ENTRY_MS);
    expect(entries[0]?.frames.get("turn-banner")).toBe("rolled");

    const cues = planOne(SEVEN, view);
    const panels = cues.filter((cue): cue is FxRollCue => cue.kind === "roll");
    expect(panels).toHaveLength(1);
    const [panel] = panels;
    expect(panel?.die).toBe("coin");
    expect(panel?.items).toHaveLength(7);
    expect(panel?.items.map((item) => item.tumble[item.tumble.length - 1])).toEqual(["heads", "tails", "heads", "heads", "tails", "heads", "tails"]);
    expect(panel?.durationMs).toBe(ROLLED_ENTRY_MS + FX_BANNER_TAIL_MS);
    // Every coin lands inside the entry.
    for (const item of panel?.items ?? []) expect(item.landMs).toBeLessThanOrEqual(ROLLED_ENTRY_MS);
  });

  it("R2652001 the roll plays before the buff it decided", () => {
    const order = started([SEVEN, BUFFED, RIDER, GRANTED], 1).map((entry) => entry.type);
    expect(order).toEqual(["rolled", "buffed", "rolled", "keywordGranted"]);
  });

  it("R2652001 ROLLED_ENTRY_MS divides by the effects speed and is 0 under reduced motion", () => {
    expect(ANIMATIONS.rolled.durationMs).toBe(ROLLED_ENTRY_MS);
    expect(durationFor("rolled", false)).toBe(ROLLED_ENTRY_MS);
    expect(durationFor("rolled", true)).toBe(0);
    expect(scaleForSpeed(ROLLED_ENTRY_MS, 1)).toBe(ROLLED_ENTRY_MS);
    expect(scaleForSpeed(ROLLED_ENTRY_MS, 2)).toBe(ROLLED_ENTRY_MS / 2);
    expect(scaleForSpeed(ROLLED_ENTRY_MS, 0.5)).toBe(ROLLED_ENTRY_MS * 2);
    // Through the runner: the entry it starts is the scaled one, and reduced motion starts none.
    expect(started([SEVEN], 2).map((entry) => entry.durationMs)).toEqual([ROLLED_ENTRY_MS / 2]);
    expect(started([SEVEN], 1, { reducedMotion: true })).toEqual([]);
    expect(planEntries([SEVEN], fullBoardView(), true)[0]?.durationMs).toBe(0);
  });

  it("R2652001 the burst budget never squeezes a roll", () => {
    const burst = [...Array.from({ length: 12 }, (_, n) => hit(n)), SEVEN, ...Array.from({ length: 12 }, (_, n) => hit(n + 12))];
    const entries = started(burst, 1, { burstBudgetMs: ROLLED_ENTRY_MS });
    const rolls = entries.filter((entry) => entry.type === "rolled");
    expect(rolls.map((entry) => entry.durationMs)).toEqual([ROLLED_ENTRY_MS]);
    // The hits around it took the squeeze, down to the floor.
    const hits = entries.filter((entry) => entry.type === "damage");
    expect(hits.length).toBeGreaterThan(0);
    for (const entry of hits) expect(entry.durationMs).toBeLessThan(ANIMATIONS.damage.durationMs);
    for (const entry of hits) expect(entry.durationMs).toBeGreaterThanOrEqual(MIN_ENTRY_MS);
  });
});
