// The coach machine (tutorial/coach.ts), on hand-built views: R292, R314 on the real lessons' scripts
// too, and R4702004. The coach reads the human's view, their legal actions and whether the AI owes a move,
// and nothing else (CLAUDE.md rule 7); with no Skip step to fall back on (R314), nothing may strand a player.

import { describe, expect, it } from "vitest";

import type { ActionBody, GameEvent, PlayerView } from "@jackioh/shared";

import { baseView, card, emptySide, resetIds } from "../test/fixtures.ts";
import {
  COACH_START,
  activeStep,
  coachAck,
  coachDisplay,
  coachHint,
  coachObserve,
  coachStart,
  type CoachCtx,
  type CoachDisplay,
  type CoachState,
  type CoachStep,
  type LessonScript,
} from "./coach.ts";
import { TUTORIAL_STEP_TURNS_MAX } from "./config.ts";
import type { Discovery } from "./discoveries.ts";
import { LESSON_SCRIPTS } from "./scripts/index.ts";
import { momentOf, playOf } from "./scripts/kit.ts";
import { info } from "./steps.ts";
import { coachTargets } from "./targets.ts";

const VANILLA = "core-008";

function ctx(view: PlayerView, legal: ActionBody[] = [], fresh: GameEvent[] = [], aiToAct = false): CoachCtx {
  return { view, legal, fresh, aiToAct };
}

function myTurn(turn: number, hand = [card({ instanceId: "h1", defId: VANILLA, cost: 1 })], over: Partial<PlayerView> = {}): PlayerView {
  return baseView({ turn, active: "p1", you: emptySide("p1", { hand }), ...over });
}

/** p2's turn, the human still holding Mr. Vanilla unless `over` says otherwise. */
function aiTurnView(turn: number, over: Partial<PlayerView> = {}): PlayerView {
  return baseView({ turn, active: "p2", you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: VANILLA, cost: 1 })] }), ...over });
}

const PLAY_VANILLA: ActionBody = { type: "play", instanceId: "h1", zone: { row: "units", lane: 1 } };

/** The one Field guide entry these scripts watch: found once the player plays a card. */
const GUIDE: readonly Discovery[] = [
  { id: "x", name: "Mana", rule: "each card costs mana.", lesson: "basics", found: (c) => c.fresh.some((event) => event.type === "cardPlayed" && event.player === c.view.viewer) },
];
const PLAYED: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "h1", defId: VANILLA, costPaid: 1 };

/** A moment as the lessons build theirs (scripts/kit.ts): it asks for Mr. Vanilla to be played, and sets up `x`. */
function askForVanilla(id: string, over: { revisit?: boolean } = {}): CoachStep {
  return momentOf({
    id,
    title: "Play it",
    text: "What can you afford?",
    discovers: ["x"],
    hints: ["Your cards show their cost.", "Play Mr. Vanilla."],
    pick: playOf([VANILLA]),
    ...over,
  });
}

