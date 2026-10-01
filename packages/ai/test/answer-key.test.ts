// R465: a multiple-choice problem's key never leaves the engine (Classic+ #42 KY's Test, the `answer`
// prompt kind, docs/classic-sets.md B5 E18). `viewFor` never sends it; this file proves the AI's half:
// `redact` (R185) strips it from what the AI reads — its own prompt's resume data included — exactly
// as a human never sees it, so two states that differ only in which option is right redact alike and
// the AI answers them alike, from what the prompt shows.

import { describe, expect, it } from "vitest";
import {
  ANSWER_KEY,
  answerKeyOf,
  createRng,
  effects,
  hashState,
  legalActions,
  makeContext,
  registerScripts,
  registeredScripts,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { decide, redact } from "../src/index";
import { act, clone, dealtGame } from "./_support";

/** A test-only continuation: the right answer deals 10 to the enemy hero, a wrong one nothing. */
const QUIZ_DEF = "ai-test-quiz";
const quiz: Script = {
  resume: {
    answered: (ctx) =>
      effects.answeredCorrectly(ctx) ? [effects.damage({ to: { of: "enemyHero" }, amount: 10 })] : [],
  },
};

/** p1's main phase with an `answer` prompt open for p1, its key the option at `right`. */
function asked(seed: string, right: number): GameState {
  registerScripts({ ...registeredScripts(), [QUIZ_DEF]: { base: quiz, radiant: quiz } });
  let state = dealtGame(seed);
  state = act(state, "p1", { type: "mulligan", keep: state.players.p1.hand.map((card) => card.id) });
  state = act(state, "p2", { type: "mulligan", keep: state.players.p2.hand.map((card) => card.id) });
  const sink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
  const ctx = { ...makeContext(sink, null, { controller: "p1" }), defId: QUIZ_DEF };
  effects
    .chooseAnswer({ step: "answered", statement: "7 + 5 = ?", options: ["12", "11", "13", "75"], correct: right, shuffle: false })
    .apply(ctx);
  state.rngCursor = sink.rng.cursor;
  return state;
}

describe("R465: the AI never reads a problem's key", () => {
  it("R465 redaction strips the key from the AI's own prompt, and states that differ only in it redact alike", () => {
    const first = asked("r465-a", 0);
    const second = asked("r465-a", 3);
    expect(answerKeyOf(first.pending?.resume.data ?? {})).toBe("A");
    expect(answerKeyOf(second.pending?.resume.data ?? {})).toBe("D");
    for (const seat of ["p1", "p2"] as const) {
      const seen = redact(first, seat);
      expect(JSON.stringify(seen)).not.toContain(ANSWER_KEY);
      expect(hashState(seen)).toBe(hashState(redact(second, seat)));
    }
    // The chooser still sees the problem: its statement and its four options.
    expect(redact(first, "p1").pending?.options.map((option) => option.label)).toEqual(["12", "11", "13", "75"]);
    // The true state is untouched.
    expect(answerKeyOf(first.pending?.resume.data ?? {})).toBe("A");
  });

  it("R465 the AI answers from what the prompt shows: the same answer whichever option is right", () => {
    const decisions = [0, 1, 2, 3].map((right) => {
      const state = asked("r465-b", right);
      const decision = decide(clone(state), "p1", { rng: createRng("r465-decide") });
      expect(decision).not.toBeNull();
      const legal = legalActions(state, "p1").map((action) => JSON.stringify(action));
      expect(legal).toContain(JSON.stringify(decision?.action));
      return JSON.stringify(decision?.action);
    });
    expect(new Set(decisions).size).toBe(1);
  });
});
