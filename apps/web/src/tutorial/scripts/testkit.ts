// Shared helpers of the lesson tests (`*.test.ts` beside this file): fold a lesson run's log back into
// its steps, and replay the coach over it. Only tests import this file.

import { aiToAct } from "@jackioh/ai";
import { beginGame, createGame, legalActions, reduce, viewFor, type GameState } from "@jackioh/engine";
import { opponentOf, type Action, type GameEvent, type PlayerId, type PlayerView } from "@jackioh/shared";

import { newEventsSince } from "../../game/animations.ts";
import { COACH_START, coachAck, coachDisplay, coachObserve, type CoachCtx } from "../coach.ts";
import type { LessonRun } from "../harness.ts";

/** One accepted action of the game and everything it emitted, unredacted, with the states around it. */
export type Step = { action: Action; events: GameEvent[]; before: GameState; after: GameState };

/** Fold the run's log one action at a time, keeping every event (the test may read what the page may not). */
export function stepsOf(run: LessonRun): Step[] {
  const { seed, decks, handicaps, log } = run.debug;
  let state = beginGame(createGame({ seed, decks, handicaps })).state;
  const steps: Step[] = [];
  for (const action of log) {
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`the log does not fold: ${result.error}`);
    steps.push({ action, events: result.events, before: state, after: result.state });
    state = result.state;
  }
  return steps;
}

/**
 * Every run of "Got it" bubbles the coach showed on this line, in order: tips and info steps, with
 * no move of the player's and no step asking for one between them. Rebuilt by folding the run's
 * log and reading each snapshot the way the harness's coach did (the page's own reads: the human's
 * view, legal actions and whether the AI owes a move), which the tips it saw prove.
 */
export function gotItRuns(run: LessonRun): { runs: string[][]; tips: string[] } {
  const { seed, decks, handicaps, log } = run.debug;
  const human = run.humanSeat;
  const ai = opponentOf(human);
  let state = beginGame(createGame({ seed, decks, handicaps })).state;
  let coach = COACH_START;
  let previous: PlayerView | null = null;
  const runs: string[][] = [];
  const tips: string[] = [];
  let current: string[] = [];
  const close = (): void => {
    if (current.length > 0) runs.push(current);
    current = [];
  };
  const observe = (): void => {
    const view = viewFor(state, human);
    const ctx: CoachCtx = {
      view,
      legal: legalActions(state, human),
      fresh: previous === null ? [] : newEventsSince(previous.events, view.events),
      aiToAct: aiToAct(state, ai),
    };
    previous = view;
    coach = coachObserve(run.script, coach, ctx);
    let display = coachDisplay(run.script, coach, ctx);
    while (display.mode === "tip" || (display.mode === "step" && display.ack)) {
      current.push(display.id);
      if (display.mode === "tip") tips.push(display.id);
      coach = coachAck(run.script, coach, ctx);
      display = coachDisplay(run.script, coach, ctx);
    }
    if (display.mode === "step") close();
  };
  observe();
  for (const action of log) {
    if (action.playerId === human) close();
    state = reduce(state, action).state;
    observe();
  }
  close();
  return { runs, tips };
}

/** The player-turn number (the engine's `turn`) of the human's own turn `n` (p1 takes the odd ones). */
export function gameTurnOf(seat: PlayerId, n: number): number {
  return seat === "p1" ? 2 * n - 1 : 2 * n;
}
