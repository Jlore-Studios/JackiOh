// R2208000: the AI's clock in a practice game that earns. A game on a server seed decides under
// PRACTICE_EARNING_AI_CLOCK_MS, since the server re-runs its AI moves with no clock (R1303002); every
// other game keeps PRACTICE_AI_CLOCK_MS. A decision the long cap still cuts marks the game, in its save too.
//
// The mocked `decide` runs the real search without its deadline (the fake clock is not the `Date.now()`
// the engine's deadline reads), and records the deadlines it was asked with.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { PlayerId } from "@jackioh/shared";

import { PRACTICE_AI_CLOCK_MS, PRACTICE_EARNING_AI_CLOCK_MS } from "./config.ts";
import { createPracticeCore } from "./core.ts";
import { memorySaveStore, type PracticeSaveStore } from "./saveStore.ts";
import { TUTORIAL_LESSONS } from "../tutorial/lessons.ts";
import { lessonStartConfig } from "../tutorial/start.ts";
import type { PracticeRequest, PracticeRequestBody, PracticeResponse, PracticeStartConfig } from "./protocol.ts";

const plan = vi.hoisted(() => ({ cut: 0, deadlines: [] as (number | undefined)[] }));

vi.mock("@jackioh/ai", async (importOriginal) => {
  const real = await importOriginal<typeof import("@jackioh/ai")>();
  return {
    ...real,
    decide: (...args: Parameters<typeof real.decide>): ReturnType<typeof real.decide> => {
      const [state, seat, options] = args;
      plan.deadlines.push(options.deadlineMs);
      const { deadlineMs: _deadline, ...unclocked } = options;
      const decision = real.decide(state, seat, unclocked);
      if (decision === null || plan.cut === 0) return decision;
      plan.cut -= 1;
      return { ...decision, stats: { ...decision.stats, stoppedBy: "clock" } };
    },
  };
});

const HUMAN: PlayerId = "p2";

/** A worker clock that has started: a decision gets a deadline only when it reads above 0. */
const NOW = 1_000;

type Driver = { send(body: PracticeRequestBody): PracticeResponse };

function driver(saves?: PracticeSaveStore): Driver {
  const core = createPracticeCore({ now: () => NOW, dev: true, budget: AI_GATE_BUDGET, ...(saves === undefined ? {} : { saves }) });
  let id = 0;
  return {
    send(body) {
      id += 1;
      return core.handle({ id, ...body } as PracticeRequest);
    },
  };
}

/** Medium; every `decide` call is a move of the AI's, as the lethal hint (R2291000) asks `lethalHint`, never `decide`. */
function config(over: Partial<PracticeStartConfig> = {}): PracticeStartConfig {
  return { seed: "core-clock", difficulty: "medium", humanSeat: HUMAN, deck: { kind: "random" }, ...over };
}

function claimAnswer(d: Driver): Extract<PracticeResponse, { type: "claim" }> {
  const response = d.send({ type: "claim" });
  if (response.type !== "claim") throw new Error(`expected a claim, got ${JSON.stringify(response).slice(0, 400)}`);
  return response;
}

beforeEach(() => {
  plan.cut = 0;
  plan.deadlines = [];
});

describe("R2208000 an earning practice game's AI clock", () => {
  it("R2208000 an earning game's decision carries PRACTICE_EARNING_AI_CLOCK_MS; a free game, a lesson and a sandbox game carry PRACTICE_AI_CLOCK_MS", () => {
    expect(PRACTICE_EARNING_AI_CLOCK_MS).toBeGreaterThan(PRACTICE_AI_CLOCK_MS);
    const lesson = TUTORIAL_LESSONS[0];
    if (lesson === undefined) throw new Error("no tutorial lesson");
    const cases: [string, PracticeStartConfig, number][] = [
      ["an earning game", config({ earning: true }), PRACTICE_EARNING_AI_CLOCK_MS],
      ["a free game", config(), PRACTICE_AI_CLOCK_MS],
      ["a lesson", lessonStartConfig(lesson), PRACTICE_AI_CLOCK_MS],
      ["a sandbox game", config({ sandbox: true }), PRACTICE_AI_CLOCK_MS],
    ];
    for (const [name, start, cap] of cases) {
      plan.deadlines = [];
      const d = driver();
      expect(d.send({ type: "start", config: start }).type, name).toBe("started");
      expect(d.send({ type: "aiStep" }).type, name).toBe("snapshot");
      expect(plan.deadlines[0], name).toBe(NOW + cap);
    }
  });

  it("R2208000 a cut decision marks an earning game, the mark survives a resume, and the claim's answer carries it beside the claim", () => {
    const start = config({ earning: true });
    const saves = memorySaveStore();
    const before = driver(saves);
    expect(before.send({ type: "start", config: start }).type).toBe("started");
    plan.cut = 1;
    expect(before.send({ type: "aiStep" }).type).toBe("snapshot");
    expect(plan.cut, "the cut decision was taken").toBe(0);
    expect(saves.read()?.clockCut).toBe(true);

    // The reload: a new worker over the same device's save, asked for the game the page remembers.
    const after = driver(saves);
    expect(after.send({ type: "resume", config: start }).type).toBe("started");
    expect(after.send({ type: "act", action: { type: "concede" } }).type).toBe("snapshot");
    const answer = claimAnswer(after);
    expect(answer.clockCut).toBe(true);
    expect(Object.keys(answer.claim).sort(), "the mark is not in the claim the server folds").toEqual(["log", "result", "seed"]);
  });

  it("R2208000 an earning game with no cut decision, and a free game even with one, carry no mark", () => {
    const uncut = memorySaveStore();
    const earned = driver(uncut);
    expect(earned.send({ type: "start", config: config({ earning: true }) }).type).toBe("started");
    expect(earned.send({ type: "aiStep" }).type).toBe("snapshot");
    expect(uncut.read(), "the game is saved").not.toBeNull();
    expect(uncut.read()).not.toHaveProperty("clockCut");
    expect(earned.send({ type: "act", action: { type: "concede" } }).type).toBe("snapshot");
    expect(claimAnswer(earned)).not.toHaveProperty("clockCut");

    const free = memorySaveStore();
    const played = driver(free);
    expect(played.send({ type: "start", config: config() }).type).toBe("started");
    plan.cut = 1;
    expect(played.send({ type: "aiStep" }).type).toBe("snapshot");
    expect(plan.cut, "the cut decision was taken").toBe(0);
    expect(free.read(), "the game is saved").not.toBeNull();
    expect(free.read()).not.toHaveProperty("clockCut");
  });
});
