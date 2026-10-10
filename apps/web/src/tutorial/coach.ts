// Tutorial coach tracks lesson steps and tips without deciding legality (SPEC §9.10, R292;
// CLAUDE.md rule 7). There is no skip: only ackable `info` steps or tips may hold AI (R314).
// Non-final steps expire after `TUTORIAL_STEP_TURNS_MAX` player turns; tips survive steps until game
// end, and the HUD always offers exit.

import type { ActionBody, GameEvent, PlayerView, Row } from "@jackioh/shared";

import type { Side } from "../game/contract.ts";
import { TUTORIAL_STEP_TURNS_MAX } from "./config.ts";

/** Everything the coach reads, all of it already the page's. */
export type CoachCtx = {
  view: PlayerView;
  /** legalActions(state, human). */
  legal: readonly ActionBody[];
  /** Events new since the previously observed view. */
  fresh: readonly GameEvent[];
  aiToAct: boolean;
  /** Public catalog lookup (§5.1); absent without a catalog in tests. */
  nameOf?: (defId: string) => string | undefined;
};

/** Board anchors; `coachTargets` maps them to the board's `data-testid`s. */
export type CoachAnchor =
  /** The human's hand card of this definition (a deck holds each card once, §2.6). */
  | { kind: "handCard"; defId: string }
  | { kind: "hand" }
  | { kind: "unit"; side: Side; defId: string }
  | { kind: "units"; side: Side }
  | { kind: "backrow"; side: Side; lane?: number }
  | { kind: "zone"; side: Side; row: Row; lane: number }
  | { kind: "hero"; side: Side }
  | { kind: "mana" }
  | { kind: "endTurn" }
  | { kind: "prompt" }
  | { kind: "library"; side: Side }
  | { kind: "graveyard"; side: Side };

type Text = string | ((ctx: CoachCtx) => string);
type AnchorOf = CoachAnchor | null | ((ctx: CoachCtx) => CoachAnchor | null);

export type CoachStep = {
  /** Tests and the page's `data-coach-step` use this lesson-unique id. */
  id: string;
  title: string;
  text: Text;
  anchor?: AnchorOf;
  /** `info`: "Got it" completes it. `act`: its `done` completes it. */
  kind: "info" | "act";
  /** `when` gates activation. */
  when?: (ctx: CoachCtx) => boolean;
  /** Required for `act`; receives the activation view. */
  done?: (ctx: CoachCtx, since: PlayerView) => boolean;
  /** Retires a no-longer-sensible step without showing it. */
  moot?: (ctx: CoachCtx, since: PlayerView | null) => boolean;
  /** Lesson tests follow this legal-action predicate; the page never decides by it (R293). */
  expect?: (action: ActionBody, ctx: CoachCtx) => boolean;
  /** Only `info` may hold AI; `coachDisplay` ignores this for `act` (R314). */
  holdAi?: boolean;
  final?: boolean;
  /** Retire if its player turn passes before it shows (R82). */
  turnBound?: boolean;
};

export type CoachTip = {
  id: string;
  title: string;
  text: Text;
  anchor?: AnchorOf;
  when: (ctx: CoachCtx) => boolean;
  holdAi?: boolean;
};

export type LessonScript = {
  lessonId: string;
  steps: readonly CoachStep[];
  tips: readonly CoachTip[];
};

export type StepOutcome = "done" | "moot" | "expired";

export type CoachState = {
  index: number;
  since: PlayerView | null;
  turnsOnStep: number;
  currentFrom: number | null;
  outcomes: Readonly<Record<string, StepOutcome>>;
  tipsSeen: readonly string[];
  tipQueue: readonly string[];
  lastTurn: number | null;
  finished: boolean;
};

export const COACH_START: CoachState = {
  index: 0,
  since: null,
  turnsOnStep: 0,
  currentFrom: null,
  outcomes: {},
  tipsSeen: [],
  tipQueue: [],
  lastTurn: null,
  finished: false,
};

export function textOf(text: Text, ctx: CoachCtx): string {
  return typeof text === "function" ? text(ctx) : text;
}

export function anchorOf(anchor: AnchorOf | undefined, ctx: CoachCtx): CoachAnchor | null {
  if (anchor === undefined || anchor === null) return null;
  return typeof anchor === "function" ? anchor(ctx) : anchor;
}

/** A predicate that throws is a lesson bug; the coach reads it as "no" rather than stopping the game. */
function safely(test: () => boolean): boolean {
  try {
    return test();
  } catch {
    return false;
  }
}

function retire(state: CoachState, step: CoachStep, outcome: StepOutcome): CoachState {
  return {
    ...state,
    index: state.index + 1,
    since: null,
    turnsOnStep: 0,
    currentFrom: null,
    outcomes: { ...state.outcomes, [step.id]: outcome },
  };
}

