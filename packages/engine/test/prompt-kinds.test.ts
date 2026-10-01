// Patch v0.2.0's prompt kinds (docs/classic-sets.md B5 E17, E18): `number`, `answer`, `cell`,
// `reward`, `pick`, a `mode` prompt the other player holds, and the other player's hand as a prompt's
// options. For each one this file proves what the brief asks of a prompt: what `promptAnswers` lists,
// what `whyAnswerRefused` refuses, what `viewFor` shows the chooser and — R97, R177 — that the other
// seat learns only that a prompt is open and whose it is; that a timeout answers it with R79's AI
// policy; that a paused state survives `JSON.parse(JSON.stringify(...))` and answers exactly as the
// original does; and that a game through it replays from its log (§9.2, §9.3).

import type { PendingView, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { HERO_HEALTH } from "../src/config";
import { ANSWER_OPTION_IDS, chooseFromHand } from "../src/effects";
import { makeContext } from "../src/resolve";
import { ANSWER_KEY, MAX_PROMPT_ANSWERS, answerKeyOf, promptAnswers, whyAnswerRefused } from "../src/prompts";
import { legalActions } from "../src/reduce";
import { hashState } from "../src/replay";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { plain } from "./fixtures/combat";
import { eventsOfType, inHand, put, setLibrary, slot } from "./fixtures/harness";
import {
  BACK_BUDGET,
  GLITCH_OPTIONS,
  PICKLE_OPTIONS,
  QUEST_REWARDS,
  QUIZ,
  acquire,
  backFromGy,
  crossPick,
  glitch,
  grunt,
  mill,
  mindMelt,
  numberer,
  papaya,
  pickle,
  prize,
  quest,
  quickdrawOf,
  quiz,
} from "./fixtures/prompts";
import {
  act,
  answerKeys,
  board,
  castNow,
  expectReplays,
  handCard,
  must,
  openAs,
  replayable,
  roundTrip,
} from "./fixtures/promptHarness";
import { draw } from "../src/draw";
import { createRng } from "../src/rng";
import { sinkFor } from "./fixtures/harness";
import { settle } from "../src/triggers";

/**
 * R81, R97: the other seat's view of a prompt: that it is open, and whose — the whole of `pending`,
 * so no option, caption, cost or budget travels — and a `promptOpened` event that names the player
 * and the kind and nothing more.
 */
function expectOnlyThatItIsOpen(state: GameState, viewer: PlayerId, holder: PlayerId): void {
  const view = viewFor(state, viewer);
  expect(view.pending).toEqual({ forYou: false, pendingFor: holder });
  for (const opened of eventsOfType(view.events, "promptOpened")) {
    expect(Object.keys(opened).sort()).toEqual(["choiceId", "kind", "player", "type"]);
  }
}

function forYou(view: PendingView | null): Extract<PendingView, { forYou: true }> {
  if (view === null || !view.forYou) throw new Error("expected the viewer's own prompt");
  return view;
}

function graveCard(state: GameState, player: PlayerId, defId: string, cost?: number): CardInstance {
  const card = newInstance(state, defId, player, { z: "graveyard", player });
  if (cost !== undefined) card.costOverride = cost;
  state.players[player].graveyard.push(card);
  return card;
}

describe("E18: a mode prompt the other player holds (Classic #8)", () => {
  it("E18 the other player answers, and the step runs as the asking card's controller", () => {
    const state = board("pickle-owner");
    inHand(state, plain.id, "p2", 2);
    setLibrary(state, "p1", [plain.id, plain.id]);
    setLibrary(state, "p2", [plain.id, plain.id, plain.id]);
    castNow(state, pickle.id, "p1");

    const first = openAs(state, "mode", "p2");
    expect(first.options.map((option) => option.key)).toEqual(PICKLE_OPTIONS.map((option) => `mode:${option}`));
    expect(promptAnswers(first).map((answer) => answer.selection)).toEqual(
      PICKLE_OPTIONS.map((option) => [{ pick: "mode", option }]),
    );
    // The card's controller may not answer the other player's prompt.
    expect(
      whyAnswerRefused(first, { playerId: "p1", choiceId: first.id, selection: [{ pick: "mode", option: "draw" }] }),
    ).toBe("that prompt belongs to the other player");
    expectOnlyThatItIsOpen(state, "p1", "p2");
    expect(forYou(viewFor(state, "p2").pending).options.map((option) => option.label)).toEqual([...PICKLE_OPTIONS]);

    // "You draw" is the asking card's controller's draw, though the other player answered.
    expect(answerKeys(state, "mode:draw").error).toBeNull();
    expect(state.players.p1.hand).toHaveLength(1);
    expect(state.players.p2.hand).toHaveLength(2);

    openAs(state, "mode", "p2");
    answerKeys(state, "mode:exile");
    expect(state.players.p2.library).toHaveLength(2);
    expect(state.players.p2.exile).toHaveLength(1);

    // "They discard": their own hand pick, held by them, continued as the Pickle's controller.
    openAs(state, "mode", "p2");
    answerKeys(state, "mode:discard");
    const pick = openAs(state, "hand", "p2");
    expect(pick.options.map((option) => option.selection)).toEqual(
      state.players.p2.hand.map((card) => ({ pick: "instance", instanceId: card.id })),
    );
    expectOnlyThatItIsOpen(state, "p1", "p2");
    const kept = state.players.p2.hand[0] as CardInstance;
    const gone = state.players.p2.hand[1] as CardInstance;
    answerKeys(state, `instance:${gone.id}`);
    expect(state.players.p2.hand.map((card) => card.id)).toEqual([kept.id]);
    expect(state.players.p2.graveyard.map((card) => card.id)).toEqual([gone.id]);
    // Three choices, then no more.
    expect(state.pending).toBeNull();
  });

  it("E18 a paused other-player prompt survives a JSON round trip and answers the same", () => {
    const state = board("pickle-json");
    inHand(state, plain.id, "p2", 2);
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    setLibrary(state, "p2", [plain.id, plain.id]);
    castNow(state, pickle.id, "p1");
    const copy = roundTrip(state);
    expect(hashState(copy)).toBe(hashState(state));
    for (const key of ["mode:draw", "mode:discard"]) {
      answerKeys(state, key);
      answerKeys(copy, key);
    }
    expect(hashState(copy)).toBe(hashState(state));
    expect(state.pending?.kind).toBe("hand");
  });

  it("E18 R79 the other player's timeout answers their prompt with the AI policy, and nothing more", () => {
    let state = board("pickle-timeout");
    inHand(state, plain.id, "p2", 1);
    setLibrary(state, "p1", [plain.id, plain.id, plain.id]);
    setLibrary(state, "p2", [plain.id, plain.id]);
    castNow(state, pickle.id, "p1");
    const before = must(state.pending, "Pickle's first choice").id;
    state = act(state, { type: "timeout", playerId: "p2" });
    // It answered that one prompt: the next choice is open, still theirs, and the turn is still p1's.
    expect(state.pending?.id).not.toBe(before);
    expect(state.pending?.playerId).toBe("p2");
    expect(state.active).toBe("p1");
    // The active player's clock cannot answer the other player's prompt.
    const held = must(state.pending, "the second choice").id;
    state = act(state, { type: "timeout", playerId: "p1" });
    expect(state.pending?.id).toBe(held);
  });

  it("E18 a Pickle game replays from its log", () => {
    const qd = quickdrawOf(pickle).id;
    const { state: dealt, log, decks } = replayable("pickle-replay", [qd]);
    let state = dealt;
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", qd).id }, log);
    for (const option of ["draw", "exile", "discard"]) {
      const pending = openAs(state, "mode", "p2");
      state = act(state, { type: "answer", playerId: "p2", choiceId: pending.id, selection: [{ pick: "mode", option }] }, log);
    }
    const discard = openAs(state, "hand", "p2");
    state = act(state, { type: "answer", playerId: "p2", choiceId: discard.id, selection: [at0(discard)] }, log);
    expect(state.pending).toBeNull();
    expectReplays("pickle-replay", decks, log, state);
  });
});

function at0(pending: { options: { selection: import("@jackioh/shared").Selection }[] }): import("@jackioh/shared").Selection {
  return must(pending.options[0], "a first option").selection;
}

describe("E18: the number kind", () => {
  it("E18 a number declared with the play (R81) travels in the action, from a fixed list", () => {
    const state = board("glitch");
    const card = inHand(state, glitch.id, "p1")[0] as CardInstance;
    const plays = legalActions(state, "p1").filter((action) => action.type === "play" && action.instanceId === card.id);
    expect(plays.map((play) => (play.type === "play" ? play.modes : []))).toEqual(GLITCH_OPTIONS.map((n) => [n]));
    expect(() => act(state, { type: "play", playerId: "p1", instanceId: card.id, modes: ["11"] })).toThrow();
    const next = act(state, { type: "play", playerId: "p1", instanceId: card.id, modes: ["7"] });
    expect(next.players.p2.hero.health).toBe(HERO_HEALTH - 7);
  });

  it("E18 a number asked at resolution offers the range, and the answer is the number", () => {
    const state = board("numberer");
    castNow(state, numberer.id);
    const pending = openAs(state, "number", "p1");
    expect(pending.options.map((option) => option.key)).toEqual(["mode:1", "mode:2", "mode:3"]);
    expect(promptAnswers(pending)).toHaveLength(3);
    expect(forYou(viewFor(state, "p1").pending).kind).toBe("number");
    expectOnlyThatItIsOpen(state, "p2", "p1");
    const copy = roundTrip(state);
    answerKeys(state, "mode:3");
    answerKeys(copy, "mode:3");
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 3);
    expect(hashState(copy)).toBe(hashState(state));
  });

  it("E18 R79 a timeout answers a number prompt with one of its numbers", () => {
    let state = board("numberer-timeout");
    castNow(state, numberer.id);
    state = act(state, { type: "timeout", playerId: "p1" });
    expect(state.pending).toBeNull();
    const lost = HERO_HEALTH - state.players.p2.hero.health;
    expect([1, 2, 3]).toContain(lost);
  });
});

