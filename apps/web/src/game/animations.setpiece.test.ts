// R2785000: a set piece, the one beat per action that may hold the game. Whatever claims an entry
// (`setPieceOf`, or a Mythic's takeover, `takeoverClaim`) gets a hold that is capped, scaled by the
// effects speed, pinned against the burst budget and taken once per action, set by Big moments; a
// skip ends it at once. `spell-fixture` is the fixture set piece the queue is shown with here.

import type { CardType, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  ANIMATIONS,
  MIN_ENTRY_MS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_HOLD_MS,
  SET_PIECE_SHORT_HOLD_MS,
  SET_PIECES_PER_ACTION,
  TAKEOVER_HOLD_MS,
  TAKEOVER_SET_PIECE,
  createAnimationQueue,
  entranceEntries,
  leadMs,
  planEntries,
  scaleForSpeed,
  setPieceEntries,
  setPieceHoldMs,
  takeoverClaim,
  type AnimationEntry,
  type AnimationQueueOptions,
  type EntranceResolver,
  type RunnerSignal,
  type SetPieceClaimer,
} from "./animations.ts";
import type { FxBigMoments, FxIntensity } from "../fx/settings.ts";
import { baseView } from "../test/fixtures.ts";

const CARDS: Record<string, { type: CardType; rarity: string }> = {
  "spell-mythic": { type: "Spell", rarity: "Mythic" },
  "spell-fixture": { type: "Spell", rarity: "Epic" },
  "spell-plain": { type: "Spell", rarity: "Common" },
};
const resolve: EntranceResolver = (defId) => CARDS[defId] ?? null;
const p1 = baseView();

const play = (defId: string, instanceId = `i-${defId}`, player: PlayerId = "p1"): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 3 });

/** The fixture set piece: any entry that plays `spell-fixture` asks for the whole beat. */
const fixture: SetPieceClaimer = (entry) =>
  entry.events.some((e) => e.type === "cardPlayed" && e.defId === "spell-fixture") ? { key: "fixture", holdMs: SET_PIECE_HOLD_MS } : null;

const FULL = { reduced: false, bigMoments: "full", speed: 1 } as const;
const held = (entries: readonly AnimationEntry[]): number[] => entries.map((entry) => entry.setPiece?.holdMs ?? 0);

type Settings = { speed?: number; intensity?: FxIntensity; bigMoments?: FxBigMoments };

/** A queue whose timers run only when the test says, and whose signals it records. */
function queueOf(settings: Settings = {}, over: Partial<AnimationQueueOptions> = {}) {
  const timers: { ms: number; run: () => void }[] = [];
  const signals: RunnerSignal["kind"][] = [];
  let settled = 0;
  const queue = createAnimationQueue({
    reducedMotion: false,
    schedule: (run, ms) => {
      timers.push({ ms, run });
    },
    settings: () => ({ speed: settings.speed ?? 1, motion: "system", intensity: settings.intensity ?? "normal", bigMoments: settings.bigMoments ?? "full" }),
    entranceOf: resolve,
    setPieceOf: fixture,
    onSettled: () => {
      settled += 1;
    },
    ...over,
  });
  queue.subscribeSignals((signal) => signals.push(signal.kind));
  return { queue, timers, signals, settled: () => settled };
}

