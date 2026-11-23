// The runner's budgets and its life (game/animations.ts `createAnimationQueue`), at their edges: issue
// #124's cast segments, each with `CAST_BUDGET_MS` of its own; the burst budget's exact fit; R201's speed
// dividing a cast's budget as it does the burst's; BUILD M5-T4's `onSettled`, once a batch; and
// `drain`, `skip` and `reset`, after which no timer from before can end a later entry (#4038).

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { baseView, emptySide, unit } from "../test/fixtures.ts";
import { CAST_ENTRY_MS, createAnimationQueue, type AnimationEntry, type AnimationQueueOptions, type RunnerSignal } from "./animations.ts";

const view: PlayerView = baseView({
  you: emptySide("p1", { units: [unit("p1", { instanceId: "y1" }), null, null, null, null] }),
  opponent: emptySide("p2", { units: [unit("p2", { instanceId: "e1" }), null, null, null, null] }),
});

/** C+ #47 Jogg's Box, whose play casts the cards after it (`runs.ts`). */
const box: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "box", defId: "classicplus-047", costPaid: 8 };
const announce = (id: string): GameEvent => ({ type: "cardAnnounced", player: "p1", instanceId: id, defId: "core-010", cardType: "Spell", costPaid: 0, targets: [] });
const played = (id: string): GameEvent => ({ type: "cardPlayed", player: "p1", instanceId: id, defId: "core-010", costPaid: 0 });
/** 150 ms each. */
const mana = (n: number): GameEvent => ({ type: "manaChanged", player: "p1", current: n, max: 10 });
/** 600 ms each; a turn's start ends every play still open. */
const turn = (n: number): GameEvent => ({ type: "turnStarted", player: "p1", turn: n });
/** 500 ms each, the board's. */
const rotated = (): GameEvent => ({ type: "rotated", direction: "left" });
const gameOver: GameEvent = { type: "gameOver", winner: "p1", reason: "hero-death" };
const prompt: GameEvent = { type: "promptOpened", player: "p1" };
const times = <T>(n: number, make: (k: number) => T): T[] => Array.from({ length: n }, (_, k) => make(k));

/** A queue whose timers run only when the test says. */
function rig(over: Partial<AnimationQueueOptions> = {}) {
  const timers: { ms: number; run: () => void }[] = [];
  const onSettled = vi.fn();
  const queue = createAnimationQueue({
    reducedMotion: false,
    schedule: (run, ms) => {
      timers.push({ ms, run });
    },
    settings: () => ({ speed: 1, motion: "system", intensity: "normal", bigMoments: "full" }),
    onSettled,
    ...over,
  });
  /** Every timer scheduled from here on, run in turn, and what each was scheduled for. */
  const flush = (): number[] => {
    const ms: number[] = [];
    for (let guard = 0; timers.length > 0; guard += 1) {
      expect(guard).toBeLessThan(10_000);
      const next = timers.shift();
      if (next === undefined) break;
      ms.push(next.ms);
      next.run();
    }
    return ms;
  };
  return { queue, timers, onSettled, flush };
}

