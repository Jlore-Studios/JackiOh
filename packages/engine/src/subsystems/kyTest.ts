// Classic+ #42 KY's Test's question bank (docs/classic-sets.md B5 E31, SPEC §8.7 row 42, R420, R580).
//
// The machinery that is not card data: the Easy generator, a problem drawn from a bank handed in as
// data (the Medium and Hard problems live in `packages/cards`, which the engine never imports), the
// reward roll, and the two prompts. Each step is a plain-data continuation, so a paused game survives
// JSON and replays (§9.3):
//
//   cry    one reward rolled per difficulty (`KY_TEST_REWARDS`; Hard has one entry, so no draw, R129),
//          then a `mode` prompt of the three difficulties, each labelled "<difficulty>: <reward>".
//   ask    a problem of the chosen difficulty — generated for Easy (R580), drawn from the bank
//          otherwise — as an `answer` prompt (R465): the statement and the options in an rng-shuffled
//          order, the right letter in the resume data where no view reaches (§10.8).
//   grade  a right answer gains that difficulty's rolled reward (every card Radiant on the Radiant
//          face); a wrong one nothing. Pools are non-token cards of every set (R380) through
//          `addRandomFromCatalog`, which never offers the running card (R387); the hand cap burns.

import { HAND_CAP, KY_TEST_DIFFICULTIES, KY_TEST_EASY_ADDENDS, KY_TEST_EASY_MISSES, KY_TEST_OPTIONS, KY_TEST_REWARDS } from "../config";
import { addRandomFromCatalog, addToHand, answeredCorrectly, chooseAnswer, chooseMode, chosenOptions } from "../effects";
import type { Rng } from "../rng";
import type { Effect, EffectContext, Hook } from "../script";

export type KyTestDifficulty = (typeof KY_TEST_DIFFICULTIES)[number];

/** R420: one problem; `answer` is the one of its `KY_TEST_OPTIONS` options that is right. */
export type KyTestProblem = {
  readonly id: string;
  readonly difficulty: KyTestDifficulty;
  readonly statement: string;
  readonly options: readonly string[];
  readonly answer: string;
};

/** Resume-data keys: the rolled reward id per difficulty, and the difficulty chosen. */
const ROLLED = "kyTestRewards";
const CHOSEN = "kyTestDifficulty";

/**
 * R420, R580: a + b with each addend drawn from `KY_TEST_EASY_ADDENDS`, and three wrong sums, each
 * a + b moved by a different entry of `KY_TEST_EASY_MISSES` — so the four options always differ.
 */
export function easyProblem(rng: Rng): KyTestProblem {
  const { min, max } = KY_TEST_EASY_ADDENDS;
  const a = min + rng.int(max - min + 1);
  const b = min + rng.int(max - min + 1);
  const misses = rng.shuffle(KY_TEST_EASY_MISSES).slice(0, KY_TEST_OPTIONS - 1);
  return {
    id: `easy:${a}+${b}`,
    difficulty: "Easy",
    statement: `${a} + ${b} = ?`,
    options: [a + b, ...misses.map((miss) => a + b + miss)].map(String),
    answer: String(a + b),
  };
}

function isDifficulty(value: unknown): value is KyTestDifficulty {
  return (KY_TEST_DIFFICULTIES as readonly unknown[]).includes(value);
}

/** The whole card: its Cry and the two steps its prompts re-enter. `bank` holds the Medium and Hard problems. */
export function kyTestScript(bank: readonly KyTestProblem[]): { cry: Hook; resume: Record<string, Hook> } {
  const offer: Effect = {
    kind: "kyTestOffer",
    apply(ctx): void {
      const rolled: Record<string, string> = {};
      const labels: Record<string, string> = {};
      for (const difficulty of KY_TEST_DIFFICULTIES) {
        const list = KY_TEST_REWARDS[difficulty];
        const reward = list.length === 1 ? list[0] : ctx.rng.pick(list); // R129: one entry, no draw
        rolled[difficulty] = reward?.id ?? "";
        labels[difficulty] = `${difficulty}: ${reward?.label ?? ""}`;
      }
      chooseMode({ options: [...KY_TEST_DIFFICULTIES], labels, step: "ask", prompt: "Choose a problem", data: { [ROLLED]: rolled } }).apply(ctx);
    },
  };

  const ask: Effect = {
    kind: "kyTestAsk",
    apply(ctx): void {
      const difficulty = chosenOptions(ctx).find(isDifficulty);
      if (difficulty === undefined) return;
      const problems = bank.filter((problem) => problem.difficulty === difficulty);
      const problem =
        difficulty === "Easy" ? easyProblem(ctx.rng) : problems.length === 1 ? problems[0] : ctx.rng.pick(problems);
      if (problem === undefined) return;
      chooseAnswer({
        step: "grade",
        statement: problem.statement,
        options: problem.options,
        correct: problem.options.indexOf(problem.answer),
        data: { [CHOSEN]: difficulty, kyTestProblem: problem.id },
      }).apply(ctx);
    },
  };

  const grade: Hook = (ctx: EffectContext) => {
    const difficulty = ctx.data[CHOSEN];
    if (!answeredCorrectly(ctx) || !isDifficulty(difficulty)) return [];
    const id = (ctx.data[ROLLED] as Record<string, unknown> | undefined)?.[difficulty];
    const reward = KY_TEST_REWARDS[difficulty].find((entry) => entry.id === id);
    if (reward === undefined) return [];
    const count = reward.count === "fill" ? HAND_CAP - ctx.state.players[ctx.controller].hand.length : reward.count;
    if (count <= 0) return []; // R129: a full hand has no room to fill, and nothing is drawn
    const riders = { radiant: ctx.radiant, ...(reward.costOverride === undefined ? {} : { costOverride: reward.costOverride }) };
    const defId = reward.defId;
    if (defId !== undefined) return Array.from({ length: count }, () => addToHand({ defId, ...riders }));
    return [addRandomFromCatalog({ query: { ...reward.pool }, count, ...riders })];
  };

  return { cry: () => [offer], resume: { ask: () => [ask], grade } };
}
