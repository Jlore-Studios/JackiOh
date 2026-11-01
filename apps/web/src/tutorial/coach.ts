// Tutorial coach tracks lesson steps and tips without deciding legality (SPEC §9.10, R292;
// CLAUDE.md rule 7). There is no skip: only ackable `info` steps or tips may hold AI (R314), non-final
// steps expire, and the HUD always offers exit. R2205000: the coach recognizes each Field Guide entry
// (discoveries.ts) from the view alone and names its rule in a bubble.

import type { ActionBody, CardDef, GameEvent, PlayerView, Row } from "@jackioh/shared";

import type { Side } from "../game/contract.ts";
import { t } from "../i18n/i18n.ts";
import { TUTORIAL_BUBBLES_PER_MOVE_MAX, TUTORIAL_MOMENT_TURNS_MAX, TUTORIAL_STEP_TURNS_MAX } from "./config.ts";
import { FIELD_GUIDE, type Discovery } from "./discoveries.ts";
import { lessonById } from "./lessons.ts";

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
  /** The public catalog's definition (§5.1, R2205000). */
  defOf?: (defId: string) => CardDef | undefined;
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

/** A fixed English line, looked up where it is shown, or one built then (R711000). */
export type Text = string | ((ctx: CoachCtx) => string);
type AnchorOf = CoachAnchor | null | ((ctx: CoachCtx) => CoachAnchor | null);

export type CoachStep = {
  /** Tests and the page's `data-coach-step` use this lesson-unique id. */
  id: string;
  title: Text;
  /** A `moment`'s question: the goal, never the click. */
  text: Text;
  anchor?: AnchorOf;
  /** `info`: "Got it" completes it. `act`: its `done` completes it. `moment`: finding what it `discovers`, or its `done`. */
  kind: "info" | "act" | "moment";
  when?: (ctx: CoachCtx) => boolean;
  /** Required for `act`; receives the activation view. */
  done?: (ctx: CoachCtx, since: PlayerView) => boolean;
  /** Retires a no-longer-sensible step without showing it. */
  moot?: (ctx: CoachCtx, since: PlayerView | null) => boolean;
  /** Lesson tests follow this legal-action predicate; the page never decides by it (R293). */
  expect?: (action: ActionBody, ctx: CoachCtx) => boolean;
  /** Only `info` may hold AI; `coachDisplay` ignores this for `act` and `moment` (R314). */
  holdAi?: boolean;
  final?: boolean;
  /** Retire if its player turn passes before it shows (R82). */
  turnBound?: boolean;
  /** `moment`: the Field Guide entries it sets up; it is done once each is found in this game. */
  discovers?: readonly string[];
  /** `moment`: the nudge, then the pointed hint, shown as the player stalls or asks (R2205000). */
  hints?: readonly Text[];
  /** `moment`: what the last hint points at. */
  hintAnchor?: AnchorOf;
  /** `moment`: re-offers an earlier lesson's mechanic, so it is moot once the Field Guide holds it (R314). */
  revisit?: boolean;
};

export type CoachTip = {
  id: string;
  title: Text;
  text: Text;
  anchor?: AnchorOf;
  when: (ctx: CoachCtx) => boolean;
  holdAi?: boolean;
};

export type LessonScript = {
  lessonId: string;
  steps: readonly CoachStep[];
  tips: readonly CoachTip[];
  /** The Field Guide entries this lesson lists; their discovery bubbles always show (R2205000). */
  teaches?: readonly string[];
  /** The entries the coach watches for: the whole Field Guide in every lesson that lists `teaches`. */
  guide?: readonly Discovery[];
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
  /** R2205000: the Field Guide entries found in this game, in the order found. */
  found: readonly string[];
  /** Discovery bubbles still to show, oldest first. */
  discoveryQueue: readonly string[];
  /** The entries the Field Guide held when the lesson started (R2205002); a lesson's own input, not the game's. */
  known: readonly string[];
  /** `turnsOnStep` when the current step showed. */
  shownAt: number;
  hintsAsked: number;
  /** Discovery bubbles queued since the player's last own move (`TUTORIAL_BUBBLES_PER_MOVE_MAX`). */
  bubblesSinceMove: number;
  /** The steps' order as the coach takes them (`bringForward`); empty is the script's. */
  order: readonly number[];
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
  found: [],
  discoveryQueue: [],
  known: [],
  shownAt: 0,
  hintsAsked: 0,
  bubblesSinceMove: 0,
  order: [],
};

