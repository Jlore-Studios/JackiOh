// The prompt kinds patch v0.2.0 added (docs/classic-sets.md B5 E18): `number`, `answer`, `cell`,
// `reward`, a budgeted `pick`, and a mode prompt held by the player who did not play the card. The AI
// needs nothing new for them: `legalActions` lists every answer (`prompts.promptAnswers`), the beam
// scores each one through `reduce`, and `simulate` answers a prompt the other seat holds mid-line. Each
// test opens a kind for the AI's seat with a test-only continuation (the pattern of answer-key.test.ts,
// which proves R465's key-stripping and is not repeated here) and checks that `decide` answers within
// its budget, with reason "prompt", and picks the clearly better option where one is.

import { describe, expect, it } from "vitest";
import type { ActionBody, CardDef, PlayerId, Selection } from "@jackioh/shared";
import {
  AI_TUTORIAL,
  HUMAN_HANDICAP,
  cardAt,
  catalogVersion,
  createRng,
  effectiveCost,
  effects,
  findInstance,
  legalActions,
  makeContext,
  registerCatalog,
  registerScripts,
  registeredCatalog,
  registeredScripts,
  type Effect,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { AI_BUDGET, decide, playAiTurn, playMatch, type Decision } from "../src/index";
import { AI, HUMAN, act, clone, inGraveyard, isLegal, scenario, type ScenarioOptions } from "./_support";

const PROMPT_TIMEOUT = 120_000;
const PICKLE = "classic-008"; // Spell (1): the opponent chooses, three times (E18's mode held by the other player)

/**
 * The test-only card whose continuations the prompts below resume into: a Field Spell token, so a
 * verb that asks whether its source is a Spell (E35) finds a definition, and no pool ever deals it.
 */
const E18 = "ai-test-e18";
const e18Def: CardDef = {
  id: E18,
  index: "ai-test-3",
  name: "Prompt Fixture (AI test)",
  set: "Core",
  type: "Field Spell",
  tags: [],
  rarity: "Token",
  token: true,
  cost: 0,
  base: { keywords: [], text: "Asks a question." },
  radiant: { keywords: [], text: "Asks a question." },
};
const damageEnemy = (amount: number): Effect[] => (amount > 0 ? [effects.damage({ to: { of: "enemyHero" }, amount })] : []);
const continuations: Script = {
  resume: {
    // The number picked is damage to the enemy hero.
    number: (ctx) => damageEnemy(effects.chosenNumber(ctx) ?? 0),
    // The picked cards' costs, summed, are damage to the enemy hero.
    pick: (ctx) =>
      damageEnemy(
        ctx.targets.reduce((sum, selection) => {
          if (selection.pick !== "instance") return sum;
          const card = findInstance(ctx.state, selection.instanceId);
          return sum + (card === undefined ? 0 : effectiveCost(ctx.state, card));
        }, 0),
      ),
    // The unit in the picked cell is destroyed, whoever's it is.
    cell: (ctx) => {
      const [cell] = effects.chosenCells(ctx);
      const unit = cell === undefined ? null : cardAt(ctx.state, cell);
      return unit === null ? [] : [effects.destroy({ target: { of: "instance", instanceId: unit.id } })];
    },
    // Three rewards: 5 damage to your own hero, nothing, 5 damage to the enemy hero.
    reward: (ctx) => {
      const picked = effects.chosenOptions(ctx)[0];
      if (picked === "hurt-enemy") return damageEnemy(5);
      if (picked === "hurt-self") return [effects.damage({ to: { of: "selfHero" }, amount: 5 })];
      return [];
    },
    answer: () => [],
    // Run as the card's controller (the owner), whoever answered: "owner-hurt" hits the owner's hero.
    "their-mode": (ctx) => {
      const picked = effects.chosenOptions(ctx)[0];
      if (picked === "owner-hurt") return [effects.damage({ to: { of: "selfHero" }, amount: 5 })];
      if (picked === "chooser-hurt") return damageEnemy(5);
      return [];
    },
  },
};
registerCatalog({ ...registeredCatalog(), [E18]: e18Def }, catalogVersion());
registerScripts({ ...registeredScripts(), [E18]: { base: continuations, radiant: continuations } });

/** `state` with `effect` applied for a card of `controller`'s, as a resolving card would apply it. */
function opened(state: GameState, controller: PlayerId, effect: Effect): GameState {
  const sink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
  effect.apply({ ...makeContext(sink, null, { controller }), defId: E18 });
  state.rngCursor = sink.rng.cursor;
  return state;
}

/** A scenario with libraries on both sides (no fatigue in any line) and `effect` opened for p1's card. */
function asked(seed: string, effect: Effect, setup: ScenarioOptions = {}, controller: PlayerId = AI): GameState {
  const state = scenario({
    seed,
    ...setup,
    p1: { library: ["core-053", "core-030"], ...setup.p1 },
    p2: { hand: ["core-005"], library: ["core-053", "core-030"], ...setup.p2 },
  }).state;
  return opened(state, controller, effect);
}

/** decide on a prompt the AI holds: a legal answer, reason "prompt", within the budget. */
function answered(state: GameState, seed: string): Extract<ActionBody, { type: "answer" }> {
  expect(state.pending?.playerId).toBe(AI);
  const decision = decide(clone(state), AI, { rng: createRng(seed) }) as Decision;
  expect(decision).not.toBeNull();
  expect(decision.reason).toBe("prompt");
  expect(decision.stats.nodes).toBeLessThanOrEqual(AI_BUDGET.nodes);
  expect(decision.stats.simErrors).toBe(0);
  expect(decision.action.type).toBe("answer");
  expect(isLegal(state, AI, decision.action)).toBe(true);
  return decision.action as Extract<ActionBody, { type: "answer" }>;
}

describe("E18: the AI answers the new prompt kinds", () => {
  it("E18: a `number` prompt whose number is damage to the enemy hero is answered with the largest", { timeout: PROMPT_TIMEOUT }, () => {
    const state = asked("e18-number", effects.chooseNumber({ step: "number", from: 0, to: 5 }));
    expect(state.pending?.kind).toBe("number");
    expect(answered(state, "e18-number").selection).toEqual([{ pick: "mode", option: "5" }]);
  });

  it("E18: a budgeted `pick` takes the most the budget affords", { timeout: PROMPT_TIMEOUT }, () => {
    // Midrange Menace (3), Pointmaster (2), Archivist (2), Mr. Vanilla (1), two picks within (4): the
    // best sets spend all four.
    const state = asked(
      "e18-pick",
      effects.choosePick({ step: "pick", from: [{ zone: "graveyard" }], max: 2, budget: 4 }),
      { p1: { graveyard: ["core-019", "core-020", "core-030", "core-008"] } },
    );
    expect(state.pending?.kind).toBe("pick");
    expect(state.pending?.budget).toBe(4);
    const picked = answered(state, "e18-pick").selection.map((selection: Selection) => {
      if (selection.pick !== "instance") throw new Error("a pick answers with cards");
      const card = findInstance(state, selection.instanceId);
      if (card === undefined) throw new Error("no such card");
      return effectiveCost(state, card);
    });
    expect(picked.reduce((sum, cost) => sum + cost, 0)).toBe(4);
  });

  it("E18: a `cell` prompt picks the one cell where the destroy hits an enemy", { timeout: PROMPT_TIMEOUT }, () => {
    // p1's Mr. Vanilla in lane 1 and p2's Pointmaster in lane 3; every other cell is empty.
    const state = asked("e18-cell", effects.chooseCell({ step: "cell", cells: { rows: ["units"] } }), {
      p1: { field: ["core-008"] },
      p2: { field: [{ def: "core-020", lane: 3 }] },
    });
    expect(state.pending?.kind).toBe("cell");
    expect(answered(state, "e18-cell").selection).toEqual([{ pick: "zone", player: HUMAN, row: "units", lane: 3 }]);
  });

  it("E18: a `reward` prompt takes the reward that hurts the enemy", { timeout: PROMPT_TIMEOUT }, () => {
    const rewards = [
      { id: "hurt-self", label: "Take 5" },
      { id: "nothing", label: "Nothing" },
      { id: "hurt-enemy", label: "Deal 5" },
    ];
    const state = asked("e18-reward", effects.chooseReward({ step: "reward", rewards }));
    expect(state.pending?.kind).toBe("reward");
    expect(answered(state, "e18-reward").selection).toEqual([{ pick: "mode", option: "hurt-enemy" }]);
  });

  it("E18: an `answer` prompt gets one of its options", { timeout: PROMPT_TIMEOUT }, () => {
    const state = asked(
      "e18-answer",
      effects.chooseAnswer({ step: "answer", statement: "2 + 2 = ?", options: ["3", "4", "5"], correct: 1, shuffle: false }),
    );
    expect(state.pending?.kind).toBe("answer");
    const options = state.pending?.options.map((option) => option.selection) ?? [];
    expect(options).toContainEqual(answered(state, "e18-answer").selection[0]);
  });

  it("E18: a mode prompt the opponent's card hands the AI, on the opponent's turn, is answered against the card's owner", { timeout: PROMPT_TIMEOUT }, () => {
    // p2 plays a card whose question p1 answers (`by: "enemy"`); the answer runs as p2's.
    const state = asked(
      "e18-their-mode",
      effects.chooseMode({ step: "their-mode", options: ["chooser-hurt", "owner-hurt"], by: "enemy" }),
      { active: HUMAN, turn: 10, p1: { hand: ["core-053"] } },
      HUMAN,
    );
    expect(state.active).toBe(HUMAN);
    expect(state.pending?.kind).toBe("mode");
    expect(answered(state, "e18-their-mode").selection).toEqual([{ pick: "mode", option: "owner-hurt" }]);
  });

  it("E18: the AI's own play opens mode prompts the opponent holds; the search answers them in simulation and the turn completes", { timeout: PROMPT_TIMEOUT }, () => {
    // Pickle is the AI's only card: the opponent chooses three times, discarding from their own hand.
    let state = scenario({
      seed: "e18-pickle",
      p1: { hand: [PICKLE], library: ["core-053", "core-030", "core-037"] },
      p2: { hand: ["core-005", "core-011", "core-035"], library: ["core-053", "core-030", "core-037"] },
    }).state;
    const start = state.turn;

    const first = decide(clone(state), AI, { rng: createRng("e18-pickle") });
    expect(first?.action.type).toBe("play");
    expect(first?.stats.simErrors).toBe(0);

    const decisions: Decision[] = [];
    let humanAnswers = 0;
    for (let step = 0; step < 40 && state.result === null && state.turn === start; step += 1) {
      const pending = state.pending;
      if (pending !== null && pending.playerId === HUMAN) {
        expect(pending.kind === "mode" || pending.kind === "hand").toBe(true);
        const answer = legalActions(state, HUMAN).find((action) => action.type === "answer");
        if (answer === undefined) throw new Error("the human's prompt has no answer");
        state = act(state, HUMAN, answer, `pickle-human-${step}`);
        humanAnswers += 1;
        continue;
      }
      const turn = playAiTurn(state, AI, { rng: createRng(`e18-pickle-${step}`) });
      decisions.push(...turn.decisions);
      if (turn.actions.length === 0) break;
      state = turn.state;
    }

    expect(humanAnswers).toBeGreaterThanOrEqual(3);
    expect(inGraveyard(state, AI, PICKLE)).toBe(true);
    expect(state.result !== null || state.turn > start).toBe(true);
    expect(decisions.every((decision) => decision.reason !== "fallback" && decision.stats.simErrors === 0)).toBe(true);
  });
});

describe("E18: a whole game with the new prompts", () => {
  it("E18: playMatch finishes a short AI-vs-greedy game in which E18 prompts open, with no refusal, throw or fallback", { timeout: 300_000 }, () => {
    // Pickle (mode prompts the opponent holds), Ancient Acquisition and Back from the GY (picks, one
    // budgeted), Mind Melt (a pick of the opponent's hand), beside cheap Units: eight cards and a hero
    // of 20, so that the game is short and the prompt cards are drawn. On this seed greedy's Pickle
    // hands the AI three mode prompts on greedy's turn, and the AI's Back from the GY asks a pick.
    const deck = [PICKLE, "classic-011", "classic-034", "classic-044", "core-008", "core-020", "core-030", "core-011"];
    const handicap = { ...HUMAN_HANDICAP, deckSize: deck.length, heroHealth: AI_TUTORIAL.heroHealth };
    const opened: string[] = [];
    const record = playMatch(
      {
        seed: "e18-match",
        decks: [deck, deck],
        handicaps: { p1: handicap, p2: handicap },
        controllers: { p1: { kind: "ai" }, p2: { kind: "greedy" } },
      },
      {
        afterAction: (_before, after) => {
          const pending = after.pending;
          if (pending === null) return;
          const theirs = pending.kind === "mode" && pending.playerId !== after.active;
          if (theirs || ["number", "answer", "cell", "reward", "pick"].includes(pending.kind)) opened.push(pending.kind);
        },
      },
    );
    expect(record.result).not.toBeNull();
    expect(record.thrown).toEqual([]);
    expect(record.rejected).toEqual([]);
    expect(record.fallbacks).toBe(0);
    expect(opened, opened.join(" ")).toContain("mode");
    expect(opened, opened.join(" ")).toContain("pick");
  });
});
