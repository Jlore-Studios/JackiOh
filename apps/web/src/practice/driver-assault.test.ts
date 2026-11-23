// R4582000, R4582001 (issue #4582): an Infinite Assault fight in the practice driver. A config with
// `assault` is dealt by the machine from its setup (`crates/shared`'s `deal.rs`: the seed, both decks
// and handicaps, the enemy's marks at the AI's seat); it brings no last board and leaves none, is kept
// in its own save slot while in progress, and once over answers `assaultReport` with what the
// Assault reads off the machine's setup and log. The mock below stands in for part B's
// `assaultFightReport`, as the dungeon's part tests mocked `runFightSetup`.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import { createRng, subsystems } from "@jackioh/engine";
import { AI_DIFFICULTY, HUMAN_HANDICAP } from "@jackioh/engine/config";
import type { GameState, Rng } from "@jackioh/engine";
import type { AssaultAction, AssaultFightSetup, AssaultLevel, RunFightSetup } from "@jackioh/run";
import type { PlayerId } from "@jackioh/shared";

import { createPracticeDriver } from "./driver.ts";
import { PRACTICE_PRESETS } from "./decks.ts";
import { memorySaveStore, type PracticeSaveStore } from "./saveStore.ts";
import type { PracticeDebug, PracticeRequest, PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "./protocol.ts";

const plan = vi.hoisted(() => ({
  calls: [] as { game: unknown; human: unknown }[],
  answer: null as null | ({ action: unknown } | { error: string }),
}));

vi.mock("../wasm/index.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../wasm/index.ts")>();
  return {
    ...actual,
    assaultFightReport: (game: unknown, human: unknown) => {
      plan.calls.push({ game, human });
      if (plan.answer === null) throw new Error("no assault report answer scripted");
      return plan.answer;
    },
  };
});

function presetCards(index: number): string[] {
  const preset = PRACTICE_PRESETS[index];
  if (preset === undefined) throw new Error(`no practice preset ${String(index)}`);
  return [...preset.cards];
}

const HUMAN_DECK = presetCards(0);
const ENEMY_DECK = presetCards(1);
const MARKS = [{ index: 0, radiant: true, upgrades: 1 }];

const LEVEL: AssaultLevel = {
  level: 1,
  enemy: "Rogue Model",
  doctrine: null,
  commander: null,
  escalations: [],
  upgrades: [],
  rarities: [20, 0, 0, 0, 0],
  deck: ENEMY_DECK,
  marks: [],
  handicap: { deckSize: ENEMY_DECK.length, manaBonus: 0, manaCap: 4, extraOpeningCards: 0, extraDrawsPerTurn: 0, heroHealth: 20 },
};

const SETUP: AssaultFightSetup = {
  seed: "ASSAULTSED-2-1",
  humanDeck: HUMAN_DECK,
  humanHandicap: { ...HUMAN_HANDICAP, deckSize: HUMAN_DECK.length },
  enemyDeck: ENEMY_DECK,
  enemyHandicap: { ...AI_DIFFICULTY.easy, deckSize: ENEMY_DECK.length },
  enemyMarks: MARKS,
  level: LEVEL,
};

/** A fight whose config says otherwise everywhere the setup decides: the seed and the deck. */
const FIGHT: PracticeStartConfig = {
  seed: SETUP.seed,
  difficulty: "easy",
  humanSeat: "p1",
  deck: { kind: "random" },
  assault: SETUP,
};

const WON: AssaultAction = { kind: "fightOver", won: true, turns: 7 };

const FREE: PracticeStartConfig = { seed: "driver-assault-free", difficulty: "easy", humanSeat: "p1", deck: { kind: "random" } };

const RUN_SETUP: RunFightSetup = {
  seed: "RUNSED-9-9",
  humanDeck: HUMAN_DECK,
  humanHandicap: { ...HUMAN_HANDICAP, deckSize: HUMAN_DECK.length },
  humanMarks: [],
  enemyDeck: ENEMY_DECK,
  enemyHandicap: { ...AI_DIFFICULTY.easy, deckSize: ENEMY_DECK.length },
  enemy: { id: "cave-troll", name: "Cave Troll", tier: 1, deck: ENEMY_DECK, artifacts: [] },
};