describe("issue #124 each cast is a segment of the budget with CAST_BUDGET_MS of its own", () => {
  // A cast holds CAST_ENTRY_MS out of its segment's budget; ten 150 ms entries after it share the rest.
  const castThenTen = [box, announce("c1"), played("c1"), ...times(10, mana)];

  it("#124 by default a cast's segment fits CAST_BUDGET_MS, the cast holding its whole entry", () => {
    const { queue, flush } = rig();
    queue.enqueue(castThenTen, view);
    // 2200 − 800 leaves 1400 for 1500: each 150 plays as 140.
    expect(flush()).toEqual([400, CAST_ENTRY_MS, ...times(10, () => 140)]);
  });

  it("#124 a given castBudgetMs is the segment's budget", () => {
    const { queue, flush } = rig({ castBudgetMs: 1100 });
    queue.enqueue(castThenTen, view);
    // 1100 − 800 leaves 300 for 1500, below the floor: each plays MIN_ENTRY_MS.
    expect(flush()).toEqual([400, CAST_ENTRY_MS, ...times(10, () => 120)]);
  });

  it("#124 a cast that opens a batch, the play casting it open since a prompt, starts its own segment", () => {
    const { queue, flush } = rig();
    queue.enqueue([box, prompt], view);
    expect(flush()).toEqual([400, 150]);

    queue.enqueue([announce("c1"), played("c1"), ...times(10, mana)], view);
    expect(flush()).toEqual([CAST_ENTRY_MS, ...times(10, () => 140)]);
  });

  it("#124 what comes in while a cast is in flight shares what is left of its segment", () => {
    const { queue, timers, flush } = rig();
    queue.enqueue([box, announce("c1"), played("c1")], view);
    timers.shift()?.run();
    expect(queue.inFlight()?.cast).toEqual({ by: "classicplus-047", ordinal: 1 });

    queue.enqueue(times(10, mana), view);
    // The cast in flight took 800 of its 2200: 1400 for 1500.
    expect(flush()).toEqual([CAST_ENTRY_MS, ...times(10, () => 140)]);
  });

  it("#124 what comes before the first cast keeps the burst budget, and each cast after starts a segment of its own", () => {
    const { queue, flush } = rig();
    queue.enqueue([box, ...times(5, rotated), announce("c1"), played("c1"), ...times(10, mana), announce("c2"), played("c2"), ...times(10, mana)], view);
    // The first segment's 2900 fit BURST_BUDGET_MS's 2400 (a factor of 24/29), the cast after it untouched.
    expect(flush()).toEqual([331, ...times(5, () => 414), CAST_ENTRY_MS, ...times(10, () => 140), CAST_ENTRY_MS, ...times(10, () => 140)]);
  });

  it("R201 at twice the speed a cast's budget halves with everything else", () => {
    const { queue, flush } = rig({ settings: () => ({ speed: 2, motion: "system", intensity: "normal", bigMoments: "full" }) });
    queue.enqueue([box, announce("c1"), played("c1"), ...times(5, turn)], view);
    // 1100 − 400 leaves 700 for five 300s: each plays 140.
    expect(flush()).toEqual([200, 400, ...times(5, () => 140)]);
  });

  it("R201 at twice the speed the burst budget halves too", () => {
    const { queue, flush } = rig({ settings: () => ({ speed: 2, motion: "system", intensity: "normal", bigMoments: "full" }) });
    queue.enqueue(times(5, rotated), view);
    // Five 250s in 1200: each plays 240.
    expect(flush()).toEqual(times(5, () => 240));
  });
});

describe("BUILD M5-T4 the burst budget's edges", () => {
  it("M5-T4 a burst that fits its budget exactly keeps the table's durations, even below the floor", () => {
    const { queue, flush } = rig({ burstBudgetMs: 450, minEntryMs: 200 });
    queue.enqueue([mana(1), { type: "damage", sourceId: "y1", targetId: "e1", amount: 1, combat: false }], view);
    expect(flush()).toEqual([150, 300]);
  });

  it("M5-T4 a squeezed burst never gives an entry of no duration a timer", () => {
    const { queue, flush } = rig();
    queue.enqueue([...times(5, turn), gameOver], view);
    expect(flush()).toEqual(times(5, () => 480));
    expect(queue.idle()).toBe(true);
  });
});