describe("E18: the answer kind (Classic+ #42, R465)", () => {
  it("R465 an answer prompt's key stays in its resume data and never reaches viewFor", () => {
    const state = board("quiz-key");
    castNow(state, quiz.id);
    const pending = openAs(state, "answer", "p1");
    expect(pending.prompt).toBe(QUIZ.statement);
    // Four options under letters, in an order the match rng shuffled; the labels are the answers.
    expect(pending.options.map((option) => option.key)).toEqual(ANSWER_OPTION_IDS.slice(0, 4).map((id) => `mode:${id}`));
    expect([...pending.options.map((option) => option.label)].sort()).toEqual([...QUIZ.options].sort());
    const key = must(answerKeyOf(pending.resume.data), "the key");
    const right = must(pending.options.find((option) => option.label === QUIZ.options[QUIZ.correct]), "the right option");
    expect(right.key).toBe(`mode:${key}`);

    // The view is the same whichever option is right: nothing in it tells them apart.
    const twin = roundTrip(state);
    const other = must(ANSWER_OPTION_IDS.slice(0, 4).find((id) => id !== key), "another letter");
    must(twin.pending, "the twin's prompt").resume.data[ANSWER_KEY] = other;
    for (const viewer of ["p1", "p2"] as const) {
      expect(viewFor(twin, viewer)).toEqual(viewFor(state, viewer));
      expect(JSON.stringify(viewFor(state, viewer))).not.toContain(ANSWER_KEY);
    }
    expect(forYou(viewFor(state, "p1").pending).options.map((option) => option.label)).toEqual(
      pending.options.map((option) => option.label),
    );
    expectOnlyThatItIsOpen(state, "p2", "p1");
    expect(promptAnswers(pending)).toHaveLength(4);
  });

  it("R465 the right answer gains the reward, a wrong one nothing, and the list's tail runs either way", () => {
    for (const right of [true, false]) {
      const state = board(`quiz-${String(right)}`);
      castNow(state, quiz.id);
      const pending = openAs(state, "answer", "p1");
      const key = must(answerKeyOf(pending.resume.data), "the key");
      const pick = right ? key : must(ANSWER_OPTION_IDS.slice(0, 4).find((id) => id !== key), "a wrong letter");
      const copy = roundTrip(state);
      answerKeys(state, `mode:${pick}`);
      answerKeys(copy, `mode:${pick}`);
      expect(state.players.p1.hand.some((card) => card.defId === prize.id)).toBe(right);
      // The heal after the question ran after the answer (R113), right or wrong.
      expect(state.players.p1.hero.health).toBe(HERO_HEALTH + 1);
      expect(hashState(copy)).toBe(hashState(state));
    }
  });

  it("R465 R79 a timeout answers the problem from what it shows, and the key judges it as anyone's", () => {
    for (const seed of ["quiz-timeout-1", "quiz-timeout-2", "quiz-timeout-3", "quiz-timeout-4"]) {
      let state = board(seed);
      castNow(state, quiz.id);
      const pending = must(state.pending, "the problem");
      const key = must(answerKeyOf(pending.resume.data), "the key");
      const answers = legalActions(state, "p1").filter((action) => action.type === "answer");
      expect(answers).toHaveLength(4);
      // R79: the policy draws uniformly over the answers, from the match rng at its stored cursor.
      const drawn = answers[createRng(state.seed, state.rngCursor).int(answers.length)];
      const chosen = drawn?.type === "answer" ? drawn.selection[0] : undefined;
      state = act(state, { type: "timeout", playerId: "p1" });
      expect(state.pending).toBeNull();
      const right = chosen?.pick === "mode" && chosen.option === key;
      expect(state.players.p1.hand.some((card) => card.defId === prize.id)).toBe(right);
    }
  });

  it("R465 a KY's Test game replays from its log", () => {
    const qd = quickdrawOf(quiz).id;
    const { state: dealt, log, decks } = replayable("quiz-replay", [qd]);
    let state = dealt;
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", qd).id }, log);
    const pending = openAs(state, "answer", "p1");
    const key = must(answerKeyOf(pending.resume.data), "the key");
    state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [{ pick: "mode", option: key }] }, log);
    expect(state.players.p1.hand.some((card) => card.defId === prize.id)).toBe(true);
    expectReplays("quiz-replay", decks, log, state);
  });
});

