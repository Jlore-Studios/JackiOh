// C+ #42 KY's Test — SPEC §8.7 row 42, R420, R465, R580, BUILD M9 row C+ 42. The machinery is the
// engine's question bank (`subsystems/kyTest.ts`, proved with a fixture bank in
// packages/engine/test/kyTest.test.ts); this file proves the real card, the real bank and the real
// reward pools again.

import {
  HAND_CAP,
  KY_TEST_MIN_PROBLEMS,
  KY_TEST_OPTIONS,
  KY_TEST_REWARDS,
  answerKeyOf,
  beginGame,
  createGame,
  fold,
  hashState,
  legalActions,
  queryCost,
  reduce,
  type GameState,
} from "@jackioh/engine";
import type { Action, ActionInput, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/index";
import { KY_TEST_BANK } from "../../src/kyTestBank";
import { base, def, radiant } from "../../src/scripts/classic-plus/042-kys-test";
import { scenario, type Scenario } from "../_harness";

const TEST = "classicplus-042";
const GIFT = "classicplus-042-1";
const COIN = "core-t-coin";
const CONJURE_KY = "core-057";
const FILLER = "core-005";

type Difficulty = "Easy" | "Medium" | "Hard";

/** KY's Test cast from p1's hand, its first prompt open. */
function cast(seed: string, radiantFace = false, hand: readonly string[] = [FILLER]): Scenario {
  const s = scenario({ seed, p1: { hand: [{ def: TEST, radiant: radiantFace }, ...hand] }, p2: { hand: [FILLER] } });
  return s.play(TEST);
}

function labelOf(s: Scenario, difficulty: Difficulty): string {
  return s.state.pending?.options.find((option) => option.key === `mode:${difficulty}`)?.label ?? "";
}

/** The first seed whose roll for `difficulty` is `rewardId`. */
function rolled(difficulty: Difficulty, rewardId: string, radiantFace = false, hand: readonly string[] = [FILLER]): Scenario {
  const reward = KY_TEST_REWARDS[difficulty].find((entry) => entry.id === rewardId);
  for (let n = 0; n < 200; n += 1) {
    const s = cast(`kys-test-${difficulty}-${rewardId}-${n}`, radiantFace, hand);
    if (labelOf(s, difficulty) === `${difficulty}: ${reward?.label}`) return s;
  }
  throw new Error(`no seed rolls ${rewardId}`);
}

function key(s: Scenario): string {
  const found = answerKeyOf(s.state.pending?.resume.data ?? {});
  if (found === null) throw new Error("an answer prompt with its key");
  return found;
}

/** Choose the difficulty, then answer rightly or wrongly. */
function take(s: Scenario, difficulty: Difficulty, right = true): Scenario {
  s.answer(difficulty);
  const correct = key(s);
  return s.answer(`mode:${right ? correct : ["A", "B", "C", "D"].find((letter) => letter !== correct)}`);
}

/** The cards the reward put in p1's hand (the filler excluded). */
function gained(s: Scenario): { defId: string; radiant: boolean; costOverride?: number }[] {
  return s
    .hand("p1")
    .filter((card) => card.defId !== FILLER)
    .map((card) => ({ defId: card.defId, radiant: card.radiant, ...(card.costOverride === undefined ? {} : { costOverride: card.costOverride }) }));
}

describe("C+ #42 KY's Test", () => {
  it("is the engine's question bank on both faces, over the real bank", () => {
    expect(def.id).toBe(TEST);
    expect(radiant).toBe(base);
    expect(base.cry).toBeTypeOf("function");
    expect(Object.keys(base.resume ?? {}).sort()).toEqual(["ask", "grade"]);
  });

  describe("the bank (R420)", () => {
    it(`R420 holds at least KY_TEST_MIN_PROBLEMS (${KY_TEST_MIN_PROBLEMS}) Medium and Hard problems, Easy ones being generated (R580)`, () => {
      for (const difficulty of ["Medium", "Hard"] as const) {
        expect(KY_TEST_BANK.filter((problem) => problem.difficulty === difficulty).length).toBeGreaterThanOrEqual(KY_TEST_MIN_PROBLEMS);
      }
      expect(KY_TEST_BANK.some((problem) => problem.difficulty === "Easy")).toBe(false);
    });

    it("R420 every id is unique; every problem has four distinct options, one of them its answer", () => {
      expect(new Set(KY_TEST_BANK.map((problem) => problem.id)).size).toBe(KY_TEST_BANK.length);
      for (const problem of KY_TEST_BANK) {
        expect(problem.options, problem.id).toHaveLength(KY_TEST_OPTIONS);
        expect(new Set(problem.options).size, problem.id).toBe(KY_TEST_OPTIONS);
        expect(problem.options, problem.id).toContain(problem.answer);
        expect(problem.statement.length, problem.id).toBeGreaterThan(0);
      }
    });
  });

  describe("the prompts (R420)", () => {
    it("R420 as it resolves it offers Easy, Medium and Hard, each labelled with the reward rolled from its list", () => {
      const s = cast("kys-test-offer");
      const pending = s.state.pending;
      expect(pending?.kind).toBe("mode");
      expect(pending?.playerId).toBe("p1");
      expect(pending?.options.map((option) => option.key)).toEqual(["mode:Easy", "mode:Medium", "mode:Hard"]);
      for (const difficulty of ["Easy", "Medium"] as const) {
        expect(KY_TEST_REWARDS[difficulty].map((reward) => `${difficulty}: ${reward.label}`)).toContain(labelOf(s, difficulty));
      }
      expect(labelOf(s, "Hard")).toBe("Hard: KY's Gift, which costs (0)");
    });

    it("R420 every label is the reward as the card's text prints it", () => {
      const text = cardDef(TEST).base.text;
      for (const difficulty of ["Easy", "Medium", "Hard"] as const) {
        for (const reward of KY_TEST_REWARDS[difficulty]) expect(text).toContain(reward.label);
      }
    });

    it("R420 the chosen difficulty's problem opens as an answer prompt: the statement and four options under letters", () => {
      const s = cast("kys-test-medium").answer("Medium");
      const pending = s.state.pending;
      expect(pending?.kind).toBe("answer");
      const problem = KY_TEST_BANK.find((entry) => entry.statement === pending?.prompt);
      expect(problem?.difficulty).toBe("Medium");
      expect(pending?.options.map((option) => option.key)).toEqual(["mode:A", "mode:B", "mode:C", "mode:D"]);
      expect([...(pending?.options.map((option) => option.label) ?? [])].sort()).toEqual([...(problem?.options ?? [])].sort());
      expect(pending?.options.find((option) => option.key === `mode:${key(s)}`)?.label).toBe(problem?.answer);
    });

    it("R420 a Hard choice asks a Hard problem of the bank", () => {
      const s = cast("kys-test-hard").answer("Hard");
      expect(KY_TEST_BANK.find((entry) => entry.statement === s.state.pending?.prompt)?.difficulty).toBe("Hard");
    });

    it("R580 an Easy choice asks a generated a + b with both addends from 10 to 99", () => {
      const s = cast("kys-test-easy").answer("Easy");
      const match = /^(\d+) \+ (\d+) = \?$/.exec(s.state.pending?.prompt ?? "");
      expect(match).not.toBeNull();
      const [a, b] = [Number(match?.[1]), Number(match?.[2])];
      for (const addend of [a, b]) expect(addend >= 10 && addend <= 99).toBe(true);
      expect(s.state.pending?.options.find((option) => option.key === `mode:${key(s)}`)?.label).toBe(String(a + b));
    });

    it("R420 legalActions lists the three difficulties, then all four answers, so the AI and the fuzz reach every one", () => {
      const s = cast("kys-test-legal");
      expect(legalActions(s.state, "p1").filter((action) => action.type === "answer")).toHaveLength(3);
      s.answer("Medium");
      expect(legalActions(s.state, "p1").filter((action) => action.type === "answer")).toHaveLength(4);
      expect(legalActions(s.state, "p2").filter((action) => action.type === "answer")).toEqual([]);
    });

    it("R420 a wrong answer gives nothing, and the Spell was still played", () => {
      const s = rolled("Easy", "coins");
      take(s, "Easy", false);
      expect(s.state.pending).toBeNull();
      expect(gained(s)).toEqual([]);
      s.expectInZone(TEST, "graveyard").expectEvents("cardPlayed", "cardResolved");
    });
  });

  describe("hidden information (R97, R177, R465)", () => {
    it("R465 the key never reaches a view: the chooser sees the labels, the statement and the options; the opponent only that a prompt is open", () => {
      const s = cast("kys-test-view");
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      const offer = s.view("p1").pending;
      expect(offer?.forYou === true ? offer.options.map((option) => option.label) : []).toEqual(
        s.state.pending?.options.map((option) => option.label),
      );
      s.answer("Hard");
      const mine = s.view("p1").pending;
      expect(mine?.forYou === true ? mine.prompt : "").toBe(s.state.pending?.prompt);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      for (const viewer of ["p1", "p2"] as const) {
        expect(JSON.stringify(s.view(viewer))).not.toContain("__answerKey");
        expect(JSON.stringify(s.view(viewer))).not.toContain("kyTestProblem");
      }
    });

    it("R97 the reward's cards reach p1's hand under the sentinel for the opponent", () => {
      const s = cast("kys-test-hidden");
      take(s, "Hard");
      const added = (events: GameEvent[]) => events.filter((event) => event.type === "addedToHand");
      expect(added(s.view("p1").events).map((event) => (event.type === "addedToHand" ? event.defId : ""))).toEqual([GIFT]);
      expect(added(s.view("p2").events).map((event) => (event.type === "addedToHand" ? event.defId : ""))).toEqual(["hidden"]);
    });
  });

  describe("the rewards (R420): pools of non-token cards of every set, never KY's Test (R387)", () => {
    it("R420 Easy: 3 The Coins", () => {
      const s = take(rolled("Easy", "coins"), "Easy");
      expect(gained(s)).toEqual([COIN, COIN, COIN].map((defId) => ({ defId, radiant: false })));
    });

    it("R420 Easy: a random (2) Cost KY card, which is Conjure KY", () => {
      const s = take(rolled("Easy", "ky"), "Easy");
      expect(gained(s)).toEqual([{ defId: CONJURE_KY, radiant: false }]);
    });

    it("R420 R387 Easy: a random Legendary card, which costs (0), never KY's Test", () => {
      for (let n = 0; n < 4; n += 1) {
        const s = take(rolled("Easy", "legendary", false, [FILLER, ...Array.from({ length: n }, () => FILLER)]), "Easy");
        const [card] = gained(s);
        expect(cardDef(card?.defId ?? "").rarity).toBe("Legendary");
        expect(cardDef(card?.defId ?? "").token).toBe(false);
        expect(card?.defId).not.toBe(TEST);
        expect(card?.costOverride).toBe(0);
      }
    });

    it("R420 Easy: 2 random Books", () => {
      const s = take(rolled("Easy", "books"), "Easy");
      const cards = gained(s);
      expect(cards).toHaveLength(2);
      for (const card of cards) expect(cardDef(card.defId).tags).toContain("Book");
    });

    it("R420 Medium: 2 random (4) Cost cards, which cost (1)", () => {
      const s = take(rolled("Medium", "fours"), "Medium");
      const cards = gained(s);
      expect(cards).toHaveLength(2);
      for (const card of cards) {
        expect(queryCost(cardDef(card.defId))).toBe(4);
        expect(cardDef(card.defId).token).toBe(false);
        expect(card.costOverride).toBe(1);
      }
    });

    it("R420 Medium: 5 random Books", () => {
      const cards = gained(take(rolled("Medium", "books"), "Medium"));
      expect(cards).toHaveLength(5);
      for (const card of cards) expect(cardDef(card.defId).tags).toContain("Book");
    });

    it("R420 R387 Medium: 5 random KY cards, never KY's Test and never a token", () => {
      const cards = gained(take(rolled("Medium", "ky"), "Medium"));
      expect(cards).toHaveLength(5);
      for (const card of cards) {
        expect(cardDef(card.defId).tags).toContain("KY");
        expect(cardDef(card.defId).token).toBe(false);
        expect(card.defId).not.toBe(TEST);
      }
    });

    it("R420 Medium: fill your hand with random Books, until it holds 10", () => {
      const s = take(rolled("Medium", "fill"), "Medium");
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      for (const card of gained(s)) expect(cardDef(card.defId).tags).toContain("Book");
    });

    it("R420 Hard: KY's Gift, which costs (0)", () => {
      const s = take(cast("kys-test-gift"), "Hard");
      expect(gained(s)).toEqual([{ defId: GIFT, radiant: false, costOverride: 0 }]);
    });

    it("§2.4 a full hand burns the reward cards that do not fit", () => {
      const hand = Array.from({ length: 8 }, () => FILLER);
      const s = take(rolled("Easy", "coins", false, hand), "Easy");
      // 8 fillers in hand: two Coins fit, the third is burned.
      expect(s.hand("p1")).toHaveLength(HAND_CAP);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("R420 radiant: every reward card is Radiant", () => {
      expect(gained(take(cast("kys-test-radiant-gift", true), "Hard"))).toEqual([{ defId: GIFT, radiant: true, costOverride: 0 }]);
      expect(gained(take(rolled("Easy", "coins", true), "Easy")).map((card) => card.radiant)).toEqual([true, true, true]);
      const books = gained(take(rolled("Medium", "books", true), "Medium"));
      expect(books.map((card) => card.radiant)).toEqual([true, true, true, true, true]);
    });
  });

  describe("pauses and replay (§9.3)", () => {
    it("R420 paused at either prompt, the state survives JSON, key included, and answers to the same hash", () => {
      const s = rolled("Medium", "ky");
      const nonce = { n: 0 };
      const answer = (state: GameState, option: string): GameState => {
        nonce.n += 1;
        const action = {
          type: "answer",
          playerId: "p1",
          choiceId: state.pending?.id ?? "",
          selection: [{ pick: "mode", option }],
          nonce: `kys-json-${nonce.n}`,
        } as Action;
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(result.error);
        return result.state;
      };
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(hashState(thawed)).toBe(hashState(s.state));
      const live = answer(s.state, "Medium");
      const frozen = answer(thawed, "Medium");
      expect(hashState(frozen)).toBe(hashState(live));
      const right = answerKeyOf(live.pending?.resume.data ?? {}) ?? "";
      const again = JSON.parse(JSON.stringify(live)) as GameState;
      expect(answerKeyOf(again.pending?.resume.data ?? {})).toBe(right);
      expect(hashState(answer(again, right))).toBe(hashState(answer(live, right)));
    });

    it("R420 R79 a timeout answers the open problem, as the turn clock does any prompt", () => {
      const s = cast("kys-test-timeout").answer("Easy");
      const result = reduce(s.state, { type: "timeout", playerId: "p1", nonce: "kys-timeout" } as Action);
      expect(result.error).toBeUndefined();
      expect(result.state.pending).toBeNull();
      expect(result.state.players.p1.graveyard.some((card) => card.defId === TEST)).toBe(true);
    });

    it("R420 a game with KY's Test replays from its log to the same hash", () => {
      const deck = (first: string): string[] => [
        first,
        ...["core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008", "core-009", "core-010"],
        ...["core-011", "core-012", "core-013", "core-014", "core-016", "core-017", "core-019", "core-020", "core-025"],
      ];
      const decks: [string[], string[]] = [deck(TEST), deck("core-026")];
      let found: { seed: string; state: GameState } | null = null;
      for (let n = 0; n < 300 && found === null; n += 1) {
        const seed = `kys-test-replay-${n}`;
        const begun = beginGame(createGame({ seed, decks })).state;
        if (begun.players.p1.hand.some((card) => card.defId === TEST)) found = { seed, state: begun };
      }
      if (found === null) throw new Error("no seed deals KY's Test to p1");
      const log: Action[] = [];
      const act = (state: GameState, body: ActionInput): GameState => {
        const action = { ...body, nonce: `kys-replay-${log.length}` } as Action;
        const result = reduce(state, action);
        if (result.error !== undefined) throw new Error(result.error);
        log.push(action);
        return result.state;
      };
      let state = found.state;
      for (const player of ["p1", "p2"] as const) {
        state = act(state, { type: "mulligan", playerId: player, keep: state.players[player].hand.map((card) => card.id) } as ActionInput);
      }
      const card = state.players.p1.hand.find((held) => held.defId === TEST);
      state = act(state, { type: "play", playerId: "p1", instanceId: card?.id ?? "" } as ActionInput);
      state = act(state, { type: "answer", playerId: "p1", choiceId: state.pending?.id ?? "", selection: [{ pick: "mode", option: "Hard" }] } as ActionInput);
      const right = answerKeyOf(state.pending?.resume.data ?? {}) ?? "";
      state = act(state, { type: "answer", playerId: "p1", choiceId: state.pending?.id ?? "", selection: [{ pick: "mode", option: right }] } as ActionInput);
      expect(state.players.p1.hand.some((held) => held.defId === GIFT)).toBe(true);
      const replayed = fold({ seed: found.seed, decks, log });
      expect(replayed.errors).toEqual([]);
      expect(hashState(replayed.state)).toBe(hashState(state));
    });
  });
});
