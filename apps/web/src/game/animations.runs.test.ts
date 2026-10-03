// The runner's issue-#124 entries (game/animations.ts over game/runs.ts): whole-pile impacts play
// once, sweeps play as one roll of fog, and casts play held up one card at a time.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it, vi } from "vitest";

import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import {
  CAST_ENTRY_MS,
  SWEEP_MS,
  ZONE_IMPACT_MS,
  createAnimationQueue,
  planEntries,
} from "./animations.ts";

function pileView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      libraryCount: 12,
      graveyard: [card({ instanceId: "g1" }), card({ instanceId: "g2" })],
      units: [unit("p1", { instanceId: "y1" }), unit("p1", { instanceId: "y2" }), null, null, null],
    }),
    opponent: emptySide("p2", {
      hand: { count: 4 },
      units: [unit("p2", { instanceId: "e1" }), unit("p2", { instanceId: "e2" }), null, null, null],
    }),
  });
}

function degraded(id: string): GameEvent {
  return { type: "degraded", instanceId: id, defId: "core-003", change: { kind: "cost", delta: -1 } };
}

function hit(targetId: string): GameEvent {
  return { type: "damage", sourceId: "spell", targetId, amount: 2, combat: false };
}

describe("planEntries whole-pile impacts", () => {
  it("plays a run that reaches every card of a pile as one zone entry on that pile", () => {
    const view = pileView();
    const entries = planEntries([degraded("g1"), degraded("g2")], view, false);
    expect(entries).toHaveLength(1);
    const entry = entries[0];
    expect(entry?.zone).toEqual({ side: "you", pile: "graveyard", count: 2, events: 2 });
    expect(entry?.durationMs).toBe(ZONE_IMPACT_MS);
    expect(entry?.frames.get("graveyard-you")).toBe("degraded");
  });

  it("still plays one entry per card when the effect names a number of them", () => {
    const view = pileView();
    const entries = planEntries([degraded("g1")], view, false);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.zone).toBe(undefined);
  });
});

describe("planEntries sweeps", () => {
  it("plays a run that sweeps a side as one entry carrying the sweep", () => {
    const view = pileView();
    const entries = planEntries([hit("e1"), hit("e2")], view, false);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.sweep).toEqual({ tone: "damage", sides: ["opponent"], sourceId: "spell" });
    expect(entries[0]?.durationMs).toBe(SWEEP_MS);
  });
});

describe("planEntries casts", () => {
  const box = { type: "cardPlayed", player: "p1", instanceId: "box", defId: "classicplus-047", costPaid: 8 } as const;
  const announce = (id: string): GameEvent => ({ type: "cardAnnounced", player: "p1", instanceId: id, defId: "core-010", cardType: "Spell", costPaid: 0, targets: [] });
  const cast = (id: string): GameEvent => ({ type: "cardPlayed", player: "p1", instanceId: id, defId: "core-010", costPaid: 0 });
  const done = (id: string): GameEvent => ({ type: "cardResolved", player: "p1", instanceId: id, defId: "core-010", permanent: false, costPaid: 0 });

  it("holds each cast of a burst up with its announce, naming the caster and the ordinal", () => {
    const view = pileView();
    const entries = planEntries([box, announce("c1"), cast("c1"), done("c1"), announce("c2"), cast("c2"), done("c2")], view, false);
    const casts = entries.filter((entry) => entry.cast !== undefined);
    expect(casts).toHaveLength(2);
    expect(casts[0]?.cast).toEqual({ by: "classicplus-047", ordinal: 1 });
    expect(casts[1]?.cast).toEqual({ by: "classicplus-047", ordinal: 2 });
    expect(casts[0]?.durationMs).toBe(CAST_ENTRY_MS);
    expect(casts[0]?.events.map((event) => event.type)).toEqual(["cardAnnounced", "cardPlayed"]);
  });
});

describe("planEntries reduced motion", () => {
  it("zeroes the new entries' durations too", () => {
    const view = pileView();
    const entries = planEntries([degraded("g1"), degraded("g2"), hit("e1"), hit("e2")], view, true);
    expect(entries.map((entry) => entry.durationMs)).toEqual([0, 0]);
  });
});

describe("createAnimationQueue budgets", () => {
  function fakeClock() {
    const calls: { ms: number; run: () => void }[] = [];
    return {
      calls,
      schedule: vi.fn((fn: () => void, ms: number) => {
        calls.push({ ms, run: fn });
      }),
      flush(): void {
        let guard = 0;
        while (calls.length > 0) {
          guard += 1;
          expect(guard).toBeLessThan(100_000);
          calls.shift()?.run();
        }
      },
    };
  }

  it("keeps a zone impact's and a sweep's time while squeezing the rest", () => {
    const view = pileView();
    const clock = fakeClock();
    const queue = createAnimationQueue({ schedule: clock.schedule, reducedMotion: false, burstBudgetMs: 300 });
    const filler: GameEvent[] = [];
    for (let k = 0; k < 12; k += 1) filler.push({ type: "manaChanged", player: "p1", current: 1, max: 4 });
    queue.enqueue([...filler.slice(0, 6), degraded("g1"), degraded("g2"), hit("e1"), hit("e2"), ...filler.slice(6)], view);
    clock.flush();
    const scheduled = clock.schedule.mock.calls.map((call) => Number(call[1]));
    expect(scheduled).toContain(ZONE_IMPACT_MS);
    expect(scheduled).toContain(SWEEP_MS);
    expect(queue.idle()).toBe(true);
  });
});
