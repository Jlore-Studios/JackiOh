// A finished practice game's claim (SPEC §14.8, R1303000–R1303002): what the worker hands the page
// for the server to check, and the game the server's tests check it against.
//
// The REAL core plays here, as in `core.test.ts`, but at the browser's own budget, AI_BUDGET, with a
// frozen clock (`now: () => 0`), so the wall-clock cap never cuts a decision short: every AI move is
// the one the server's clock-free re-run makes (R1303002). The first test's game is the fixture
// `crates/server/tests/fixtures/practice-claim.json`, which the server's
// `r1303002_the_ai_rerun_matches_the_game_the_worker_played` folds and pays; after an AI, card or
// engine change that plays it differently, record it again with
// `JACKIOH_RECORD_PRACTICE_CLAIM=1 pnpm --dir apps/web exec vitest run src/practice/claim.test.ts`.
// A training lane's promotion records it with its gate (learn.yml; training/README.md, What a lane
// may change): it is the one file outside crates/ai/** and training/history/** a lane may change.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { createRng, subsystems } from "@jackioh/engine";
import type { GameState, Rng } from "@jackioh/engine";
import { AI_BUDGET } from "@jackioh/ai";

import { createPracticeCore } from "./core.ts";
import type { PracticeCore, PracticeCoreEnv } from "./core.ts";
import { memorySaveStore } from "./saveStore.ts";
import type {
  PracticeClaim,
  PracticeDebug,
  PracticeRequest,
  PracticeRequestBody,
  PracticeResponse,
  PracticeSnapshot,
  PracticeStartConfig,
} from "./protocol.ts";

// ---------------------------------------------------------------------------------------------
// the driver (core.test.ts's, at the browser's budget)
// ---------------------------------------------------------------------------------------------

const ENV: PracticeCoreEnv = { now: () => 0, dev: true, budget: AI_BUDGET };

/** Most requests the fixture's game sends before it gives up on the game ending by itself. */
const GAME_CAP = 2000;

const FIXTURE_PATH = join(dirname(fileURLToPath(import.meta.url)), "../../../../crates/server/tests/fixtures/practice-claim.json");
const RECORD = process.env.JACKIOH_RECORD_PRACTICE_CLAIM === "1";
const RECORD_COMMAND = "JACKIOH_RECORD_PRACTICE_CLAIM=1 pnpm --dir apps/web exec vitest run src/practice/claim.test.ts";

type Driver = { core: PracticeCore; send(body: PracticeRequestBody): PracticeResponse };