const RUN_FIGHT: PracticeStartConfig = {
  seed: RUN_SETUP.seed,
  difficulty: "hard",
  humanSeat: "p1",
  deck: { kind: "random" },
  run: RUN_SETUP,
};

type Driver = { send(body: PracticeRequestBody): PracticeResponse };

function driver(saves?: PracticeSaveStore): Driver {
  const core = createPracticeDriver({ now: () => 0, dev: true, budget: AI_GATE_BUDGET, ...(saves === undefined ? {} : { saves }) });
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

function debugOf(response: PracticeResponse): PracticeDebug {
  if (response.type === "debug") return response.debug;
  throw new Error(`expected debug, got ${JSON.stringify(response).slice(0, 400)}`);
}

beforeEach(() => {
  plan.calls = [];
  plan.answer = null;
});

describe("R4582000 assaultReport answers assaultFightReport over the finished fight", () => {
  it("R4582000 assaultReport answers assaultFightReport over the finished fight's setup and log, for the human's seat", () => {
    const d = driver();
    snapshotOf(d.send({ type: "start", config: FIGHT }));
    snapshotOf(d.send({ type: "act", action: { type: "concede" } }));
    const debug = debugOf(d.send({ type: "debug" }));
    plan.answer = { action: WON };
    const answered = d.send({ type: "assaultReport" });
    if (answered.type !== "assaultReport" || !("action" in answered)) throw new Error(`expected a report, got ${JSON.stringify(answered)}`);
    expect(plan.calls).toHaveLength(1);
    expect(plan.calls[0]?.human).toBe("p1");
    expect(plan.calls[0]?.game).toMatchObject({ seed: debug.seed, decks: debug.decks, handicaps: debug.handicaps, log: debug.log });
    expect(answered.action).toEqual(WON);
    expect(JSON.stringify(answered), "the report names no nonce (rule 7)").not.toContain("nonce");
  });

  it("R4582000 the Assault's refusal crosses as its error", () => {
    const d = driver();
    snapshotOf(d.send({ type: "start", config: FIGHT }));
    snapshotOf(d.send({ type: "act", action: { type: "concede" } }));
    plan.answer = { error: "the fight does not fold" };
    const answered = d.send({ type: "assaultReport" });
    expect(answered).toEqual(expect.objectContaining({ type: "assaultReport", error: "the fight does not fold" }));
  });

  it("R4582000 assaultReport is refused before the end and for a game that is no Assault fight", () => {
    const fight = driver();
    snapshotOf(fight.send({ type: "start", config: FIGHT }));
    expect(fight.send({ type: "assaultReport" })).toEqual(expect.objectContaining({ type: "failed" }));

    const free = driver();
    snapshotOf(free.send({ type: "start", config: FREE }));
    snapshotOf(free.send({ type: "act", action: { type: "concede" } }));
    expect(free.send({ type: "assaultReport" })).toEqual(expect.objectContaining({ type: "failed" }));
    expect(plan.calls, "the Assault is never asked").toEqual([]);
  });
});

describe("R4582001 an Assault fight left unfinished waits where it stood", () => {
  /** The true state behind a driver, the test's only door into it (`debugOf` reads an answer). */
  function debugDriver(d: Driver): PracticeDebug {
    const response = d.send({ type: "debug" });
    if (response.type !== "debug") throw new Error(`expected debug, got ${JSON.stringify(response).slice(0, 400)}`);
    return response.debug;
  }

  /** One step of the fight: the AI's when it owes one, else the human's random policy on the true state. */
  function fightStep(d: Driver, human: PlayerId, rng: Rng, last: PracticeSnapshot): PracticeSnapshot | null {
    if (last.view.result !== null) return null;
    if (last.aiToAct) return snapshotOf(d.send({ type: "aiStep" }));
    const body = subsystems.chooseAction(debugDriver(d).state as GameState, human, rng);
    if (body === null) return null;
    return snapshotOf(d.send({ type: "act", action: body }));
  }

  it("R4582001 an Assault fight played two turns is kept in its own slot and resumes on its turn", { timeout: 120_000 }, () => {
    const saves = memorySaveStore();
    // A free game the device already holds: the fight must never touch it.
    const free = driver(saves);
    snapshotOf(free.send({ type: "start", config: FREE }));
    const freeSave = saves.read();
    expect(freeSave).not.toBeNull();

    const human: PlayerId = "p1";
    const d = driver(saves);
    let last = snapshotOf(d.send({ type: "start", config: FIGHT }));
    const rng = createRng(`${SETUP.seed}:human`);
    for (let n = 0; n < 400 && last.view.turn < 2 && last.view.result === null; n += 1) {
      const next = fightStep(d, human, rng, last);
      if (next === null) break;
      last = next;
    }
    expect(last.view.result, "the walk leaves the fight in progress").toBeNull();
    expect(last.view.turn, "two turns are played").toBeGreaterThanOrEqual(2);

    const before = debugDriver(d);
    const fightSave = saves.readAssaultFight();
    expect(fightSave, "the fight is kept in its own slot").not.toBeNull();
    expect(fightSave?.log).toEqual(before.log);
    expect(fightSave?.hash).toBe(before.hash);
    expect(saves.read(), "the free game's save is untouched").toEqual(freeSave);

    // The reload: a fresh driver over the same store, named by the fight's seed as the page names it.
    const again = driver(saves);
    const resumed = again.send({ type: "resume", config: { ...FIGHT, seed: SETUP.seed } });
    expect(resumed.type).toBe("started");
    if (resumed.type !== "started") throw new Error(`the fight did not resume: ${JSON.stringify(resumed).slice(0, 400)}`);
    expect(Object.keys(resumed).sort()).toEqual(["aiSeat", "defs", "id", "snapshot", "type"]);
    expect(resumed.snapshot).toEqual(last);
    expect(JSON.stringify(resumed), "the resumed answer names no nonce (rule 7)").not.toContain("nonce");
    const after = debugDriver(again);
    expect(after.hash, "the fight reaches the hash it was left at").toBe(before.hash);
    expect(after.log).toEqual(before.log);

    // A finished fight clears its slot.
    const over = snapshotOf(again.send({ type: "act", action: { type: "concede" } }));
    expect(over.view.result).not.toBeNull();
    expect(saves.readAssaultFight(), "a finished fight keeps nothing").toBeNull();
    expect(saves.read(), "the free game is still there").toEqual(freeSave);
  });

  it("R4582001 an Assault fight resumes only from its own slot", () => {
    const saves = memorySaveStore();
    const d = driver(saves);
    // A free game saved under the fight's seed: asked as a fight, it is refused and leaves no game running.
    snapshotOf(d.send({ type: "start", config: { ...FREE, seed: SETUP.seed } }));
    expect(d.send({ type: "resume", config: { ...FIGHT, seed: SETUP.seed } }).type).toBe("failed");
    expect(d.send({ type: "aiStep" }).type, "a refused resume leaves no game running").toBe("failed");
    // A run fight is kept beside it: dealing the Assault fight leaves the run fight's slot alone.
    snapshotOf(d.send({ type: "start", config: RUN_FIGHT }));
    const runSave = saves.readRunFight();
    expect(runSave).not.toBeNull();
    snapshotOf(d.send({ type: "start", config: FIGHT }));
    expect(saves.readRunFight(), "an Assault start leaves the run fight kept").toEqual(runSave);
    // A fight's own save resumes it, but no other attempt's.
    const other = driver(saves);
    expect(other.send({ type: "resume", config: { ...FIGHT, seed: "ASSAULTSED-9-9" } }).type).toBe("failed");
    expect(other.send({ type: "resume", config: { ...FIGHT, seed: SETUP.seed } }).type).toBe("started");
  });
});
