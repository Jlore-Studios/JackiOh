// A Glitch in a practice game (issue #170): after a swap the human plays the AI's former seat and the
// AI the human's (R677), and a voided game shows its result and leaves no last board (R679).
//
// No deck deals a Glitch on cue, so `reduce` is the real one with a hook: a test names the change a
// Glitch would have made (`seatSwaps` up by one, or the voided result), and the hook makes it on the
// state the next accepted action leaves. Everything else is the real core.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { legalActions, type GameState } from "@jackioh/engine";
import { AI_GATE_BUDGET, aiToAct } from "@jackioh/ai";
import type { ActionBody, PlayerId } from "@jackioh/shared";

import { createPracticeCore } from "./core.ts";
import type { PracticeDebug, PracticeRequest, PracticeRequestBody, PracticeResponse, PracticeSnapshot } from "./protocol.ts";

const hook = vi.hoisted(() => ({ next: null as ((state: GameState) => void) | null }));

vi.mock("@jackioh/engine", async (importOriginal) => {
  const real = await importOriginal<typeof import("@jackioh/engine")>();
  return {
    ...real,
    reduce: (...args: Parameters<typeof real.reduce>): ReturnType<typeof real.reduce> => {
      const result = real.reduce(...args);
      if (result.error === undefined && hook.next !== null) {
        hook.next(result.state);
        hook.next = null;
      }
      return result;
    },
  };
});

const HUMAN: PlayerId = "p2";
const AI: PlayerId = "p1";

type Driver = { send(body: PracticeRequestBody): PracticeResponse };

function driver(): Driver {
  const core = createPracticeCore({ now: () => 0, dev: true, budget: AI_GATE_BUDGET });
  let id = 0;
  return {
    send(body) {
      id += 1;
      return core.handle({ id, ...body } as PracticeRequest);
    },
  };
}

function snapshotOf(response: PracticeResponse): PracticeSnapshot {
  if (response.type === "started" || response.type === "snapshot") return response.snapshot;
  throw new Error(`expected a snapshot, got ${JSON.stringify(response).slice(0, 400)}`);
}

function debugOf(d: Driver): PracticeDebug {
  const response = d.send({ type: "debug" });
  if (response.type !== "debug") throw new Error(`expected debug, got ${JSON.stringify(response).slice(0, 400)}`);
  return response.debug;
}

/** Started with the human on p2, the AI's mulligan answered, the human's still open. */
function opened(d: Driver, seed: string): PracticeSnapshot {
  let last = snapshotOf(d.send({ type: "start", config: { seed, difficulty: "easy", humanSeat: HUMAN, deck: { kind: "random" } } }));
  if (last.aiToAct) last = snapshotOf(d.send({ type: "aiStep" }));
  return last;
}

function keepAll(snapshot: PracticeSnapshot): ActionBody {
  const pending = snapshot.view.pending;
  if (pending === null || !pending.forYou || pending.kind !== "mulligan") throw new Error("expected the human's mulligan");
  return { type: "mulligan", keep: pending.options.map((option) => option.key) };
}

beforeEach(() => {
  hook.next = null;
});

describe("R677 a Glitch swap in practice", () => {
  it("R677 after a swap the human sees, acts from and is credited in the AI's former seat, and the AI plays the human's", { timeout: 60_000 }, () => {
    const d = driver();
    const before = opened(d, "glitch-swap");
    expect(before.view.viewer).toBe(HUMAN);

    hook.next = (state) => {
      state.seatSwaps = (state.seatSwaps ?? 0) + 1;
    };
    const swapped = snapshotOf(d.send({ type: "act", action: keepAll(before) }));
    const state = debugOf(d).state as GameState;
    expect(state.seatSwaps).toBe(1);
    // The human's view, legal actions and the AI's turn to act all read the seats as they are now.
    expect(swapped.view.viewer).toBe(AI);
    expect(swapped.legal).toEqual(legalActions(state, AI));
    expect(swapped.aiToAct).toBe(aiToAct(state, HUMAN));

    // The human's next action goes as the seat it plays now, its nonce stream unbroken.
    const action = swapped.legal.find((candidate) => candidate.type === "endTurn") ?? swapped.legal[0];
    if (action === undefined) throw new Error("the human has no legal action after the swap");
    const after = snapshotOf(d.send({ type: "act", action }));
    expect(after.error).toBeNull();
    const log = debugOf(d).log;
    const last = log[log.length - 1];
    expect(last?.playerId).toBe(AI);
    expect(last?.nonce).toBe("h1");

    // And the AI's next action goes as the human's starting seat.
    let next = after;
    for (let n = 0; n < 5 && !next.aiToAct && next.view.result === null; n += 1) {
      const endTurn = next.legal.find((candidate) => candidate.type === "endTurn");
      if (endTurn === undefined) break;
      next = snapshotOf(d.send({ type: "act", action: endTurn }));
    }
    expect(next.aiToAct, "the AI owes an action once the human passes").toBe(true);
    d.send({ type: "aiStep" });
    const aiLog = debugOf(d).log;
    const aiLast = aiLog[aiLog.length - 1];
    expect(aiLast?.playerId).toBe(HUMAN);
    expect(aiLast?.nonce.startsWith("a")).toBe(true);
  });
});

describe("R679 a voided practice game", () => {
  it("R679 shows the voided result and leaves no last board, where a conceded game leaves one", { timeout: 60_000 }, () => {
    const voided = driver();
    const first = opened(voided, "glitch-void");
    hook.next = (state) => {
      state.result = { winner: "draw", reason: "voided" };
    };
    const over = snapshotOf(voided.send({ type: "act", action: keepAll(first) }));
    expect(over.view.result).toEqual({ winner: "draw", reason: "voided" });
    expect(over.lastBoard).toBeUndefined();

    const conceded = driver();
    opened(conceded, "glitch-void");
    const done = snapshotOf(conceded.send({ type: "act", action: { type: "concede" } }));
    expect(done.view.result?.reason).toBe("concede");
    expect(done.lastBoard).toBeDefined();
  });
});