describe("BUILD M5-T4 onSettled fires once for each batch", () => {
  it("M5-T4 a queue never given a batch settles nothing when drained", () => {
    const { queue, onSettled } = rig();
    queue.drain();
    expect(onSettled).not.toHaveBeenCalled();
  });

  it("M5-T4 a batch settles once: a drain after it has settled settles nothing more", () => {
    const signals: RunnerSignal["kind"][] = [];
    const { queue, onSettled, flush } = rig();
    queue.subscribeSignals((signal) => signals.push(signal.kind));
    queue.enqueue([mana(1), mana(2)], view);
    flush();
    expect(onSettled).toHaveBeenCalledTimes(1);

    queue.drain();
    expect(onSettled).toHaveBeenCalledTimes(1);
    expect(signals.filter((kind) => kind === "idle")).toHaveLength(1);
  });

  it("M5-T4 a reset batch is owed nothing: a drain after the reset settles nothing", () => {
    const { queue, onSettled } = rig();
    queue.enqueue([mana(1), mana(2)], view);
    queue.reset();
    queue.drain();
    expect(onSettled).not.toHaveBeenCalled();
  });

  it("M5-T4 the queue is not idle while an entry plays, though nothing waits behind it", () => {
    const { queue, flush } = rig();
    queue.enqueue([mana(1)], view);
    expect(queue.pending()).toBe(0);
    expect(queue.idle()).toBe(false);
    flush();
    expect(queue.idle()).toBe(true);
  });

  it("M5-T4 a drain inside a still entry's signal cuts every entry still waiting, and settles the batch once", () => {
    const cut: (readonly AnimationEntry[])[] = [];
    const kinds: RunnerSignal["kind"][] = [];
    let idleAtStill: boolean | null = null;
    const { queue, onSettled } = rig({ reducedMotion: true });
    queue.subscribeSignals((signal) => {
      kinds.push(signal.kind);
      if (signal.kind === "still" && cut.length === 0) {
        idleAtStill = queue.idle();
        queue.drain();
      }
      if (signal.kind === "drain") cut.push(signal.entries);
    });
    queue.enqueue([mana(1), mana(2), mana(3)], view);
    expect(idleAtStill).toBe(false);
    expect(cut.map((entries) => entries.map((entry) => entry.events))).toEqual([[[mana(2)], [mana(3)]]]);
    // The drain settled the batch: the pump, finding the queue empty after it, owes it no second idle.
    expect(kinds).toEqual(["still", "drain"]);
    expect(onSettled).toHaveBeenCalledTimes(1);
  });

  it("M5-T4 a drain cuts the entry in flight first, tells its subscribers, and a signal listener taken off hears nothing", () => {
    const cut: (readonly AnimationEntry[])[] = [];
    const { queue } = rig();
    const off = queue.subscribeSignals((signal) => {
      if (signal.kind === "drain") cut.push(signal.entries);
    });
    queue.enqueue([mana(1), mana(2)], view);
    const seen = vi.fn();
    queue.subscribe(seen);
    queue.drain();
    expect(cut.map((entries) => entries.map((entry) => entry.events))).toEqual([[[mana(1)], [mana(2)]]]);
    expect(seen).toHaveBeenCalledTimes(1);

    off();
    queue.enqueue([mana(3)], view);
    queue.drain();
    expect(cut).toHaveLength(1);
  });
});

describe("drain, skip and reset: no timer from before ends a later entry", () => {
  it("M5-T4 a timer from before a drain and a reset leaves the entry playing after them alone", () => {
    for (const order of [["drain", "reset"], ["reset", "drain"]] as const) {
      const { queue, timers } = rig();
      queue.enqueue([turn(1)], view);
      const stale = timers.shift();
      for (const step of order) queue[step]();
      queue.enqueue([turn(2)], view);

      stale?.run();
      expect(queue.inFlight()?.events, order.join(" then ")).toEqual([turn(2)]);
    }
  });

  it("R2785000 a skipped set piece's timer, after a drain, leaves the entry playing after them alone", () => {
    const { queue, timers } = rig({ setPieceOf: (entry) => (entry.type === "turnStarted" ? { key: "test", holdMs: 500 } : null) });
    queue.enqueue([turn(1), mana(1)], view);
    expect(queue.inFlight()?.setPiece?.key).toBe("test");
    const stale = timers.shift();
    expect(queue.skip()).toBe(true);
    queue.drain();
    queue.enqueue([mana(2)], view);

    stale?.run();
    expect(queue.inFlight()?.events).toEqual([mana(2)]);
  });
});

describe("reset forgets the batches before it", () => {
  it("#124 a play left open before a reset casts nothing after it", () => {
    const { queue, flush } = rig();
    queue.enqueue([box, prompt], view);
    flush();
    queue.reset();
    queue.enqueue([announce("c1"), played("c1")], view);
    expect(queue.inFlight()?.type).toBe("cardAnnounced");
    expect(queue.inFlight()?.cast).toBeUndefined();
  });

  it("R2722001 a card that made its entrance before a reset makes it again after", () => {
    const summoned: GameEvent = { type: "summoned", player: "p1", instanceId: "l1", defId: "core-001", row: "units", lane: 2 };
    const { queue, flush } = rig({ entranceOf: () => ({ type: "Unit", rarity: "Legendary" }), setPieces: false });
    queue.enqueue([summoned], view);
    expect(queue.inFlight()?.entrance?.instanceId).toBe("l1");
    flush();
    queue.enqueue([summoned], view);
    expect(queue.inFlight()?.entrance).toBeUndefined();
    flush();

    queue.reset();
    queue.enqueue([summoned], view);
    expect(queue.inFlight()?.entrance?.instanceId).toBe("l1");
  });

  it("M5-T4 a reset tells its subscribers", () => {
    const { queue } = rig();
    queue.enqueue([mana(1)], view);
    const seen = vi.fn();
    queue.subscribe(seen);
    queue.reset();
    expect(seen).toHaveBeenCalledTimes(1);
    expect(queue.animating().size).toBe(0);
  });
});