describe("E18: the cell kind (Classic+ #62)", () => {
  it("E18 cells are zones of both sides and both rows, a lane at a time, with done after the first", () => {
    const state = board("papaya");
    const target = put(state, plain.id, slot("p2", "units", 2));
    const own = put(state, plain.id, slot("p1", "units", 4));
    castNow(state, papaya.id);
    const first = openAs(state, "cell", "p1");
    expect(first.options).toHaveLength(20);
    // The chooser's side from the hero outward — backrow, units — then the other side's units, backrow.
    expect(first.options.slice(0, 2).map((option) => option.key)).toEqual(["zone:p1:backrow:1", "zone:p1:backrow:2"]);
    expect(first.options.slice(5, 7).map((option) => option.key)).toEqual(["zone:p1:units:1", "zone:p1:units:2"]);
    expect(first.options.slice(10, 12).map((option) => option.key)).toEqual(["zone:p2:units:1", "zone:p2:units:2"]);
    expect(first.options.some((option) => option.selection.pick === "none")).toBe(false);
    const cells = forYou(viewFor(state, "p1").pending).options;
    expect(cells[10]).toMatchObject({ player: "p2", row: "units", lane: 1 });
    expectOnlyThatItIsOpen(state, "p2", "p1");

    answerKeys(state, "zone:p2:units:2");
    const second = openAs(state, "cell", "p1");
    // Lane 2 is used: 16 cells of the four other lanes, and "done".
    expect(second.options).toHaveLength(17);
    expect(second.options.some((option) => option.key.endsWith(":2"))).toBe(false);
    expect(second.options.at(-1)?.selection).toEqual({ pick: "none" });
    expect(promptAnswers(second)).toHaveLength(17);
    const copy = roundTrip(state);
    answerKeys(state, "none");
    answerKeys(copy, "none");
    expect(state.players.p2.exile.map((card) => card.id)).toEqual([target.id]);
    expect(state.players.p1.units[3]?.[0]?.id).toBe(own.id);
    expect(hashState(copy)).toBe(hashState(state));
  });

  it("E18 four cells in four lanes end the question without a done", () => {
    const state = board("papaya-four");
    castNow(state, papaya.id);
    for (const key of ["zone:p1:backrow:1", "zone:p2:units:2", "zone:p2:backrow:3", "zone:p1:units:4"]) {
      openAs(state, "cell", "p1");
      answerKeys(state, key);
    }
    expect(state.pending).toBeNull();
  });

  it("E18 R79 a timeout answers a cell prompt with one of its cells", () => {
    let state = board("papaya-timeout");
    castNow(state, papaya.id);
    const first = must(state.pending, "the first cell").id;
    state = act(state, { type: "timeout", playerId: "p1" });
    // The turn clock answers every prompt the answers open in turn, then ends the turn (R79).
    expect(state.pending).toBeNull();
    expect(state.active).toBe("p2");
    expect(first).toMatch(/^q/);
  });

  it("E18 a Papaya game replays from its log", () => {
    const qd = quickdrawOf(papaya).id;
    const { state: dealt, log, decks } = replayable("papaya-replay", [qd]);
    let state = dealt;
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", qd).id }, log);
    for (const selection of [
      { pick: "zone" as const, player: "p2" as const, row: "units" as const, lane: 3 },
      { pick: "none" as const },
    ]) {
      const pending = openAs(state, "cell", "p1");
      state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [selection] }, log);
    }
    expect(state.pending).toBeNull();
    expectReplays("papaya-replay", decks, log, state);
  });
});

