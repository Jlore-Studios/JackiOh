// B22, B25, R2706000 (DR14 part E, #2706): a dungeon run's fight in the practice driver. A config with
// `run` plays the setup's seed, decks, handicaps and the human's deck marks, the AI at the other seat;
// it brings no last board and leaves none, is never saved or kept, and once over answers `runReport`
// with what the run reads off its setup and log. The machine is Rust (`crates/shared/tests/practice.rs`
// holds the same rulings natively); these hold what the web sees of it, through the real WebAssembly module.

import { describe, expect, it } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import { createRng, subsystems } from "@jackioh/engine";
import { AI_DIFFICULTY, HUMAN_HANDICAP } from "@jackioh/engine/config";
import type { GameState, Rng } from "@jackioh/engine";
import type { RunFightSetup } from "@jackioh/run";
import type { PlayerId } from "@jackioh/shared";

import { createPracticeDriver } from "./driver.ts";
import { PRACTICE_PRESETS } from "./decks.ts";
import { memorySaveStore, type PracticeSaveStore } from "./saveStore.ts";
import type { PracticeDebug, PracticeRequest, PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "./protocol.ts";

function presetCards(index: number): string[] {
  const preset = PRACTICE_PRESETS[index];
  if (preset === undefined) throw new Error(`no practice preset ${String(index)}`);
  return [...preset.cards];
}

const HUMAN_DECK = presetCards(0);
const ENEMY_DECK = presetCards(1);
const MARKS = [{ index: 0, radiant: true, upgrades: 1 }];

const SETUP: RunFightSetup = {
  seed: "RUNSED-2-1",
  humanDeck: HUMAN_DECK,
  humanHandicap: { ...HUMAN_HANDICAP, deckSize: HUMAN_DECK.length, heroHealth: 17 },
  humanMarks: MARKS,
  enemyDeck: ENEMY_DECK,
  enemyHandicap: { ...AI_DIFFICULTY.easy, deckSize: ENEMY_DECK.length, heroHealth: 20 },
  enemy: { id: "cave-troll", name: "Cave Troll", heroHealth: 20, deck: ENEMY_DECK, artifacts: [] },
};

/** A fight whose config says otherwise everywhere the setup decides: the seed, the tier, the deck, a last board. */
const FIGHT: PracticeStartConfig = {
  seed: "ignored",
  difficulty: "hard",
  humanSeat: "p1",
  deck: { kind: "random" },
  run: SETUP,
  lastBoard: [{ defId: HUMAN_DECK[0] ?? "", radiant: false }],
};

const FREE: PracticeStartConfig = { seed: "driver-run-free", difficulty: "easy", humanSeat: "p1", deck: { kind: "random" } };

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

describe("B22 a dungeon run's fight starts from the run's setup", () => {
  it("B22 a run's fight is created from its setup: its seed, decks, handicaps and marks, the AI at p2", () => {
    const d = driver();
    const started = d.send({ type: "start", config: FIGHT });
    if (started.type !== "started") throw new Error(`the fight did not start: ${JSON.stringify(started)}`);
    expect(started.aiSeat).toBe("p2");
    // Each hero starts at its setup's health: the run's HP and the enemy's.
    expect(started.snapshot.view.you.hero.health).toBe(17);
    expect(started.snapshot.view.opponent.hero.health).toBe(20);

    // The seed, the decks and both handicaps are the setup's; no seat is dealt (R433) and no last board comes in.
    const debug = debugOf(d.send({ type: "debug" }));
    expect(debug.seed).toBe(SETUP.seed);
    expect(debug.decks).toEqual([HUMAN_DECK, ENEMY_DECK]);
    expect(debug.handicaps).toEqual({ p1: SETUP.humanHandicap, p2: SETUP.enemyHandicap });
    expect(debug.deckMarks).toEqual({ p1: MARKS });
    expect(debug).not.toHaveProperty("dealt");
    expect(debug).not.toHaveProperty("lastBoards");
  });

  it("B22 a setup without marks creates no deckMarks, so the game is created as any other", () => {
    const d = driver();
    d.send({ type: "start", config: { ...FIGHT, run: { ...SETUP, humanMarks: [] } } });
    expect(debugOf(d.send({ type: "debug" }))).not.toHaveProperty("deckMarks");
  });

  it("B22 a free game is created as before, with no deckMarks", () => {
    const d = driver();
    d.send({ type: "start", config: FREE });
    const debug = debugOf(d.send({ type: "debug" }));
    expect(debug).not.toHaveProperty("deckMarks");
    expect(debug).toHaveProperty("dealt", ["p1"]);
  });
});

describe("R3476000 a run fight left unfinished waits where it stood", () => {
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

  it("R3476000 a run fight played three turns resumes on its turn with its board, hands and log", { timeout: 120_000 }, () => {
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
    for (let n = 0; n < 400 && last.view.turn < 3 && last.view.result === null; n += 1) {
      const next = fightStep(d, human, rng, last);
      if (next === null) break;
      last = next;
    }
    expect(last.view.result, "the walk leaves the fight in progress").toBeNull();
    expect(last.view.turn, "three turns are played").toBeGreaterThanOrEqual(3);

    const before = debugDriver(d);
    const fightSave = saves.readRunFight();
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

    // The AI picks its stream up at the saved cursor: both cores play on alike.
    const rngA = createRng(`${SETUP.seed}:next`);
    const rngB = createRng(`${SETUP.seed}:next`);
    let a = last;
    let b = resumed.snapshot;
    for (let n = 0; n < 20 && a.view.result === null; n += 1) {
      const ra = fightStep(d, human, rngA, a);
      const rb = fightStep(again, human, rngB, b);
      if (ra === null || rb === null) break;
      a = ra;
      b = rb;
      expect(rb).toEqual(ra);
    }
    expect(debugDriver(again).hash).toBe(debugDriver(d).hash);

    // A finished fight clears its slot, and the run's report carries no log either.
    const over = snapshotOf(again.send({ type: "act", action: { type: "concede" } }));
    expect(over.view.result).not.toBeNull();
    expect(saves.readRunFight(), "a finished fight keeps nothing").toBeNull();
    expect(saves.read(), "the free game is still there").toEqual(freeSave);
    const reported = again.send({ type: "runReport" });
    expect(reported.type).toBe("runReport");
    expect(JSON.stringify(reported), "the report names no nonce (rule 7)").not.toContain("nonce");
  });

  it("R3476000 a missing fight, another fight, or a free game's save resumes no fight", () => {
    const saves = memorySaveStore();
    const d = driver(saves);
    snapshotOf(d.send({ type: "start", config: FREE }));
    // The free game's save is no fight: asked as one, it is refused and leaves no game running.
    expect(d.send({ type: "resume", config: { ...FIGHT, seed: SETUP.seed } }).type).toBe("failed");
    expect(d.send({ type: "aiStep" }).type, "a refused resume leaves no game running").toBe("failed");
    // A fight's own save resumes it, but no other fight's.
    snapshotOf(d.send({ type: "start", config: FIGHT }));
    expect(saves.readRunFight()).not.toBeNull();
    const other = driver(saves);
    expect(other.send({ type: "resume", config: { ...FIGHT, seed: "RUNSED-9-9" } }).type).toBe("failed");
    expect(other.send({ type: "resume", config: { ...FIGHT, seed: SETUP.seed } }).type).toBe("started");
    // The free game is still there to resume too: the slots never overwrite each other.
    expect(driver(saves).send({ type: "resume", config: FREE }).type).toBe("started");
  });
});

describe("R2706000 a run's fight is a game of its own kind", () => {
  it("R2706000 it brings no last board and leaves none, leaves the free game's save as it was, and is not kept", () => {
    const saves = memorySaveStore();
    const d = driver(saves);
    expect(d.send({ type: "start", config: FREE }).type).toBe("started");
    const freeSave = saves.read();
    expect(freeSave).not.toBeNull();

    snapshotOf(d.send({ type: "start", config: FIGHT }));
    expect(debugOf(d.send({ type: "debug" }))).not.toHaveProperty("lastBoards");
    const over = snapshotOf(d.send({ type: "act", action: { type: "concede" } }));
    expect(over.view.result).toEqual(expect.objectContaining({ winner: "p2" }));
    expect(over).not.toHaveProperty("lastBoard");
    expect(over).not.toHaveProperty("replay");
    expect(saves.read()).toBe(freeSave);
    expect(saves.readReplays()).toEqual([]);

    // The free game the device saved is still there to pick up.
    expect(d.send({ type: "resume", config: FREE }).type).toBe("started");
  });
});

describe("B25 runReport answers the run's FightOver once the fight is over", () => {
  it("B25 runReport answers the run's FightOver from the finished fight: its setup and log, for the human's seat", () => {
    const d = driver();
    snapshotOf(d.send({ type: "start", config: FIGHT }));
    snapshotOf(d.send({ type: "act", action: { type: "concede" } }));
    const answered = d.send({ type: "runReport" });
    if (answered.type !== "runReport" || !("action" in answered)) throw new Error(`expected a report, got ${JSON.stringify(answered)}`);
    // The human conceded: the run's fight is lost, at the HP it began with, the log never crossing.
    expect(answered.action).toMatchObject({ type: "fightOver", won: false, hp: 17 });
    expect(JSON.stringify(answered)).not.toContain("nonce");
  });

  it("B25 runReport is refused before the end and for a game that is no fight", () => {
    const fight = driver();
    snapshotOf(fight.send({ type: "start", config: FIGHT }));
    expect(fight.send({ type: "runReport" })).toEqual(expect.objectContaining({ type: "failed" }));

    const free = driver();
    snapshotOf(free.send({ type: "start", config: FREE }));
    snapshotOf(free.send({ type: "act", action: { type: "concede" } }));
    expect(free.send({ type: "runReport" })).toEqual(expect.objectContaining({ type: "failed" }));
  });
});
