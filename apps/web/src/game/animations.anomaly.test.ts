// R3337003: an Anomaly's lay in the runner. A `boardReplaced` that names an Anomaly (R3337000) and every
// event after it up to the next hook play as one rift entry, which the Anomaly claims as its set piece
// (`anomalyClaim`), held as R2785000 holds every set piece: capped, scaled, set by Big moments, and none
// under Reduce motion. A replacement naming no Anomaly plays as it did.

import type { GameEvent, PlayerId, UnitView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { anomalyClaim } from "../fx/anomaly.ts";
import type { FxBigMoments } from "../fx/settings.ts";
import {
  ANOMALY_LAND_MS,
  ANOMALY_RIFT_HOLD_MS,
  ANOMALY_SET_PIECE,
  BURST_BUDGET_MS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_SHORT_HOLD_MS,
  createAnimationQueue,
  planEntries,
  scaleForSpeed,
  slamEntries,
  type AnimationEntry,
} from "./animations.ts";
import { testid } from "./contract.ts";
import { createPlayTracker } from "./runs.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";

const ANOMALY_FOUND = "stinkyscape-t-anomaly-found";
const ASTEROID = "stinkyscape-171";

const exiled = (u: UnitView): GameEvent => ({ type: "exiled", instanceId: u.instanceId, defId: u.defId, owner: u.owner });
const summoned = (instanceId: string, lane: number, row: "units" | "backrow" = "units", player: PlayerId = "p1"): GameEvent => ({
  type: "summoned",
  player,
  instanceId,
  defId: row === "units" ? ASTEROID : "hidden",
  row,
  lane,
});
const played: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "found", defId: ANOMALY_FOUND, costPaid: 4 };
const resolved: GameEvent = { type: "cardResolved", player: "p1", instanceId: "found", defId: ANOMALY_FOUND, permanent: false, costPaid: 4 };
const cry: GameEvent = { type: "hookResolving", sourceId: "rock-1", defId: ASTEROID, controller: "p1", hook: "cry" };

/** p1's side holds two Units in lanes 1 and 2; p2's one in lane 1. */
function board() {
  const a = unit("p1", { instanceId: "a" });
  const b = unit("p1", { instanceId: "b" });
  const view = baseView({
    you: emptySide("p1", { units: [a, b, null, null, null] }),
    opponent: emptySide("p2", { units: [unit("p2", { instanceId: "foe" }), null, null, null, null] }),
  });
  return { view, a, b };
}

/** The cast: its play, the replacement naming `anomaly`, the old side's exiles, the board's landings, a Cry and the play's end. */
function cast(anomaly: string | undefined, a: UnitView, b: UnitView): GameEvent[] {
  const replaced: GameEvent = anomaly === undefined ? { type: "boardReplaced", player: "p1" } : { type: "boardReplaced", player: "p1", anomaly };
  return [played, replaced, exiled(a), exiled(b), summoned("rock-1", 2), summoned("rock-2", 4), summoned("trap", 3, "backrow"), cry, resolved];
}

/** A queue whose timers run when the test says, claiming set pieces as Game does. */
function queueOf(settings: { speed?: number; bigMoments?: FxBigMoments } = {}, reducedMotion = false) {
  const timers: { ms: number; run: () => void }[] = [];
  const started: AnimationEntry[] = [];
  const queue = createAnimationQueue({
    reducedMotion,
    schedule: (run, ms) => {
      timers.push({ ms, run });
    },
    settings: () => ({ speed: settings.speed ?? 1, motion: "system", intensity: "normal", bigMoments: settings.bigMoments ?? "full" }),
    setPieceOf: anomalyClaim,
    burstBudgetMs: BURST_BUDGET_MS,
  });
  queue.subscribeSignals((signal) => {
    if (signal.kind === "start" || signal.kind === "still") started.push(signal.entry);
  });
  const playAll = (): AnimationEntry[] => {
    for (let guard = 0; guard < 200 && timers.length > 0; guard += 1) timers.shift()?.run();
    return started;
  };
  return { queue, playAll };
}