/** Retire waiting steps as moot before showing ones as done (R82). */
function settleSteps(script: LessonScript, start: CoachState, ctx: CoachCtx): CoachState {
  let state = start;
  // Each pass stops or retires a step, bounding the loop by `steps.length`.
  for (;;) {
    const step = script.steps[state.index];
    if (step === undefined) return state.since === null ? state : { ...state, since: null };
    if (state.currentFrom === null) state = { ...state, currentFrom: ctx.view.turn };

    const isMoot = (since: PlayerView | null): boolean =>
      step.moot !== undefined && safely(() => step.moot?.(ctx, since) === true);
    const isDone = (since: PlayerView): boolean =>
      step.done !== undefined && safely(() => step.done?.(ctx, since) === true);

    if (state.since === null) {
      const turnPassed = step.turnBound === true && ctx.view.turn !== state.currentFrom;
      if (turnPassed || isMoot(null)) {
        state = retire(state, step, "moot");
        continue;
      }
      if (step.when !== undefined && !safely(() => step.when?.(ctx) === true)) return state;
      state = { ...state, since: ctx.view };
    }

    const since = state.since ?? ctx.view;
    if (isDone(since)) {
      state = retire(state, step, "done");
      continue;
    }
    if (isMoot(since)) {
      state = retire(state, step, "moot");
      continue;
    }
    return state;
  }
}

function ownTurnStarted(state: CoachState, view: PlayerView): boolean {
  return state.lastTurn !== null && view.turn !== state.lastTurn && view.active === view.viewer;
}

/** Read every page snapshot in order, including the first. */
export function coachObserve(script: LessonScript, current: CoachState, ctx: CoachCtx): CoachState {
  if (current.finished) return current;
  if (ctx.view.result !== null) {
    return { ...current, finished: true, since: null, tipQueue: [], lastTurn: ctx.view.turn };
  }

  let state: CoachState = current;

  if (ownTurnStarted(state, ctx.view)) {
    const turns = state.turnsOnStep + 1;
    const step = script.steps[state.index];
    if (step !== undefined && step.final !== true && turns > TUTORIAL_STEP_TURNS_MAX) {
      state = retire(state, step, "expired");
    } else {
      state = { ...state, turnsOnStep: turns };
    }
  }
  state = { ...state, lastTurn: ctx.view.turn };

  const queued = new Set([...state.tipsSeen, ...state.tipQueue]);
  const triggered = script.tips.filter((tip) => !queued.has(tip.id) && safely(() => tip.when(ctx)));
  if (triggered.length > 0) state = { ...state, tipQueue: [...state.tipQueue, ...triggered.map((tip) => tip.id)] };

  return settleSteps(script, state, ctx);
}

function dropTip(state: CoachState): CoachState {
  const [shown, ...rest] = state.tipQueue;
  if (shown === undefined) return state;
  return { ...state, tipQueue: rest, tipsSeen: [...state.tipsSeen, shown] };
}

export function coachAck(script: LessonScript, state: CoachState, ctx: CoachCtx): CoachState {
  if (state.finished) return state;
  if (state.tipQueue.length > 0) return dropTip(state);
  const step = script.steps[state.index];
  if (step === undefined || state.since === null || step.kind !== "info") return state;
  return settleSteps(script, retire(state, step, "done"), ctx);
}

export type CoachDisplay =
  | { mode: "finished" }
  | { mode: "waiting"; stepNumber: number; stepCount: number; aiToAct: boolean }
  | {
      mode: "tip" | "step";
      id: string;
      title: string;
      text: string;
      anchor: CoachAnchor | null;
      ack: boolean;
      /** Hold the AI while this shows: only ever with `ack`, so "Got it" always lets go (R314). */
      holdAi: boolean;
      stepNumber: number;
      stepCount: number;
    };

export function coachDisplay(script: LessonScript, state: CoachState, ctx: CoachCtx): CoachDisplay {
  if (state.finished || ctx.view.result !== null) return { mode: "finished" };
  const stepCount = script.steps.length;
  const stepNumber = Math.min(state.index + 1, stepCount);

  const tipId = state.tipQueue[0];
  const tip = tipId === undefined ? undefined : script.tips.find((candidate) => candidate.id === tipId);
  if (tip !== undefined) {
    return {
      mode: "tip",
      id: tip.id,
      title: tip.title,
      text: textOf(tip.text, ctx),
      anchor: anchorOf(tip.anchor, ctx),
      ack: true,
      holdAi: tip.holdAi === true,
      stepNumber,
      stepCount,
    };
  }

  const step = script.steps[state.index];
  if (step === undefined) return { mode: "finished" };
  if (state.since === null) return { mode: "waiting", stepNumber, stepCount, aiToAct: ctx.aiToAct };
  return {
    mode: "step",
    id: step.id,
    title: step.title,
    text: textOf(step.text, ctx),
    anchor: anchorOf(step.anchor, ctx),
    ack: step.kind === "info",
    holdAi: step.kind === "info" && step.holdAi === true,
    stepNumber,
    stepCount,
  };
}

/** The displayed step for lesson tests. */
export function activeStep(script: LessonScript, state: CoachState): CoachStep | null {
  if (state.finished || state.since === null || state.tipQueue.length > 0) return null;
  return script.steps[state.index] ?? null;
}