describe("E18: the reward kind (Classic #90)", () => {
  it("E18 a reward prompt is its card's controller's, on the other player's turn too, with its own clock (R79)", () => {
    let state = board("quest");
    put(state, quest.id, slot("p1", "backrow", 1));
    setLibrary(state, "p2", [plain.id, plain.id]);
    state.active = "p2";
    const sink = sinkFor(state);
    draw(sink, "p2", 1);
    settle(sink);
    state.rngCursor = sink.rng.cursor;
    const pending = openAs(state, "reward", "p1");
    expect(pending.options.map((option) => ({ key: option.key, label: option.label }))).toEqual(
      QUEST_REWARDS.map((reward) => ({ key: `mode:${reward.id}`, label: reward.label })),
    );
    expect(forYou(viewFor(state, "p1").pending).options.map((option) => option.label)).toEqual(
      QUEST_REWARDS.map((reward) => reward.label),
    );
    expectOnlyThatItIsOpen(state, "p2", "p1");
    const copy = roundTrip(state);
    answerKeys(copy, "mode:A");
    expect(copy.players.p1.hero.health).toBe(HERO_HEALTH + 6);
    // The non-active player's clock answers it (R79), and the turn stays p2's.
    state = act(state, { type: "timeout", playerId: "p1" });
    expect(state.pending).toBeNull();
    expect(state.active).toBe("p2");
    const healed = state.players.p1.hero.health === HERO_HEALTH + 6;
    const hit = state.players.p2.hero.health === HERO_HEALTH - 3;
    expect(healed !== hit).toBe(true);
  });

  it("E18 a quest game, its reward asked on the other player's turn, replays from its log", () => {
    const qd = quickdrawOf(quest).id;
    const { state: dealt, log, decks } = replayable("quest-replay", [qd]);
    let state = act(dealt, { type: "play", playerId: "p1", instanceId: handCard(dealt, "p1", qd).id, zone: { row: "backrow", lane: 1 } }, log);
    if (state.active === "p1") state = act(state, { type: "endTurn", playerId: "p1" }, log);
    // p2's start-of-turn draw completed the quest: p1's reward prompt, on p2's turn.
    const pending = openAs(state, "reward", "p1");
    expect(state.active).toBe("p2");
    state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [{ pick: "mode", option: "B" }] }, log);
    expect(state.players.p2.hero.health).toBe(HERO_HEALTH - 3);
    expectReplays("quest-replay", decks, log, state);
  });
});

