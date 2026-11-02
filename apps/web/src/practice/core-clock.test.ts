// R2208000, R2560000: the AI's clock in a practice game that earns. The server does not re-run a
// claim's AI moves, so a game on a server seed (`earning: true`) decides under PRACTICE_AI_CLOCK_MS like
// any other, and a cut decision marks nothing: not the game's save, not the claim's answer.
//
// `@jackioh/ai` is mocked with a `decide` that is the real search until a test says its next decisions
// were cut short by the clock (`stoppedBy: "clock"`). The real search runs without its deadline: the
// fake clock is not `Date.now()`, which the engine's deadline reads.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { PlayerId } from "@jackioh/shared";

import { PRACTICE_AI_CLOCK_MS } from "./config.ts";
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

describe("R2208000 R2560000 an earning practice game's AI clock", () => {
  it("R2208000 R2560000 every practice game's decision carries PRACTICE_AI_CLOCK_MS, an earning game's too", () => {
    const lesson = TUTORIAL_LESSONS[0];
    if (lesson === undefined) throw new Error("no tutorial lesson");
    const cases: [string, PracticeStartConfig][] = [
      ["an earning game", config({ earning: true })],
      ["a free game", config()],
      ["a lesson", lessonStartConfig(lesson)],
      ["a sandbox game", config({ sandbox: true })],
    ];
    for (const [name, start] of cases) {
      plan.deadlines = [];
      const d = driver();
      expect(d.send({ type: "start", config: start }).type, name).toBe("started");
      expect(d.send({ type: "aiStep" }).type, name).toBe("snapshot");
      expect(plan.deadlines[0], name).toBe(NOW + PRACTICE_AI_CLOCK_MS);
    }
  });

  it("R2208000 R2560000 a cut decision marks no earning game: its save and its claim's answer hold nothing more", () => {
    const start = config({ earning: true });
    const saves = memorySaveStore();
    const before = driver(saves);
    expect(before.send({ type: "start", config: start }).type).toBe("started");
    plan.cut = 1;
    expect(before.send({ type: "aiStep" }).type).toBe("snapshot");
    expect(plan.cut, "the cut decision was taken").toBe(0);
    expect(Object.keys(saves.read() ?? {}).sort()).toEqual(["aiCursor", "catalog", "config", "hash", "log", "versions"]);

    // The reload: a new worker over the same device's save, asked for the game the page remembers.
    const after = driver(saves);
    expect(after.send({ type: "resume", config: start }).type).toBe("started");
    expect(after.send({ type: "act", action: { type: "concede" } }).type).toBe("snapshot");
    const answer = claimAnswer(after);
    expect(Object.keys(answer).sort()).toEqual(["claim", "id", "type"]);
    expect(Object.keys(answer.claim).sort(), "the claim the server folds is the seed, the result and the log").toEqual(["log", "result", "seed"]);
  });
});
