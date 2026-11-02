// B12: every run binding answers JSON in and out, a refusal as `{ error }`, through the typed
// functions in `./index.ts`, in both browser modules: create a run from `runContent([])` and a
// starter of `starterAccess`, reduce a `go`, and get a refusal for an illegal action.

import { afterEach, describe, expect, it } from "vitest";

import type { RunAction, RunFile, RunSetup, RunUnlocks } from "@jackioh/run";

import { luauBytes } from "../test/luau.ts";
import {
  runCheckSetup,
  runConstants,
  runContent,
  runCreate,
  runDailySeed,
  runEmbers,
  runFightReport,
  runFightSetup,
  runFold,
  runLevelCost,
  runMaxHp,
  runReduce,
  runScore,
  runSeed,
  useLuauModule,
  type ReplayInput,
} from "./index.ts";

afterEach(async () => {
  await useLuauModule(false);
});

/** Every typed run function, once, against whichever module is in use. */
function exerciseRunSurface(): void {
  const content = runContent([]);
  expect(content.pool.length).toBeGreaterThan(0);
  const constants = runConstants();
  expect(constants.starterSize).toBeGreaterThan(0);

  const starter = content.starterAccess.slice(0, constants.starterSize);
  expect(starter).toHaveLength(constants.starterSize);
  const unlocks: RunUnlocks = {
    level: constants.slotLadder.length,
    cards: [],
    artifacts: [],
    owned: null,
  };
  const setup: RunSetup = {
    seed: runDailySeed("2026-11-01", 1),
    starter,
    artifacts: [],
    curses: [],
  };
  expect(runCheckSetup(setup, unlocks, content)).toBeNull();
  expect(runCheckSetup({ ...setup, seed: "bad" }, unlocks, content)).toMatch(/capital letters/);

  const created = runCreate(setup, unlocks, content);
  if ("error" in created) throw new Error(created.error);
  expect(runCheckSetup({ ...setup, seed: "bad" }, unlocks, content)).not.toBeNull();

  const illegal = runReduce(created.state, content, { type: "heal" });
  if (!("error" in illegal)) throw new Error("a heal on the map is refused");
  expect(illegal.error.length).toBeGreaterThan(0);

  const go: RunAction = { type: "go", col: 0 };
  const moved = runReduce(created.state, content, go);
  if ("error" in moved) throw new Error(moved.error);

  expect(runFightSetup(created.state, content)).toBeNull();
  const fight = runFightSetup(moved.state, content);
  if (fight === null) throw new Error("a go onto the first row starts a fight");
  expect(fight.seed).toBe(`${setup.seed}-0-0`);
  expect(fight.humanDeck).toHaveLength(starter.length);

  const game: ReplayInput = {
    seed: fight.seed,
    decks: [fight.humanDeck, fight.enemyDeck],
    handicaps: { p1: fight.humanHandicap, p2: fight.enemyHandicap },
    deckMarks: { p1: fight.humanMarks },
    log: [],
  };
  const report = runFightReport(game, "p1");
  if (!("error" in report)) throw new Error("an unfinished fight reports nothing");
  expect(report.error.length).toBeGreaterThan(0);

  const file: RunFile = { setup, unlocks, log: [go] };
  const folded = runFold(file, content);
  if ("error" in folded) throw new Error(folded.error);
  expect(folded.state).toEqual(moved.state);

  expect(runScore(moved.state, content)).toEqual(expect.any(Number));
  expect(runEmbers(runScore(moved.state, content))).toEqual(expect.any(Number));
  expect(runMaxHp(moved.state, content)).toBeGreaterThan(0);
  expect(runLevelCost(1)).not.toBeNull();
  expect(runLevelCost(constants.slotLadder.length)).toBeNull();
  expect(runSeed("anything")).toMatch(/^[A-Z]{6}$/);
  expect(runDailySeed("2026-11-01", 2)).toMatch(/^[A-Z]{6}$/);
}

describe("B12 the run bindings", () => {
  it("B12 every run binding crosses the normal module as JSON and returns refusals", () => {
    exerciseRunSurface();
  });

  it("B12 every run binding crosses the Luau module as JSON and returns refusals", async () => {
    await useLuauModule(true, luauBytes());
    exerciseRunSurface();
  });
});