describe("E18: the pick kind (Classic #34, #44)", () => {
  it("E18 an up-to pick from a pile offers every card with its cost, no Discover limit", () => {
    const state = board("acquire");
    const cards = [graveCard(state, "p1", plain.id), graveCard(state, "p1", prize.id), graveCard(state, "p1", grunt.id)];
    castNow(state, acquire.id);
    const pending = openAs(state, "pick", "p1");
    expect(pending.options.map((option) => option.key)).toEqual(cards.map((card) => `instance:${card.id}`));
    expect(pending.min).toBe(0);
    expect(pending.max).toBe(2);
    expect(pending.budget).toBeUndefined();
    const shown = forYou(viewFor(state, "p1").pending);
    expect(shown.options.map((option) => option.cost)).toEqual([1, 1, 0]);
    expect(shown.budget).toBeUndefined();
    // Every set of up to two, each once: 1 + 3 + 3.
    const answers = promptAnswers(pending);
    expect(new Set(answers.map((answer) => JSON.stringify(answer.selection))).size).toBe(7);
    expect(answers).toHaveLength(7);
    expectOnlyThatItIsOpen(state, "p2", "p1");

    const copy = roundTrip(state);
    answerKeys(state, `instance:${cards[0]?.id ?? ""}`, `instance:${cards[2]?.id ?? ""}`);
    answerKeys(copy, `instance:${cards[0]?.id ?? ""}`, `instance:${cards[2]?.id ?? ""}`);
    expect(state.players.p1.hand.map((card) => card.id)).toEqual([cards[0]?.id, cards[2]?.id]);
    expect(state.players.p1.graveyard.map((card) => card.id)).toEqual([cards[1]?.id]);
    expect(hashState(copy)).toBe(hashState(state));
  });

  it("E18 a Radiant pick reaches the graveyard and the exile pile together", () => {
    const state = board("acquire-radiant");
    graveCard(state, "p1", plain.id);
    const exiled = newInstance(state, grunt.id, "p1", { z: "exile", player: "p1" });
    state.players.p1.exile.push(exiled);
    castNow(state, acquire.id, "p1", true);
    const pending = openAs(state, "pick", "p1");
    expect(pending.options).toHaveLength(2);
    expect(pending.max).toBe(2);
    answerKeys(state, ...pending.options.map((option) => option.key));
    expect(state.players.p1.hand.map((card) => card.id)).toContain(exiled.id);
  });

  it("E18 a budgeted pick refuses picks over the budget and lists only sets that fit", () => {
    const state = board("back");
    const costs = [1, 2, 3, 4];
    const units = costs.map((cost) => graveCard(state, "p1", plain.id, cost));
    graveCard(state, "p1", glitch.id);
    castNow(state, backFromGy.id);
    const pending = openAs(state, "pick", "p1");
    // Units only (the filter), each with its cost; the budget travels in the view.
    expect(pending.options.map((option) => option.cost)).toEqual(costs);
    expect(pending.budget).toBe(BACK_BUDGET);
    expect(forYou(viewFor(state, "p1").pending).budget).toBe(BACK_BUDGET);
    const over = [units[1], units[3]].map((card) => ({ pick: "instance" as const, instanceId: card?.id ?? "" }));
    expect(whyAnswerRefused(pending, { playerId: "p1", choiceId: pending.id, selection: over })).toMatch(/budget/);
    const answers = promptAnswers(pending);
    for (const answer of answers) {
      const spent = answer.selection.reduce((sum, pick) => {
        const option = pending.options.find((o) => o.selection.pick === "instance" && pick.pick === "instance" && o.selection.instanceId === pick.instanceId);
        return sum + (option?.cost ?? 0);
      }, 0);
      expect(spent).toBeLessThanOrEqual(BACK_BUDGET);
      expect(whyAnswerRefused(pending, { playerId: "p1", choiceId: pending.id, selection: answer.selection })).toBeNull();
    }
    // 1+4 and 2+3 are the fullest sets that fit; the empty set is an answer too ("up to").
    expect(answers.map((answer) => answer.selection.length)).toContain(0);
    const refused = act(roundTrip(state), { type: "timeout", playerId: "p1" });
    expect(refused.pending).toBeNull();

    answerKeys(state, `instance:${units[0]?.id ?? ""}`, `instance:${units[3]?.id ?? ""}`);
    expect(state.players.p1.units[0]?.[0]?.id).toBe(units[0]?.id);
    expect(state.players.p1.units[1]?.[0]?.id).toBe(units[3]?.id);
    expect(state.players.p1.graveyard).toHaveLength(3);
  });

  it("E18 a pick over a long pile stays bounded and still reaches every card", () => {
    const state = board("acquire-long");
    const cards = Array.from({ length: 30 }, () => graveCard(state, "p1", plain.id));
    castNow(state, acquire.id, "p1", true);
    const pending = openAs(state, "pick", "p1");
    expect(pending.max).toBe(4);
    const answers = promptAnswers(pending);
    expect(answers).toHaveLength(MAX_PROMPT_ANSWERS);
    const reached = new Set(answers.flatMap((answer) => answer.selection.map((pick) => (pick.pick === "instance" ? pick.instanceId : ""))));
    for (const card of cards) expect(reached.has(card.id)).toBe(true);
    // Each set is listed once, its picks in offered order (R221).
    expect(new Set(answers.map((answer) => JSON.stringify(answer.selection))).size).toBe(answers.length);
  });

  it("E18 a Back from the GY game replays from its log", () => {
    const back = quickdrawOf(backFromGy).id;
    const millCard = quickdrawOf(mill).id;
    const { state: dealt, log, decks } = replayable("back-replay", [millCard, back]);
    let state = dealt;
    // The mill puts three of p1's Units in the graveyard; Back from the GY brings two of them back.
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", millCard).id }, log);
    expect(state.players.p1.graveyard.filter((card) => card.defId !== millCard)).toHaveLength(3);
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", back).id }, log);
    const pending = openAs(state, "pick", "p1");
    expect(pending.budget).toBe(BACK_BUDGET);
    const picks = pending.options.slice(0, 2).map((option) => option.selection);
    state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: picks }, log);
    expect(state.players.p1.units.filter((pile) => pile !== null)).toHaveLength(2);
    expectReplays("back-replay", decks, log, state);
  });
});