describe("R2785000 a set piece's hold", () => {
  it("R2785000 the hold is capped at SET_PIECE_HOLD_CAP_MS", () => {
    expect(setPieceHoldMs(10_000, "full", 1)).toBe(SET_PIECE_HOLD_CAP_MS);
    // A quarter speed would stretch the beat four times over: the cap holds it, in real time.
    expect(scaleForSpeed(SET_PIECE_HOLD_MS, 0.25)).toBeGreaterThan(SET_PIECE_HOLD_CAP_MS);
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "full", 0.25)).toBe(SET_PIECE_HOLD_CAP_MS);
    expect(setPieceHoldMs(TAKEOVER_HOLD_MS, "full", 0.25)).toBe(SET_PIECE_HOLD_CAP_MS);
    // And on the queue.
    const { queue } = queueOf({ speed: 0.25 });
    queue.enqueue([play("spell-fixture")], p1);
    expect(queue.inFlight()?.setPiece).toEqual({ key: "fixture", holdMs: SET_PIECE_HOLD_CAP_MS });
  });

  it("R2785000 the hold is scaled by the effects speed, never below MIN_ENTRY_MS", () => {
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "full", 1)).toBe(SET_PIECE_HOLD_MS);
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "full", 2)).toBe(scaleForSpeed(SET_PIECE_HOLD_MS, 2));
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "full", 2)).toBe(SET_PIECE_HOLD_MS / 2);
    expect(setPieceHoldMs(150, "full", 3)).toBe(MIN_ENTRY_MS);
    const { queue, timers } = queueOf({ speed: 2 });
    queue.enqueue([play("spell-fixture")], p1);
    expect(queue.inFlight()?.setPiece?.holdMs).toBe(SET_PIECE_HOLD_MS / 2);
    expect(timers.map((t) => t.ms)).toEqual([scaleForSpeed(ANIMATIONS.cardPlayed.durationMs, 2) + SET_PIECE_HOLD_MS / 2]);
  });

  it("R2785000 the hold is pinned against the burst budget", () => {
    const waits: number[] = [];
    const queue = createAnimationQueue({
      reducedMotion: false,
      burstBudgetMs: 900,
      schedule: (run, ms) => {
        waits.push(ms);
        run();
      },
      settings: () => ({ speed: 1, motion: "system", intensity: "normal", bigMoments: "full" }),
      setPieceOf: fixture,
    });
    queue.enqueue([play("spell-fixture"), play("spell-plain", "a"), play("spell-plain", "b")], p1);
    // The held entry keeps its whole time, which leaves the rest of the budget nothing: they play at the floor.
    expect(waits).toEqual([ANIMATIONS.cardPlayed.durationMs + SET_PIECE_HOLD_MS, MIN_ENTRY_MS, MIN_ENTRY_MS]);
  });

  it("R2785000 at most SET_PIECES_PER_ACTION per action", () => {
    expect(SET_PIECES_PER_ACTION).toBe(1);
    const two = setPieceEntries(planEntries([play("spell-fixture", "a"), play("spell-fixture", "b")], p1, false), fixture, FULL);
    expect(held(two)).toEqual([SET_PIECE_HOLD_MS, 0]);
    // The second plays as it would with no claim at all.
    expect(two[1]?.setPiece).toBeUndefined();
    expect(two[1]?.durationMs).toBe(ANIMATIONS.cardPlayed.durationMs);
    // The next action holds again.
    expect(held(setPieceEntries(planEntries([play("spell-fixture", "c")], p1, false), fixture, FULL))).toEqual([SET_PIECE_HOLD_MS]);
    // A set piece before a Mythic leaves the Mythic its entrance, and no hold.
    const claimOf: SetPieceClaimer = (entry) => fixture(entry) ?? takeoverClaim(entry);
    const entered = entranceEntries(planEntries([play("spell-fixture"), play("spell-mythic")], p1, false), resolve, { seen: new Set() });
    const both = setPieceEntries(entered, claimOf, FULL);
    expect(both[0]?.setPiece?.key).toBe("fixture");
    expect(both[1]?.entrance?.tier).toBe("mythic");
    expect(both[1]?.setPiece).toBeUndefined();
    expect(both[1]?.durationMs).toBe(ANIMATIONS.cardPlayed.durationMs);
  });

  it("R2785000 Big moments full, short and off", () => {
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "full", 1)).toBe(SET_PIECE_HOLD_MS);
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "short", 1)).toBe(SET_PIECE_SHORT_HOLD_MS);
    expect(setPieceHoldMs(TAKEOVER_HOLD_MS, "short", 1)).toBe(Math.min(TAKEOVER_HOLD_MS, SET_PIECE_SHORT_HOLD_MS));
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "short", 2)).toBe(scaleForSpeed(SET_PIECE_SHORT_HOLD_MS, 2));
    expect(setPieceHoldMs(SET_PIECE_HOLD_MS, "off", 1)).toBe(0);
    expect(setPieceHoldMs(TAKEOVER_HOLD_MS, "off", 0.25)).toBe(0);
    for (const [bigMoments, hold] of [["full", SET_PIECE_HOLD_MS], ["short", SET_PIECE_SHORT_HOLD_MS]] as const) {
      const { queue, timers } = queueOf({ bigMoments });
      queue.enqueue([play("spell-fixture")], p1);
      expect(queue.inFlight()?.setPiece, bigMoments).toEqual({ key: "fixture", holdMs: hold });
      expect(timers.map((t) => t.ms), bigMoments).toEqual([ANIMATIONS.cardPlayed.durationMs + hold]);
    }
    // Off plays the entry with no hold, as any other entry plays; the Mythic's takeover too.
    const { queue, timers } = queueOf({ bigMoments: "off" });
    queue.enqueue([play("spell-fixture")], p1);
    expect(queue.inFlight()?.setPiece).toBeUndefined();
    expect(timers.map((t) => t.ms)).toEqual([ANIMATIONS.cardPlayed.durationMs]);
    timers.shift()?.run();
    queue.enqueue([play("spell-mythic")], p1);
    expect(queue.inFlight()?.entrance?.tier).toBe("mythic");
    expect(queue.inFlight()?.setPiece).toBeUndefined();
  });

  it("R2785000 reduced motion, the effects off and a silent queue take no hold", () => {
    expect(held(setPieceEntries(planEntries([play("spell-fixture")], p1, true), fixture, { ...FULL, reduced: true }))).toEqual([0]);
    for (const [settings, over] of [
      [{}, { reducedMotion: true }],
      [{ intensity: "off" }, {}],
      [{}, { setPieces: false }],
    ] as const) {
      const { queue, timers } = queueOf(settings, over);
      queue.enqueue([play("spell-fixture")], p1);
      expect(queue.inFlight()?.setPiece, JSON.stringify([settings, over])).toBeUndefined();
      expect(timers.every((t) => t.ms <= ANIMATIONS.cardPlayed.durationMs), JSON.stringify([settings, over])).toBe(true);
    }
    // Reduced motion drains at once: no hold, and no still frame of it either.
    const { queue, settled } = queueOf({}, { reducedMotion: true });
    queue.enqueue([play("spell-fixture"), play("spell-mythic")], p1);
    expect(queue.idle()).toBe(true);
    expect(settled()).toBe(1);
  });

  it("R2785000 skip ends the held entry at once and the queue goes on", () => {
    const { queue, timers, signals, settled } = queueOf();
    queue.enqueue([play("spell-fixture"), play("spell-plain")], p1);
    const first = queue.inFlight();
    expect(first?.setPiece?.key).toBe("fixture");
    expect(queue.skip()).toBe(true);
    expect(signals).toEqual(["start", "skip", "start"]);
    // The next entry is in flight at once, and the skipped entry's timer, when it fires, changes nothing.
    const second = queue.inFlight();
    expect(second?.events[0]).toMatchObject({ defId: "spell-plain" });
    expect(timers).toHaveLength(2);
    timers.shift()?.run();
    expect(queue.inFlight()).toBe(second);
    expect(signals).toEqual(["start", "skip", "start"]);
    // A plain entry is no set piece: skip does nothing.
    expect(queue.skip()).toBe(false);
    expect(queue.inFlight()).toBe(second);
    timers.shift()?.run();
    expect(queue.idle()).toBe(true);
    expect(settled()).toBe(1);
    expect(queue.skip()).toBe(false);

    // Skipping the action's last entry settles it, once.
    const last = queueOf();
    last.queue.enqueue([play("spell-fixture")], p1);
    expect(last.queue.skip()).toBe(true);
    expect(last.queue.idle()).toBe(true);
    expect(last.settled()).toBe(1);
    expect(last.signals).toEqual(["start", "skip", "idle"]);
    for (const timer of last.timers.splice(0)) timer.run();
    expect(last.settled()).toBe(1);
    expect(last.signals).toEqual(["start", "skip", "idle"]);
  });

  it("R2785000 the Mythic takeover is a set piece", () => {
    const { queue, timers } = queueOf({}, { setPieceOf: undefined });
    queue.enqueue([play("spell-mythic")], p1);
    expect(queue.inFlight()?.setPiece).toEqual({ key: TAKEOVER_SET_PIECE, holdMs: TAKEOVER_HOLD_MS });
    expect(leadMs(queue.inFlight() ?? {})).toBe(TAKEOVER_HOLD_MS);
    expect(timers.map((t) => t.ms)).toEqual([ANIMATIONS.cardPlayed.durationMs + TAKEOVER_HOLD_MS]);
    // Its entrance carries the card, and no hold of its own.
    expect(queue.inFlight()?.entrance).not.toHaveProperty("holdMs");
    expect(queue.skip()).toBe(true);
    expect(queue.idle()).toBe(true);
  });
});
