// B22, B25, R2706000 (DR14 part E, #2706): a dungeon run's fight in the practice driver. A config with
// `run` plays the setup's seed, decks, handicaps and the human's deck marks, the AI at the other seat;
// it brings no last board and leaves none, is never saved or kept, and once over answers `runReport`
// with what the run reads off its setup and log. The machine is Rust (`crates/shared/tests/practice.rs`
// holds the same rulings natively); these hold what the web sees of it, through the real WebAssembly module.

import { describe, expect, it } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import { AI_DIFFICULTY, HUMAN_HANDICAP } from "@jackioh/engine/config";
import type { RunFightSetup } from "@jackioh/run";

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