describe("E18: a pick across zones (Classic #78's Radiant)", () => {
  it("E18 a pick reaches the field, the hand and the deck, and never shows the deck's order", () => {
    const state = board("cross-pick");
    const onField = put(state, plain.id, slot("p1", "units", 3));
    const held = inHand(state, grunt.id, "p1")[0] as CardInstance;
    const deck = [...setLibrary(state, "p1", [plain.id, prize.id, glitch.id])];
    // Library order is the reverse of creation order: the options must not follow it.
    state.players.p1.library = [...deck].reverse();
    castNow(state, crossPick.id);
    const pending = openAs(state, "pick", "p1");
    expect(pending.options.map((option) => option.key)).toEqual(
      [onField, held, deck[0], deck[1]].map((card) => `instance:${card?.id ?? ""}`),
    );
    expectOnlyThatItIsOpen(state, "p2", "p1");
    answerKeys(state, `instance:${deck[1]?.id ?? ""}`);
    expect(state.players.p1.exile.map((card) => card.id)).toEqual([deck[1]?.id]);
  });
});

describe("E17: the other player's hand as a prompt (Classic #11)", () => {
  it("E17 a hand prompt over the other player's hand is the chooser's to answer and to see", () => {
    const state = board("their-hand");
    const hand = inHand(state, plain.id, "p2", 2);
    const sink = sinkFor(state);
    chooseFromHand({ of: "enemy", step: "none", prompt: "Look" }).apply(makeContext(sink, null, { controller: "p1" }));
    const pending = openAs(state, "hand", "p1");
    expect(pending.options.map((option) => option.key)).toEqual(hand.map((card) => `instance:${card.id}`));
    expect(forYou(viewFor(state, "p1").pending).options.map((option) => option.defId)).toEqual([plain.id, plain.id]);
    expectOnlyThatItIsOpen(state, "p2", "p1");
  });

  it("E17 the chooser sees the other player's hand as the options; its holder sees a prompt and nothing else", () => {
    const state = board("mind-melt");
    const hand = [...inHand(state, plain.id, "p2"), ...inHand(state, grunt.id, "p2"), ...inHand(state, prize.id, "p2")];
    (hand[2] as CardInstance).radiant = true;
    castNow(state, mindMelt.id);
    const pending = openAs(state, "pick", "p1");
    expect(pending.min).toBe(1);
    expect(pending.max).toBe(1);
    expect(pending.options.map((option) => option.key)).toEqual(hand.map((card) => `instance:${card.id}`));
    const shown = forYou(viewFor(state, "p1").pending).options;
    expect(shown.map((option) => option.defId)).toEqual(hand.map((card) => card.defId));
    expect(shown[2]?.radiant).toBe(true);
    // The hand itself is still a count in the chooser's view: the prompt is the only window.
    expect(viewFor(state, "p1").opponent.hand).toEqual({ count: 3 });
    expectOnlyThatItIsOpen(state, "p2", "p1");
    expect(JSON.stringify(viewFor(state, "p2").pending)).not.toContain(plain.id);

    const { sink } = answerKeys(state, `instance:${hand[1]?.id ?? ""}`);
    // The view's events are the actions `reduce` applied (§9.3); this answer went in directly.
    state.applied.push({ nonce: "mind-melt-answer", events: sink.events });
    expect(state.players.p2.exile.map((card) => card.id)).toEqual([hand[1]?.id]);
    // Exile is public: both seats read the card that left.
    for (const viewer of ["p1", "p2"] as const) {
      const exiled = eventsOfType(viewFor(state, viewer).events, "exiled").at(-1);
      expect(exiled?.defId).toBe(grunt.id);
    }
  });

  it("E17 Radiant groups the other player's hand by the cost each would be played for, and exiles a group", () => {
    const state = board("mind-melt-radiant");
    const ones = inHand(state, plain.id, "p2", 2);
    const zero = inHand(state, grunt.id, "p2")[0] as CardInstance;
    const three = inHand(state, plain.id, "p2")[0] as CardInstance;
    three.costOverride = 3;
    castNow(state, mindMelt.id, "p1", true);
    const pending = openAs(state, "mode", "p1");
    expect(pending.options.map((option) => option.key)).toEqual(["mode:0", "mode:1", "mode:3"]);
    expect(pending.options[1]?.label).toContain(plain.name);
    expectOnlyThatItIsOpen(state, "p2", "p1");
    answerKeys(state, "mode:1");
    expect(state.players.p2.exile.map((card) => card.id).sort()).toEqual(ones.map((card) => card.id).sort());
    expect(state.players.p2.hand.map((card) => card.id).sort()).toEqual([zero.id, three.id].sort());
  });

  it("E17 a Mind Melt game replays from its log", () => {
    const qd = quickdrawOf(mindMelt).id;
    const { state: dealt, log, decks } = replayable("mind-melt-replay", [qd]);
    let state = dealt;
    state = act(state, { type: "play", playerId: "p1", instanceId: handCard(state, "p1", qd).id }, log);
    const pending = openAs(state, "pick", "p1");
    state = act(state, { type: "answer", playerId: "p1", choiceId: pending.id, selection: [at0(pending)] }, log);
    expect(state.players.p2.exile).toHaveLength(1);
    expectReplays("mind-melt-replay", decks, log, state);
  });
});