describe("R292 the coach", () => {
  it("R292 activates a step only when its moment comes, and its last hint points at the element it names", () => {
    resetIds();
    const script: LessonScript = { lessonId: "t", steps: [askForVanilla("play")], tips: [] };
    let state = coachObserve(script, COACH_START, ctx(aiTurnView(2), [], [], true));
    expect(coachDisplay(script, state, ctx(aiTurnView(2), [], [], true))).toEqual({
      mode: "waiting",
      stepNumber: 1,
      stepCount: 1,
      aiToAct: true,
    });

    const view = myTurn(3);
    const context = ctx(view, [PLAY_VANILLA, { type: "endTurn" }]);
    state = coachObserve(script, state, context);
    const display = coachDisplay(script, state, context);
    expect(display.mode).toBe("step");
    if (display.mode !== "step") return;
    expect(display.title).toBe("Play it");
    expect(display.ack).toBe(false);
    expect(display.anchor, "the question points at nothing").toBeNull();
    const pointed = coachDisplay(script, coachHint(script, coachHint(script, state)), context);
    expect(pointed.mode === "step" ? coachTargets(pointed.anchor, view) : null).toEqual(["hand-card-h1"]);
    expect(activeStep(script, state)?.id).toBe("play");
  });

  it("R292 waits while the engine offers no such action, since legality is the engine's", () => {
    const script: LessonScript = { lessonId: "t", steps: [askForVanilla("play")], tips: [] };
    // In hand but not offered: the coach never decides it is playable itself.
    const state = coachObserve(script, COACH_START, ctx(myTurn(3), [{ type: "endTurn" }]));
    expect(state.since).toBeNull();
    expect(coachDisplay(script, state, ctx(myTurn(3), [{ type: "endTurn" }])).mode).toBe("waiting");
  });

  it("R292 completes a step when the view shows it done, including one done before it showed", () => {
    const script: LessonScript = {
      lessonId: "t",
      guide: GUIDE,
      teaches: ["x"],
      steps: [askForVanilla("play"), info({ id: "next", title: "Next", text: "Next." })],
      tips: [],
    };
    let state = coachObserve(script, COACH_START, ctx(myTurn(3), [PLAY_VANILLA, { type: "endTurn" }]));
    expect(activeStep(script, state)?.id).toBe("play");
    state = coachObserve(script, state, ctx(myTurn(3, []), [{ type: "endTurn" }], [PLAYED]));
    expect(state.outcomes).toEqual({ play: "done" });

    // Found on the AI's turn, before the step could show: done without a word.
    expect(coachObserve(script, COACH_START, ctx(aiTurnView(2), [], [PLAYED], true)).outcomes).toEqual({ play: "done" });
  });

  it("R292 retires a step that can no longer happen before it ever showed: a revisit of what the Field guide holds", () => {
    const script: LessonScript = {
      lessonId: "t",
      steps: [askForVanilla("again", { revisit: true }), info({ id: "next", title: "Next", text: "Next." })],
      tips: [],
    };
    const state = coachObserve(script, coachStart(["x"]), ctx(myTurn(3), [PLAY_VANILLA, { type: "endTurn" }]));
    expect(state.outcomes).toEqual({ again: "moot" });
    expect(activeStep(script, state)?.id).toBe("next");
  });

  it(`R292 expires a step still current after ${String(TUTORIAL_STEP_TURNS_MAX)} of the player's own turns, but never the final one`, () => {
    const never = (): boolean => false;
    const script: LessonScript = {
      lessonId: "t",
      steps: [
        info({ id: "stuck", title: "Stuck", text: "Waits for something that never comes.", when: never }),
        info({ id: "win", title: "Win", text: "Win the game.", final: true, when: never }),
      ],
      tips: [],
    };
    let state = COACH_START;
    let turn = 1;
    const cycle = (): void => {
      state = coachObserve(script, state, ctx(myTurn(turn)));
      turn += 1;
      state = coachObserve(script, state, ctx(aiTurnView(turn), [], [], true));
      turn += 1;
    };
    for (let i = 0; i < TUTORIAL_STEP_TURNS_MAX + 1; i += 1) cycle();
    state = coachObserve(script, state, ctx(myTurn(turn)));
    expect(state.outcomes["stuck"]).toBe("expired");
    expect(activeStep(script, state)).toBeNull();
    expect(state.index).toBe(1);

    for (let i = 0; i < TUTORIAL_STEP_TURNS_MAX * 3; i += 1) cycle();
    expect(state.index).toBe(1);
    expect(state.finished).toBe(false);
  });

  it("R292 finishes when the game ends, whatever step it was on", () => {
    const script: LessonScript = {
      lessonId: "t",
      steps: [
        info({ id: "a", title: "A", text: "A." }),
        info({ id: "b", title: "B", text: "B." }),
      ],
      tips: [{ id: "t", title: "T", text: "T.", when: () => true }],
    };
    const over = myTurn(9, [], { result: { winner: "p1", reason: "hero-death" } });
    const state = coachObserve(script, coachObserve(script, COACH_START, ctx(myTurn(3))), ctx(over));
    expect(state.finished).toBe(true);
    expect(state.tipQueue).toEqual([]);
    expect(coachDisplay(script, state, ctx(over))).toEqual({ mode: "finished" });
    expect(coachAck(script, state, ctx(over))).toBe(state);
    expect(coachObserve(script, state, ctx(myTurn(3)))).toBe(state);
  });

  it("R292 shows a tip once, ahead of the step, while the step's done is still watched", () => {
    const coin: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "x", defId: "core-t-coin", costPaid: 0 };
    const script: LessonScript = {
      lessonId: "t",
      guide: GUIDE,
      teaches: ["x"],
      steps: [askForVanilla("play"), info({ id: "end", title: "End turn", text: "End your turn." })],
      tips: [
        {
          id: "coin",
          title: "The Coin",
          text: "The AI played The Coin.",
          holdAi: true,
          anchor: { kind: "hero", side: "opponent" },
          when: (c) => c.fresh.some((event) => event.type === "cardPlayed" && event.defId === "core-t-coin"),
        },
      ],
    };
    const legal = [PLAY_VANILLA, { type: "endTurn" } as ActionBody];
    let state = coachObserve(script, COACH_START, ctx(myTurn(3), legal, [coin]));
    const shown = coachDisplay(script, state, ctx(myTurn(3), legal, [coin]));
    expect(shown.mode).toBe("tip");
    expect(shown.mode === "tip" && shown.holdAi && shown.ack).toBe(true);
    // No step is "active" for the tests while a tip is up.
    expect(activeStep(script, state)).toBeNull();

    state = coachObserve(script, state, ctx(myTurn(3, []), [{ type: "endTurn" }], [coin, PLAYED]));
    expect(state.outcomes["play"]).toBe("done");
    expect(state.tipQueue).toEqual(["coin"]);

    // Got it takes the find's own bubble first, then the tip.
    state = coachAck(script, coachAck(script, state, ctx(myTurn(3, []))), ctx(myTurn(3, [])));
    expect(state.tipsSeen).toEqual(["coin"]);
    state = coachObserve(script, state, ctx(myTurn(3, []), [{ type: "endTurn" }], [coin]));
    expect(state.tipQueue).toEqual([]);
    expect(activeStep(script, state)?.id).toBe("end");
  });

  it("R292 still shows tips once the script's steps are through, until the game ends", () => {
    const coin: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "x", defId: "core-t-coin", costPaid: 0 };
    const script: LessonScript = {
      lessonId: "t",
      steps: [info({ id: "only", title: "Only", text: "The only step.", final: true })],
      tips: [
        {
          id: "coin",
          title: "The Coin",
          text: "The AI played The Coin.",
          when: (c) => c.fresh.some((event) => event.type === "cardPlayed" && event.defId === "core-t-coin"),
        },
      ],
    };
    let state = coachObserve(script, COACH_START, ctx(myTurn(3)));
    state = coachAck(script, state, ctx(myTurn(3)));
    expect(state.index).toBe(1);
    state = coachObserve(script, state, ctx(aiTurnView(4), [], [coin], true));
    expect(coachDisplay(script, state, ctx(aiTurnView(4), [], [coin], true)).mode).toBe("tip");
    state = coachAck(script, state, ctx(aiTurnView(4)));
    expect(coachDisplay(script, state, ctx(aiTurnView(4))).mode).toBe("finished");
  });

  it("R292 reads a lesson predicate that throws as 'no' rather than stopping the game", () => {
    const script: LessonScript = {
      lessonId: "t",
      steps: [
        info({
          id: "broken",
          title: "Broken",
          text: "A buggy step.",
          when: () => {
            throw new Error("bug");
          },
        }),
      ],
      tips: [],
    };
    expect(() => coachObserve(script, COACH_START, ctx(myTurn(3)))).not.toThrow();
  });
});

