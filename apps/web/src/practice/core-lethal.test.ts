// R2102000: the practice core's lethal hint. Against the Easy AI, and only in a free game, every
// snapshot asks `lethalHint` for the human's seat and carries `lethal: true` when it says yes; at
// Medium and Hard, and in a tutorial lesson (whose `difficulty` is a placeholder), it never asks. A
// check that throws gives no hint and the game goes on.
//
// `@jackioh/ai` is mocked with a `lethalHint` that is the real one until a test tells it to answer
// yes or throw, and that records the seats and deadlines it was asked with. The check's own tests, the hidden-card
// invariance among them, are the Rust AI's (`crates/ai/tests/ai/lethal_hint.rs`).

import { beforeEach, describe, expect, it, vi } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { PlayerId } from "@jackioh/shared";

import { PRACTICE_LETHAL_HINT_CLOCK_MS } from "./config.ts";
import { createPracticeCore } from "./core.ts";
import { TUTORIAL_LESSONS } from "../tutorial/lessons.ts";
import { lessonStartConfig } from "../tutorial/start.ts";
import type { PracticeRequest, PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "./protocol.ts";

type Mode = "real" | "yes" | "throw";

const plan = vi.hoisted(() => ({ mode: "real" as Mode, seats: [] as string[], deadlines: [] as (number | undefined)[] }));

vi.mock("@jackioh/ai", async (importOriginal) => {
  const real = await importOriginal<typeof import("@jackioh/ai")>();
  return {
    ...real,
    lethalHint: (...args: Parameters<typeof real.lethalHint>): boolean => {
      plan.seats.push(args[1]);
      plan.deadlines.push(args[2]);
      switch (plan.mode) {
        case "real":
          return real.lethalHint(...args);
        case "yes":
          return true;
        case "throw":
          throw new Error("scripted lethalHint failure");
      }
    },
  };
});

const HUMAN: PlayerId = "p2";

type Driver = { send(body: PracticeRequestBody): PracticeResponse };

function driver(now = 0): Driver {
  const core = createPracticeCore({ now: () => now, dev: true, budget: AI_GATE_BUDGET });
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

function config(over: Partial<PracticeStartConfig> = {}): PracticeStartConfig {
  return { seed: "core-lethal", difficulty: "easy", humanSeat: HUMAN, deck: { kind: "random" }, ...over };
}

beforeEach(() => {
  plan.mode = "real";
  plan.seats = [];
  plan.deadlines = [];
});

describe("R2102000 the practice core's lethal hint", () => {
  it("R2102000 an Easy free game asks the check for the human's seat, and its snapshot says lethal when it says yes", () => {
    plan.mode = "yes";
    const d = driver();
    const started = snapshotOf(d.send({ type: "start", config: config() }));
    expect(started.lethal).toBe(true);
    expect(plan.seats).toEqual([HUMAN]);

    // The AI owes its mulligan; its step's snapshot asks again, for the human's seat.
    expect(started.aiToAct).toBe(true);
    const stepped = snapshotOf(d.send({ type: "aiStep" }));
    expect(stepped.lethal).toBe(true);
    expect(plan.seats.length).toBeGreaterThan(1);
    expect(plan.seats.every((seat) => seat === HUMAN)).toBe(true);
  });

  it("R2102000 Medium, Hard and a tutorial lesson never ask the check and carry no lethal", () => {
    plan.mode = "yes";
    const lesson = TUTORIAL_LESSONS[0];
    if (lesson === undefined) throw new Error("no tutorial lesson");
    const configs: PracticeStartConfig[] = [
      config({ difficulty: "medium" }),
      config({ difficulty: "hard" }),
      lessonStartConfig(lesson),
    ];
    for (const start of configs) {
      const snapshot = snapshotOf(driver().send({ type: "start", config: start }));
      expect("lethal" in snapshot, start.lesson ?? start.difficulty).toBe(false);
    }
    expect(plan.seats).toEqual([]);
  });

  it("R2102000 a check that throws gives no hint and the game goes on", () => {
    plan.mode = "throw";
    const d = driver();
    const started = d.send({ type: "start", config: config() });
    expect(started.type).toBe("started");
    expect("lethal" in snapshotOf(started)).toBe(false);
    const stepped = d.send({ type: "aiStep" });
    expect(stepped.type).toBe("snapshot");
    expect("lethal" in snapshotOf(stepped)).toBe(false);
    expect(plan.seats.length).toBeGreaterThan(1);
  });

  it("R2102000 the check runs under its own clock when the worker has one, and under none when it has not", () => {
    plan.mode = "yes";
    driver(1_000).send({ type: "start", config: config() });
    expect(plan.deadlines).toEqual([1_000 + PRACTICE_LETHAL_HINT_CLOCK_MS]);
    plan.deadlines = [];
    driver().send({ type: "start", config: config() });
    expect(plan.deadlines).toEqual([undefined]);
  });

  it("R2102000 the real check says no while the mulligans are open", () => {
    const snapshot = snapshotOf(driver().send({ type: "start", config: config() }));
    expect(snapshot.view.phase).toBe("mulligan");
    expect("lethal" in snapshot).toBe(false);
    expect(plan.seats).toEqual([HUMAN]);
  });
});