function driver(env: PracticeCoreEnv = ENV): Driver {
  const core = createPracticeCore(env);
  let id = 0;
  return {
    core,
    send(body) {
      id += 1;
      const response = core.handle({ id, ...body } as PracticeRequest);
      expect(response.id).toBe(id);
      return response;
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

function claimOf(d: Driver): PracticeClaim {
  const response = d.send({ type: "claim" });
  if (response.type !== "claim") throw new Error(`expected a claim, got ${JSON.stringify(response).slice(0, 400)}`);
  return response.claim;
}

/**
 * One step: the AI's when it owes one, else the human by §10.7's random policy on the true state,
 * from the seat it plays now (R677). Null once nobody has anything to do.
 */
function step(d: Driver, rng: Rng, last: PracticeSnapshot): PracticeResponse | null {
  if (last.view.result !== null) return null;
  if (last.aiToAct) return d.send({ type: "aiStep" });
  const body = subsystems.chooseAction(debugOf(d).state as GameState, last.view.viewer, rng);
  if (body === null) return null;
  return d.send({ type: "act", action: body });
}

/** The game played out from `from`, the human on `${seed}:human`; the last snapshot. */
function playOut(d: Driver, seed: string, from: PracticeSnapshot, cap = GAME_CAP): PracticeSnapshot {
  const rng = createRng(`${seed}:human`);
  let last = from;
  for (let n = 0; n < cap && last.view.result === null; n += 1) {
    const response = step(d, rng, last);
    if (response === null) break;
    last = snapshotOf(response);
  }
  return last;
}

function earning(over: Partial<PracticeStartConfig> = {}): PracticeStartConfig {
  return { seed: "practice-claim-test", difficulty: "easy", humanSeat: "p2", deck: { kind: "random" }, earning: true, ...over };
}

// ---------------------------------------------------------------------------------------------
// the fixture
// ---------------------------------------------------------------------------------------------

describe("R1303002 the claim the server re-runs the AI on", () => {
  it("R1303002 the worker plays the claim fixture move for move", { timeout: 60_000 }, () => {
    const config = earning({ seed: "practice-claim-fixture" });
    const d = driver();
    const last = playOut(d, config.seed, snapshotOf(d.send({ type: "start", config })));
    expect(last.view.result, `the fixture's game ends within ${String(GAME_CAP)} requests`).not.toBeNull();

    const claim = claimOf(d);
    expect(Object.keys(claim).sort()).toEqual(["log", "result", "seed"]);
    expect(claim.seed).toBe(config.seed);
    expect(claim.log).toEqual(debugOf(d).log);
    const fixture = {
      seed: claim.seed,
      difficulty: config.difficulty,
      humanSeat: config.humanSeat,
      deck: { kind: "random", lean: null },
      result: claim.result,
      log: claim.log,
    };

    if (RECORD) {
      mkdirSync(dirname(FIXTURE_PATH), { recursive: true });
      writeFileSync(FIXTURE_PATH, `${JSON.stringify(fixture, null, 2)}\n`);
      return;
    }
    let recorded: unknown = null;
    try {
      recorded = JSON.parse(readFileSync(FIXTURE_PATH, "utf8"));
    } catch {
      // Missing or unreadable: the comparison below says how to record it.
    }
    expect(recorded, `the worker no longer plays crates/server/tests/fixtures/practice-claim.json; record it again: ${RECORD_COMMAND}`).toEqual(
      fixture,
    );
  });
});

// ---------------------------------------------------------------------------------------------
// what the worker answers
// ---------------------------------------------------------------------------------------------

describe("R1303001 a claim names the seed its game began on", () => {
  it("R1303001 a game resumed after a reload claims on the seed it began on", () => {
    const config = earning({ seed: "practice-claim-resume" });
    const saves = memorySaveStore();
    const before = driver({ ...ENV, saves });
    const opened = snapshotOf(before.send({ type: "start", config }));
    const left = playOut(before, config.seed, opened, 12);
    expect(left.view.result, "the game is still in progress when the page goes").toBeNull();
    const logBefore = debugOf(before).log;
    expect(logBefore.length).toBeGreaterThan(0);

    // The reload: a new worker over the same device's save, asked for the game the page remembers.
    const after = driver({ ...ENV, saves });
    const resumed = after.send({ type: "resume", config });
    expect(resumed.type).toBe("started");
    const over = snapshotOf(after.send({ type: "act", action: { type: "concede" } }));
    expect(over.view.result).not.toBeNull();

    const claim = claimOf(after);
    expect(claim.seed).toBe(config.seed);
    expect(claim.result).toBe("loss");
    expect(claim.log.slice(0, logBefore.length)).toEqual(logBefore);
    expect(claim.log).toHaveLength(logBefore.length + 1);
    expect(claim.log[logBefore.length]).toMatchObject({ type: "concede" });
  });
});

describe("R1303000 the worker answers a claim only for a finished game on a server seed", () => {
  it("claim is refused before the end and for a game not on a server seed", () => {
    const none = driver();
    expect(none.send({ type: "claim" }).type, "no game is running").toBe("failed");

    const running = driver();
    snapshotOf(running.send({ type: "start", config: earning({ seed: "practice-claim-early" }) }));
    const early = running.send({ type: "claim" });
    expect(early.type, "the game is not over: its log would name the AI's hidden cards").toBe("failed");
    expect(JSON.stringify(early)).not.toContain('"log"');
    expect(snapshotOf(running.send({ type: "act", action: { type: "concede" } })).view.result).not.toBeNull();
    expect(claimOf(running).result).toBe("loss");

    const unpaid = driver();
    const { earning: _paid, ...free } = earning({ seed: "practice-claim-free" });
    snapshotOf(unpaid.send({ type: "start", config: free }));
    expect(snapshotOf(unpaid.send({ type: "act", action: { type: "concede" } })).view.result).not.toBeNull();
    const refused = unpaid.send({ type: "claim" });
    expect(refused.type, "a game not on a server seed is never claimed").toBe("failed");
    expect(JSON.stringify(refused)).not.toContain('"log"');
  });
});