describe("R314 no Skip step, and nothing strands the player", () => {
  /** A step's display with it showing (or a tip's, first in the queue), on the player's own turn. */
  function shownDisplay(script: LessonScript, state: Partial<CoachState>): CoachDisplay {
    const view = myTurn(3);
    return coachDisplay(script, { ...COACH_START, ...state }, ctx(view, [{ type: "endTurn" }]));
  }

  it("R314 without Skip step no step strands the player: in every lesson only a tip or an info step holds the AI, and each shows Got it", () => {
    const lessons = Object.values(LESSON_SCRIPTS);
    expect(lessons.length, "the lessons have scripts").toBeGreaterThan(0);
    for (const script of lessons) {
      expect(script.steps.length, `${script.lessonId} has steps`).toBeGreaterThan(0);
      for (const [index, step] of script.steps.entries()) {
        const where = `${script.lessonId} step ${step.id}`;
        if (step.holdAi === true) expect(step.kind, `${where} holds the AI, so it is an info step`).toBe("info");
        // Only the last step may be final: every other one can expire.
        if (step.final === true) expect(index, `${where} is final, so it is the lesson's last`).toBe(script.steps.length - 1);
        const display = shownDisplay(script, { index, since: myTurn(3) });
        expect(display.mode, where).toBe("step");
        if (display.mode !== "step") continue;
        expect(display.ack, `${where}: Got it exactly on an info step`).toBe(step.kind === "info");
        if (display.holdAi) expect(display.ack, `${where} holds the AI, so it shows Got it`).toBe(true);
      }
      for (const each of script.tips) {
        const display = shownDisplay(script, { tipQueue: [each.id] });
        expect(display.mode, `${script.lessonId} tip ${each.id}`).toBe("tip");
        if (display.mode === "tip") expect(display.ack, `${script.lessonId} tip ${each.id} shows Got it`).toBe(true);
      }
    }
  });

  it("R314 without Skip step no step strands the player: a moment never holds the AI, even when its script asks", () => {
    const script: LessonScript = { lessonId: "t", steps: [{ ...askForVanilla("ask"), holdAi: true }], tips: [] };
    const state = coachObserve(script, COACH_START, ctx(myTurn(3), [PLAY_VANILLA, { type: "endTurn" }]));
    expect(shownDisplay(script, state)).toMatchObject({ mode: "step", id: "ask", ack: false, holdAi: false });
  });

  it(`R314 without Skip step no step strands the player: a step still current after ${String(TUTORIAL_STEP_TURNS_MAX)} of the player's own turn starts expires, and the final one ends with the game`, () => {
    const script: LessonScript = {
      lessonId: "t",
      steps: [
        // Waits for a moment that never comes (the engine never offers the play).
        askForVanilla("waits"),
        // Shows, and the player never answers it.
        info({ id: "shown", title: "Read this", text: "Something the player never answers." }),
        info({ id: "win", title: "Win", text: "Win the game.", final: true }),
      ],
      tips: [],
    };
    const legal: ActionBody[] = [{ type: "endTurn" }];
    let turn = 1;
    let state = coachObserve(script, COACH_START, ctx(myTurn(turn), legal));
    const display = (): CoachDisplay => coachDisplay(script, state, ctx(myTurn(turn), legal));
    const ownTurnStart = (): void => {
      turn += 1;
      state = coachObserve(script, state, ctx(aiTurnView(turn), [], [], true));
      turn += 1;
      state = coachObserve(script, state, ctx(myTurn(turn), legal));
    };

    expect(display()).toMatchObject({ mode: "waiting", stepNumber: 1 });
    for (let start = 1; start <= TUTORIAL_STEP_TURNS_MAX; start += 1) {
      ownTurnStart();
      expect(state.index, `still on the first step after ${String(start)} of the player's turn starts`).toBe(0);
    }
    ownTurnStart();
    expect(state.outcomes).toEqual({ waits: "expired" });
    expect(display()).toMatchObject({ mode: "step", id: "shown", ack: true, holdAi: false });

    for (let start = 1; start <= TUTORIAL_STEP_TURNS_MAX; start += 1) {
      ownTurnStart();
      expect(activeStep(script, state)?.id, `still showing after ${String(start)} of the player's turn starts`).toBe("shown");
    }
    ownTurnStart();
    expect(state.outcomes).toEqual({ waits: "expired", shown: "expired" });
    expect(activeStep(script, state)?.id).toBe("win");

    for (let start = 1; start <= TUTORIAL_STEP_TURNS_MAX * 3; start += 1) ownTurnStart();
    expect(activeStep(script, state)?.id).toBe("win");
    expect(state.outcomes["win"]).toBeUndefined();
    expect(state.finished).toBe(false);

    const over = myTurn(turn + 1, [], { result: { winner: "p2", reason: "hero-death" } });
    state = coachObserve(script, state, ctx(over));
    expect(state.finished).toBe(true);
    expect(coachDisplay(script, state, ctx(over))).toEqual({ mode: "finished" });
  });
});

describe("R4702004 the coach has no act steps", () => {
  it("R4702004 every step of every shipped lesson is an info step or a moment", () => {
    const steps = Object.values(LESSON_SCRIPTS).flatMap((script) => script.steps.map((step) => ({ lesson: script.lessonId, step })));
    expect(steps.length).toBeGreaterThan(0);
    for (const { lesson, step } of steps) expect(["info", "moment"], `${lesson} step ${step.id}`).toContain(step.kind);
  });
});
