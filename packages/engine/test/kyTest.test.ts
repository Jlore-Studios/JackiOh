// Classic+ #42 KY's Test's question bank (docs/classic-sets.md B5 E31, SPEC §8.7 row 42, R420, R465,
// R580), through a fixture KY's Test and a fixture bank (`fixtures/kyTest.ts`). The real card and the
// real bank are proved again in packages/cards (test/classic-plus/042-kys-test.test.ts).

import type { Action, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { defOf, queryCost } from "../src/catalog";
import { DECK_SIZE, KY_TEST_EASY_MISSES, KY_TEST_REWARDS } from "../src/config";
import { ANSWER_KEY, answerKeyOf } from "../src/prompts";
import { beginGame, legalActions } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import { createRng } from "../src/rng";
import { easyProblem } from "../src/subsystems/kyTest";
import { createGame, type GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { vanillaDeck } from "./fixtures/catalog";
import { inHand, setupCatalog } from "./fixtures/harness";
import { FIXTURE_BANK, book, coin, four, gift, kyTest, kyTestQd, kyTwo, legend, registerKyTestFixtures } from "./fixtures/kyTest";
import { act, answerKeys, board, castNow, must, openAs, roundTrip } from "./fixtures/promptHarness";

/** p1's main phase with KY's Test cast and its first prompt open. */
function offered(seed: string, radiant = false): GameState {
  const state = board(seed);
  registerKyTestFixtures();
  castNow(state, kyTest.id, "p1", radiant);
  return state;
}

/** The label the first prompt shows for a difficulty. */
function labelOf(state: GameState, difficulty: string): string {
  return must(state.pending?.options.find((option) => option.key === `mode:${difficulty}`), difficulty).label;
}

/** The first seed of `prefix-<n>` whose roll for `difficulty` is `rewardId`. */
function rolled(prefix: string, difficulty: "Easy" | "Medium" | "Hard", rewardId: string, radiant = false): GameState {
  const label = must(KY_TEST_REWARDS[difficulty].find((reward) => reward.id === rewardId), rewardId).label;
  for (let n = 0; n < 200; n += 1) {
    const state = offered(`${prefix}-${n}`, radiant);
    if (labelOf(state, difficulty) === `${difficulty}: ${label}`) return state;
  }
  throw new Error(`no seed rolls ${rewardId} for ${difficulty}`);
}

/** Answer the difficulty, then the problem rightly or wrongly. */
function take(state: GameState, difficulty: string, right: boolean): void {
  expect(answerKeys(state, `mode:${difficulty}`).error).toBeNull();
  const pending = openAs(state, "answer", "p1");
  const key = must(answerKeyOf(pending.resume.data), "the key");
  const pick = right ? key : must(["A", "B", "C", "D"].find((letter) => letter !== key), "a wrong letter");
  expect(answerKeys(state, `mode:${pick}`).error).toBeNull();
}

function handIds(state: GameState, player: PlayerId = "p1"): string[] {
  return state.players[player].hand.map((card) => card.defId);
}

describe("C+ #42 KY's Test: the Easy generator (R580)", () => {
  it("R580 a + b with both addends from 10 to 99, and three wrong sums each moved by a different miss", () => {
    for (let n = 0; n < 300; n += 1) {
      const problem = easyProblem(createRng(`easy-${n}`));
      const [a, b] = problem.id.slice("easy:".length).split("+").map(Number);
      expect(a).toBeGreaterThanOrEqual(10);
      expect(a).toBeLessThanOrEqual(99);
      expect(b).toBeGreaterThanOrEqual(10);
      expect(b).toBeLessThanOrEqual(99);
      const sum = (a ?? 0) + (b ?? 0);
      expect(problem.statement).toBe(`${a} + ${b} = ?`);
      expect(problem.answer).toBe(String(sum));
      expect(problem.options).toHaveLength(4);
      expect(new Set(problem.options).size).toBe(4);
      expect(problem.options).toContain(problem.answer);
      const misses = problem.options.filter((option) => option !== problem.answer).map((option) => Number(option) - sum);
      expect(new Set(misses).size).toBe(3);
      for (const miss of misses) expect(KY_TEST_EASY_MISSES).toContain(miss);
    }
  });
});

describe("C+ #42 KY's Test: the two prompts (R420)", () => {
  it("R420 as it resolves it rolls a reward per difficulty and offers the three, each labelled with its reward", () => {
    const before = board("kt-offer");
    registerKyTestFixtures();
    const cursor = before.rngCursor;
    castNow(before, kyTest.id);
    const pending = openAs(before, "mode", "p1");
    expect(pending.options.map((option) => option.key)).toEqual(["mode:Easy", "mode:Medium", "mode:Hard"]);
    for (const difficulty of ["Easy", "Medium", "Hard"] as const) {
      const labels = KY_TEST_REWARDS[difficulty].map((reward) => `${difficulty}: ${reward.label}`);
      expect(labels).toContain(labelOf(before, difficulty));
    }
    expect(labelOf(before, "Hard")).toBe("Hard: KY's Gift, which costs (0)");
    // R129: Easy and Medium roll one draw each; Hard's list has one entry, so nothing is drawn for it.
    expect(before.rngCursor - cursor).toBe(2);
  });

  it("R420 each reward of a list comes up over enough seeds", () => {
    for (const difficulty of ["Easy", "Medium"] as const) {
      const seen = new Set<string>();
      for (let n = 0; n < 80; n += 1) seen.add(labelOf(offered(`kt-spread-${n}`), difficulty));
      expect(seen.size).toBe(KY_TEST_REWARDS[difficulty].length);
    }
  });

  it("R420 a Medium choice asks a bank problem of that difficulty, its options shuffled, the key in the resume data", () => {
    const state = offered("kt-medium");
    answerKeys(state, "mode:Medium");
    const pending = openAs(state, "answer", "p1");
    const problem = must(FIXTURE_BANK.find((entry) => entry.statement === pending.prompt), "a bank problem");
    expect(problem.difficulty).toBe("Medium");
    expect(pending.options.map((option) => option.key)).toEqual(["mode:A", "mode:B", "mode:C", "mode:D"]);
    expect([...pending.options.map((option) => option.label)].sort()).toEqual([...problem.options].sort());
    const key = must(answerKeyOf(pending.resume.data), "the key");
    expect(pending.options.find((option) => option.key === `mode:${key}`)?.label).toBe(problem.answer);
    expect(pending.resume.data.kyTestProblem).toBe(problem.id);
  });

  it("R420 R129 Hard's bank holds one problem here: only the option shuffle draws", () => {
    const state = offered("kt-hard");
    const cursor = state.rngCursor;
    answerKeys(state, "mode:Hard");
    expect(openAs(state, "answer", "p1").prompt).toBe(FIXTURE_BANK[2]?.statement);
    expect(state.rngCursor - cursor).toBe(3); // a shuffle of four options is three draws
  });

  it("R420 an Easy choice asks a generated a + b (R580)", () => {
    const state = offered("kt-easy");
    answerKeys(state, "mode:Easy");
    const pending = openAs(state, "answer", "p1");
    expect(pending.prompt).toMatch(/^\d\d \+ \d\d = \?$/);
    expect(pending.resume.data.kyTestProblem).toMatch(/^easy:\d\d\+\d\d$/);
  });

  it("R420 a right answer gains the reward; a wrong one gives nothing", () => {
    for (const right of [true, false]) {
      const state = rolled(`kt-grade-${String(right)}`, "Easy", "coins");
      take(state, "Easy", right);
      expect(state.pending).toBeNull();
      expect(handIds(state).filter((id) => id === coin.id)).toHaveLength(right ? 3 : 0);
    }
  });

  it("R420 legalActions lists the three difficulties, then the four answers", () => {
    const state = offered("kt-legal");
    expect(legalActions(state, "p1").filter((action) => action.type === "answer")).toHaveLength(3);
    answerKeys(state, "mode:Medium");
    expect(legalActions(state, "p1").filter((action) => action.type === "answer")).toHaveLength(4);
    expect(legalActions(state, "p2").filter((action) => action.type === "answer")).toHaveLength(0);
  });

  it("R465 the key never reaches a view: the chooser sees labels, statement and options, the opponent only that a prompt is open", () => {
    const state = offered("kt-view");
    answerKeys(state, "mode:Medium");
    const pending = openAs(state, "answer", "p1");
    const mine = viewFor(state, "p1").pending;
    expect(mine?.forYou).toBe(true);
    if (mine?.forYou !== true) return;
    expect(mine.prompt).toBe(pending.prompt);
    expect(mine.options.map((option) => option.label)).toEqual(pending.options.map((option) => option.label));
    expect(viewFor(state, "p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    // Two states differing only in the key show both seats the same view.
    const twin = roundTrip(state);
    const key = must(answerKeyOf(pending.resume.data), "the key");
    must(twin.pending, "the twin's prompt").resume.data[ANSWER_KEY] = key === "A" ? "B" : "A";
    for (const viewer of ["p1", "p2"] as const) {
      expect(viewFor(twin, viewer)).toEqual(viewFor(state, viewer));
      expect(JSON.stringify(viewFor(state, viewer))).not.toContain(ANSWER_KEY);
    }
  });

  it("R420 §9.3 paused at either prompt the state survives JSON and answers to the same game", () => {
    const state = rolled("kt-json", "Medium", "books");
    const first = roundTrip(state);
    expect(hashState(first)).toBe(hashState(state));
    answerKeys(state, "mode:Medium");
    answerKeys(first, "mode:Medium");
    expect(hashState(first)).toBe(hashState(state));
    const second = roundTrip(state);
    const key = must(answerKeyOf(must(state.pending, "the problem").resume.data), "the key");
    answerKeys(state, `mode:${key}`);
    answerKeys(second, `mode:${key}`);
    expect(hashState(second)).toBe(hashState(state));
    expect(handIds(state).filter((id) => id === book.id)).toHaveLength(5);
  });

  it("R420 a game with KY's Test replays from its log to the same hash", () => {
    setupCatalog();
    registerKyTestFixtures();
    const seed = "kt-replay";
    const decks: [string[], string[]] = [[...vanillaDeck(DECK_SIZE - 1, 1), kyTestQd.id], vanillaDeck(DECK_SIZE, 21)];
    const log: Action[] = [];
    let state = beginGame(createGame({ seed, decks })).state;
    for (const player of ["p1", "p2"] as const) {
      state = act(state, { type: "mulligan", playerId: player, keep: state.players[player].hand.map((card) => card.id) }, log);
    }
    const card = must(state.players.p1.hand.find((held) => held.defId === kyTestQd.id), "KY's Test in hand");
    state = act(state, { type: "play", playerId: "p1", instanceId: card.id }, log);
    const offer = openAs(state, "mode", "p1");
    state = act(state, { type: "answer", playerId: "p1", choiceId: offer.id, selection: [{ pick: "mode", option: "Hard" }] }, log);
    const problem = openAs(state, "answer", "p1");
    const key = must(answerKeyOf(problem.resume.data), "the key");
    state = act(state, { type: "answer", playerId: "p1", choiceId: problem.id, selection: [{ pick: "mode", option: key }] }, log);
    expect(handIds(state)).toContain(gift.id);
    const folded = fold({ seed, decks, log });
    expect(folded.errors).toEqual([]);
    expect(hashState(folded.state)).toBe(hashState(state));
  });
});

describe("C+ #42 KY's Test: the rewards (R420)", () => {
  it("R420 Easy: 3 The Coins", () => {
    const state = rolled("kt-coins", "Easy", "coins");
    take(state, "Easy", true);
    expect(handIds(state)).toEqual([coin.id, coin.id, coin.id]);
  });

  it("R420 Easy: a random (2) Cost KY card", () => {
    const state = rolled("kt-ky", "Easy", "ky");
    take(state, "Easy", true);
    expect(handIds(state)).toEqual([kyTwo.id]);
  });

  it("R420 R387 Easy: a random Legendary card, which costs (0), never KY's Test itself", () => {
    const state = rolled("kt-legend", "Easy", "legendary");
    take(state, "Easy", true);
    expect(handIds(state)).toEqual([legend.id]);
    expect(state.players.p1.hand[0]?.costOverride).toBe(0);
  });

  it("R420 Easy: 2 random Books", () => {
    const state = rolled("kt-books2", "Easy", "books");
    take(state, "Easy", true);
    expect(handIds(state)).toEqual([book.id, book.id]);
  });

  it("R420 Medium: 2 random (4) Cost cards, which cost (1)", () => {
    const state = rolled("kt-fours", "Medium", "fours");
    take(state, "Medium", true);
    // Every (4) Cost fixture of the catalog is in the pool; each one drawn is priced (1).
    expect(state.players.p1.hand.map((card) => queryCost(defOf(state, card.defId)))).toEqual([4, 4]);
    expect(state.players.p1.hand.map((card) => card.costOverride)).toEqual([1, 1]);
  });

  it("R420 R387 Medium: 5 random KY cards, never KY's Test itself", () => {
    const state = rolled("kt-ky5", "Medium", "ky");
    take(state, "Medium", true);
    expect(handIds(state)).toEqual(Array.from({ length: 5 }, () => kyTwo.id));
  });

  it("R420 Medium: fill your hand with random Books, up to the hand cap", () => {
    const state = rolled("kt-fill", "Medium", "fill");
    take(state, "Medium", true);
    expect(handIds(state)).toEqual(Array.from({ length: 10 }, () => book.id));
  });

  it("R420 R129 Medium: a full hand has no room to fill, and nothing is drawn for it", () => {
    const state = rolled("kt-fill-full", "Medium", "fill");
    answerKeys(state, "mode:Medium");
    const pending = openAs(state, "answer", "p1");
    const key = must(answerKeyOf(pending.resume.data), "the key");
    inHand(state, four.id, "p1", 10);
    const cursor = state.rngCursor;
    answerKeys(state, `mode:${key}`);
    expect(state.rngCursor).toBe(cursor);
    expect(handIds(state)).toEqual(Array.from({ length: 10 }, () => four.id));
  });

  it("R420 Hard: KY's Gift, which costs (0)", () => {
    const state = offered("kt-gift");
    take(state, "Hard", true);
    expect(handIds(state)).toEqual([gift.id]);
    expect(state.players.p1.hand[0]?.costOverride).toBe(0);
  });

  it("R420 the Radiant face's reward cards are Radiant", () => {
    const state = offered("kt-radiant", true);
    take(state, "Hard", true);
    expect(state.players.p1.hand.map((card) => card.radiant)).toEqual([true]);
    const coins = rolled("kt-radiant-coins", "Easy", "coins", true);
    take(coins, "Easy", true);
    expect(coins.players.p1.hand.map((card) => card.radiant)).toEqual([true, true, true]);
  });
});