describe("R3337003 an Anomaly's rift entry", () => {
  it("R3337003 a coded boardReplaced and its exiles and landings, up to the first Cry, are one entry", () => {
    const { view, a, b } = board();
    const events = cast("AL", a, b);
    const entries = planEntries(events, view, false, createPlayTracker());
    const rifts = entries.filter((entry) => entry.rift !== undefined);
    expect(rifts).toHaveLength(1);
    const [rift] = rifts;
    expect(rift?.events).toEqual(events.slice(1, 7));
    expect(rift?.type).toBe("boardReplaced");
    expect(rift?.durationMs).toBe(ANOMALY_LAND_MS);
    expect(rift?.rift).toEqual({
      player: "p1",
      code: "AL",
      side: "you",
      landings: [
        { zone: testid.zone("you", "units", 2), row: "units" },
        { zone: testid.zone("you", "units", 4), row: "units" },
        { zone: testid.zone("you", "backrow", 3), row: "backrow" },
      ],
    });
    // The board's own swap motion yields to the set piece: no frame for the board.
    expect(rift?.frames.has(testid.board)).toBe(false);
    expect(rift?.frames.get(testid.card("a"))).toBe("exiled");
    // The Cry and the play's end are entries of their own after it.
    const after = entries.slice(entries.indexOf(rift as AnimationEntry) + 1).flatMap((entry) => entry.events);
    expect(after).toEqual([cry, resolved]);
    expect(rift !== undefined && anomalyClaim(rift)).toEqual({ key: ANOMALY_SET_PIECE, holdMs: ANOMALY_RIFT_HOLD_MS });
    // The opponent's view of it: the same entry, on the other side.
    const theirs = planEntries(events, { ...view, viewer: "p2", you: view.opponent, opponent: view.you }, false, createPlayTracker()).find((entry) => entry.rift !== undefined);
    expect(theirs?.rift?.side).toBe("opponent");
    expect(theirs?.rift?.landings[0]?.zone).toBe(testid.zone("opponent", "units", 2));
  });

  it("R3337003 an uncoded boardReplaced, or one naming no Anomaly, plays as before", () => {
    const { view, a, b } = board();
    for (const anomaly of [undefined, "ZZ"]) {
      const entries = planEntries(cast(anomaly, a, b), view, false, createPlayTracker());
      expect(entries.some((entry) => entry.rift !== undefined), String(anomaly)).toBe(false);
      const replaced = entries.find((entry) => entry.type === "boardReplaced");
      expect(replaced?.events, String(anomaly)).toHaveLength(1);
      expect(replaced?.frames.get(testid.board), String(anomaly)).toBe("boardReplaced");
      expect(replaced !== undefined && anomalyClaim(replaced), String(anomaly)).toBeNull();
    }
  });

  it("R3337003 a rift lands no slam: its cards land out of the rift", () => {
    const { view, a, b } = board();
    const entries = planEntries(cast("AL", a, b), view, false, createPlayTracker());
    const slammed = slamEntries(entries, () => "huge", false);
    expect(slammed.find((entry) => entry.rift !== undefined)?.slam).toBeUndefined();
  });

  it("R3337003 the rift holds as anomaly: full under the cap, short, off, speed 2, none under Reduce motion", () => {
    const { view, a, b } = board();
    const heldBy = (settings: { speed?: number; bigMoments?: FxBigMoments }, reduced = false): AnimationEntry | undefined => {
      const { queue, playAll } = queueOf(settings, reduced);
      queue.enqueue(cast("WH", a, b), view);
      return playAll().find((entry) => entry.rift !== undefined);
    };
    const full = heldBy({});
    expect(full?.setPiece).toEqual({ key: ANOMALY_SET_PIECE, holdMs: ANOMALY_RIFT_HOLD_MS });
    expect(full?.setPiece?.holdMs).toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
    expect(full?.durationMs).toBe(ANOMALY_LAND_MS + ANOMALY_RIFT_HOLD_MS);
    const short = heldBy({ bigMoments: "short" });
    expect(short?.setPiece?.holdMs).toBe(SET_PIECE_SHORT_HOLD_MS);
    expect(short?.setPiece?.holdMs).toBeLessThanOrEqual(SET_PIECE_SHORT_HOLD_MS);
    const off = heldBy({ bigMoments: "off" });
    expect(off?.setPiece).toBeUndefined();
    expect(off?.durationMs).toBe(ANOMALY_LAND_MS);
    const fast = heldBy({ speed: 2 });
    expect(fast?.setPiece?.holdMs).toBe(scaleForSpeed(ANOMALY_RIFT_HOLD_MS, 2));
    const reduced = heldBy({}, true);
    expect(reduced?.setPiece).toBeUndefined();
    expect(reduced?.durationMs).toBe(0);
  });
});