/** The coach at a lesson's start, given what the player's Field Guide already holds (R2205002). */
export function coachStart(known: readonly string[]): CoachState {
  return known.length === 0 ? COACH_START : { ...COACH_START, known: [...known] };
}

export function textOf(text: Text, ctx: CoachCtx): string {
  return typeof text === "function" ? text(ctx) : t(text);
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

function stepAt(script: LessonScript, state: CoachState): CoachStep | undefined {
  return script.steps[state.order[state.index] ?? state.index];
}

/**
 * R2205000: a moment waits for its board; the first later moment in its run (to the next other step or
 * the final one) that can show now swaps places with it.
 */
function bringForward(script: LessonScript, state: CoachState, ctx: CoachCtx): CoachState | null {
  const order = state.order.length === 0 ? script.steps.map((_step, at) => at) : [...state.order];
  for (let at = state.index + 1; at < order.length; at += 1) {
    const candidate = script.steps[order[at] ?? at];
    if (candidate === undefined || candidate.kind !== "moment" || candidate.final === true) return null;
    if (state.outcomes[candidate.id] !== undefined) continue;
    if (candidate.when !== undefined && !safely(() => candidate.when?.(ctx) === true)) continue;
    const here = order[state.index] ?? state.index;
    order[state.index] = order[at] ?? at;
    order[at] = here;
    return { ...state, order, turnsOnStep: 0, currentFrom: null };
  }
  return null;
}

function guideOf(script: LessonScript): readonly Discovery[] {
  if (script.teaches === undefined) return [];
  return script.guide ?? FIELD_GUIDE;
}

function retire(state: CoachState, step: CoachStep, outcome: StepOutcome): CoachState {
  return {
    ...state,
    index: state.index + 1,
    since: null,
    turnsOnStep: 0,
    currentFrom: null,
    shownAt: 0,
    hintsAsked: 0,
    outcomes: { ...state.outcomes, [step.id]: outcome },
  };
}

function discovered(step: CoachStep, state: CoachState): boolean {
  const wanted = step.discovers ?? [];
  return step.kind === "moment" && wanted.length > 0 && wanted.every((id) => state.found.includes(id));
}

/** A revisit offers what the player has not met yet; once the Field Guide holds it, it is moot. */
function alreadyKnown(step: CoachStep, state: CoachState): boolean {
  const wanted = step.discovers ?? [];
  return step.revisit === true && wanted.every((id) => state.known.includes(id) || state.found.includes(id));
}

/** Retire waiting steps as moot before showing ones as done (R82). */
function settleSteps(script: LessonScript, start: CoachState, ctx: CoachCtx): CoachState {
  let state = start;
  // Each pass stops or retires a step, bounding the loop by `steps.length`.
  for (;;) {
    const step = stepAt(script, state);
    if (step === undefined) return state.since === null ? state : { ...state, since: null };
    if (state.currentFrom === null) state = { ...state, currentFrom: ctx.view.turn };

    const isMoot = (since: PlayerView | null): boolean =>
      step.moot !== undefined && safely(() => step.moot?.(ctx, since) === true);
    const isDone = (since: PlayerView): boolean =>
      discovered(step, state) || (step.done !== undefined && safely(() => step.done?.(ctx, since) === true));

    if (state.since === null) {
      const turnPassed = step.turnBound === true && ctx.view.turn !== state.currentFrom;
      if (turnPassed || isMoot(null) || alreadyKnown(step, state)) {
        state = retire(state, step, "moot");
        continue;
      }
      if (discovered(step, state)) {
        state = retire(state, step, "done");
        continue;
      }
      if (step.when !== undefined && !safely(() => step.when?.(ctx) === true)) {
        const ahead = step.kind === "moment" && step.final !== true ? bringForward(script, state, ctx) : null;
        if (ahead === null) return state;
        state = ahead;
        continue;
      }
      state = { ...state, since: ctx.view, shownAt: state.turnsOnStep };
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
    // A shown moment whose board has gone (its card spent, its Trap set and waiting) steps back for a
    // later moment whose board is here; it shows again when its own comes back, its find counted as ever.
    if (step.kind === "moment" && step.final !== true && step.when !== undefined && !safely(() => step.when?.(ctx) === true)) {
      const ahead = bringForward(script, { ...state, since: null, shownAt: 0, hintsAsked: 0 }, ctx);
      if (ahead !== null) {
        state = ahead;
        continue;
      }
    }
    return state;
  }
}

function ownTurnStarted(state: CoachState, view: PlayerView): boolean {
  return state.lastTurn !== null && view.turn !== state.lastTurn && view.active === view.viewer;
}

/** A shown moment has `TUTORIAL_MOMENT_TURNS_MAX` of the player's turns, its hints included; anything else `TUTORIAL_STEP_TURNS_MAX`. */
function overdue(step: CoachStep, state: CoachState, turns: number): boolean {
  if (step.final === true) return false;
  if (step.kind === "moment" && state.since !== null) return turns - state.shownAt > TUTORIAL_MOMENT_TURNS_MAX;
  return turns > TUTORIAL_STEP_TURNS_MAX;
}

/** The player's own move shows in the fresh events (a forced attack is not one). */
function playerMoved(ctx: CoachCtx): boolean {
  const { view } = ctx;
  const own = new Set([
    ...view.you.units.flatMap((unit) => (unit === null ? [] : [unit.instanceId])),
    ...view.you.graveyard.map((card) => card.instanceId),
  ]);
  return ctx.fresh.some((event) => {
    switch (event.type) {
      case "cardPlayed":
      case "turnEnded":
      case "promptAnswered":
        return event.player === view.viewer;
      case "attackDeclared":
        return !event.forced && own.has(event.attackerId);
      case "positionSwitched":
        return own.has(event.instanceId);
      default:
        return false;
    }
  });
}

/**
 * R2205000: a bubble for each new find the lesson lists or the player has not met, at most
 * `TUTORIAL_BUBBLES_PER_MOVE_MAX` "Got it"s (tips and a shown `info` step counted) between two of the player's moves.
 * R2434001: an entry a later lesson holds never bubbles, so no lesson teaches what a later one does.
 */
function discover(script: LessonScript, current: CoachState, ctx: CoachCtx): CoachState {
  const state = playerMoved(ctx) && current.bubblesSinceMove > 0 ? { ...current, bubblesSinceMove: 0 } : current;
  const fresh = guideOf(script).filter((entry) => !state.found.includes(entry.id) && safely(() => entry.found(ctx)));
  if (fresh.length === 0) return state;
  const teaches = script.teaches ?? [];
  const here = lessonById(script.lessonId)?.number ?? Infinity;
  const due = (entry: Discovery): boolean => (lessonById(entry.lesson)?.number ?? Infinity) <= here;
  const wanted = fresh
    .filter((entry) => teaches.includes(entry.id) || (!state.known.includes(entry.id) && due(entry)))
    .map((entry) => entry.id);
  const info = state.since !== null && stepAt(script, state)?.kind === "info" ? 1 : 0;
  const room = Math.max(0, TUTORIAL_BUBBLES_PER_MOVE_MAX - state.bubblesSinceMove - state.tipQueue.length - info);
  const bubbles = wanted.slice(0, room);
  return {
    ...state,
    found: [...state.found, ...fresh.map((entry) => entry.id)],
    discoveryQueue: [...state.discoveryQueue, ...bubbles],
    bubblesSinceMove: state.bubblesSinceMove + bubbles.length,
  };
}

/** Read every page snapshot in order, including the first. */
export function coachObserve(script: LessonScript, current: CoachState, ctx: CoachCtx): CoachState {
  if (current.finished) return current;
  let state: CoachState = discover(script, current, ctx);
  if (ctx.view.result !== null) {
    return { ...state, finished: true, since: null, tipQueue: [], discoveryQueue: [], lastTurn: ctx.view.turn };
  }

  if (ownTurnStarted(state, ctx.view)) {
    const turns = state.turnsOnStep + 1;
    const step = stepAt(script, state);
    if (step !== undefined && overdue(step, state, turns)) {
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
  if (state.discoveryQueue.length > 0) return { ...state, discoveryQueue: state.discoveryQueue.slice(1) };
  if (state.tipQueue.length > 0) return dropTip(state);
  const step = stepAt(script, state);
  if (step === undefined || state.since === null || step.kind !== "info") return state;
  return settleSteps(script, retire(state, step, "done"), ctx);
}

/** The shown moment's hint level: 0 is its question alone, the last its pointed hint (R2205000). */
export function hintLevel(script: LessonScript, state: CoachState): number {
  const step = stepAt(script, state);
  if (step?.kind !== "moment" || state.since === null) return 0;
  const stalled = state.turnsOnStep - state.shownAt;
  return Math.min(stalled + state.hintsAsked, step.hints?.length ?? 0);
}

/** The player asked for a hint: the shown moment's next one, if it has one left. */
export function coachHint(script: LessonScript, state: CoachState): CoachState {
  if (state.finished || state.discoveryQueue.length > 0 || state.tipQueue.length > 0) return state;
  const step = stepAt(script, state);
  if (step?.kind !== "moment" || state.since === null) return state;
  if (hintLevel(script, state) >= (step.hints?.length ?? 0)) return state;
  return { ...state, hintsAsked: state.hintsAsked + 1 };
}

/** The entries this lesson lists that the game has not shown found (R314: the win names them). */
export function missedOf(script: LessonScript, state: CoachState): string[] {
  return (script.teaches ?? []).filter((id) => !state.found.includes(id));
}

type Bubble = {
  mode: "tip" | "step" | "discovery";
  id: string;
  title: string;
  text: string;
  anchor: CoachAnchor | null;
  ack: boolean;
  /** Hold the AI while this shows: only ever with `ack`, so "Got it" always lets go (R314). */
  holdAi: boolean;
  /** A moment's hint at its current level; null below the first. */
  hint: string | null;
  canHint: boolean;
  stepNumber: number;
  stepCount: number;
};

export type CoachDisplay =
  | { mode: "finished" }
  | { mode: "waiting"; stepNumber: number; stepCount: number; aiToAct: boolean }
  | Bubble;

/** An entry's rule as a sentence of its own: it is written to follow "You found {name}:". */
export function ruleSentence(entry: Discovery): string {
  const rule = t(entry.rule);
  return rule.charAt(0).toUpperCase() + rule.slice(1);
}

export function isBubble(display: CoachDisplay): display is Bubble {
  return display.mode === "tip" || display.mode === "step" || display.mode === "discovery";
}

export function coachDisplay(script: LessonScript, state: CoachState, ctx: CoachCtx): CoachDisplay {
  if (state.finished || ctx.view.result !== null) return { mode: "finished" };
  const stepCount = script.steps.length;
  const stepNumber = Math.min(state.index + 1, stepCount);
  const plain = { hint: null, canHint: false, stepNumber, stepCount };

  const foundId = state.discoveryQueue[0];
  const entry = foundId === undefined ? undefined : guideOf(script).find((candidate) => candidate.id === foundId);
  if (entry !== undefined) {
    return {
      mode: "discovery",
      id: entry.id,
      title: t("You found {name}", { name: t(entry.name) }),
      text: ruleSentence(entry),
      anchor: null,
      ack: true,
      holdAi: false,
      ...plain,
    };
  }

  const tipId = state.tipQueue[0];
  const tip = tipId === undefined ? undefined : script.tips.find((candidate) => candidate.id === tipId);
  if (tip !== undefined) {
    return {
      mode: "tip",
      id: tip.id,
      title: textOf(tip.title, ctx),
      text: textOf(tip.text, ctx),
      anchor: anchorOf(tip.anchor, ctx),
      ack: true,
      holdAi: tip.holdAi === true,
      ...plain,
    };
  }

  const step = stepAt(script, state);
  if (step === undefined) return { mode: "finished" };
  if (state.since === null) return { mode: "waiting", stepNumber, stepCount, aiToAct: ctx.aiToAct };
  const hints = step.kind === "moment" ? (step.hints ?? []) : [];
  const level = hintLevel(script, state);
  const hint = level === 0 ? undefined : hints[level - 1];
  const pointed = hints.length > 0 && level === hints.length;
  return {
    mode: "step",
    id: step.id,
    title: textOf(step.title, ctx),
    text: textOf(step.text, ctx),
    anchor: anchorOf(pointed ? (step.hintAnchor ?? step.anchor) : step.anchor, ctx),
    ack: step.kind === "info",
    holdAi: step.kind === "info" && step.holdAi === true,
    hint: hint === undefined ? null : textOf(hint, ctx),
    canHint: level < hints.length,
    stepNumber,
    stepCount,
  };
}

/** The displayed step for lesson tests. */
export function activeStep(script: LessonScript, state: CoachState): CoachStep | null {
  if (state.finished || state.since === null || state.tipQueue.length > 0 || state.discoveryQueue.length > 0) return null;
  return stepAt(script, state) ?? null;
}
