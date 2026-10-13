// Test-only full-lesson runner on the real practice core; its coach reads browser-equivalent
// snapshots (R293). A frozen clock and fixed node budget make replays deterministic. Policies cover
// coached, coached-passive (spec 22), autonomous, and robust random play.

import { AI_GATE_BUDGET, type SearchBudget } from "@jackioh/ai";
import { createRng } from "@jackioh/engine";
import type { Action, ActionBody, PlayerId, PlayerView } from "@jackioh/shared";

import { newEventsSince } from "../game/animations.ts";
import { createPracticeCore, type PracticeCore } from "../practice/core.ts";
import type { PracticeDebug, PracticeRequestBody, PracticeResponse, PracticeSnapshot } from "../practice/protocol.ts";
import {
  COACH_START,
  activeStep,
  coachAck,
  coachDisplay,
  coachObserve,
  type CoachCtx,
  type CoachState,
  type LessonScript,
  type StepOutcome,
} from "./coach.ts";
import { lessonById, type TutorialLesson } from "./lessons.ts";
import { scriptFor } from "./scripts/index.ts";
import { heroTargetId, myMain, mulliganOpen } from "./steps.ts";

const LESSON_REQUEST_CAP = 1500;

/** Random games need a cheaper search budget to keep the robustness run affordable. */
export const RANDOM_POLICY_AI_BUDGET: SearchBudget = {
  nodes: 60,
  lethalNodes: 20,
  determinizations: 1,
  beamWidth: 2,
  rootBranching: 8,
  branching: 3,
  maxDepth: 4,
  finalists: 1,
};

/** Full-lesson timeout; AI search dominates its runtime. */
export const LESSON_GAME_TIMEOUT_MS = 180_000;
/** Per-game budget for multi-game gate tests. */
export const LESSON_GAME_MS = 90_000;
/** Per-game budget for multi-game random-policy tests. */
export const RANDOM_GAME_MS = 45_000;

export type LessonPolicy = "coach" | "coach-passive" | "autopilot" | "random";

export type LessonRun = {
  lesson: TutorialLesson;
  script: LessonScript;
  winner: PlayerId | "draw" | null;
  humanSeat: PlayerId;
  humanTurns: number;
  /** How each step ended; a step missing here was never retired (the game ended on it). */
  outcomes: Readonly<Record<string, StepOutcome>>;
  shown: { id: string; turn: number }[];
  tips: string[];
  coach: CoachState;
  debug: PracticeDebug;
  humanActions: { action: ActionBody; refused: string | null; byCoach: boolean }[];
  view: PlayerView;
};

type Options = {
  policy?: LessonPolicy;
  seed?: string;
  policySeed?: string;
  budget?: SearchBudget;
};

function isPlay(action: ActionBody): action is Extract<ActionBody, { type: "play" }> {
  return action.type === "play";
}

function hitsOwnSide(view: PlayerView, action: Extract<ActionBody, { type: "play" }>): boolean {
  const own = new Set<string>();
  for (const unit of view.you.units) if (unit !== null) own.add(unit.instanceId);
  for (const card of view.you.backrow) if (card !== null && !card.faceDown) own.add(card.instanceId);
  return (action.targets ?? []).some(
    (target) =>
      (target.pick === "instance" && own.has(target.instanceId)) ||
      (target.pick === "hero" && target.player === view.viewer) ||
      (target.pick === "zone" && target.player === view.viewer && target.row === "backrow"),
  );
}

/** Beginner fallback: keep, resolve prompts, play safely, then attack and end. */
export function autopilot(ctx: CoachCtx): ActionBody | null {
  const { view, legal } = ctx;
  if (mulliganOpen(view)) {
    const keepAll = legal.filter((action) => action.type === "mulligan").sort((a, b) =>
      a.type === "mulligan" && b.type === "mulligan" ? b.keep.length - a.keep.length : 0,
    );
    return keepAll[0] ?? null;
  }
  if (view.pending !== null && view.pending.forYou) {
    return legal.find((action) => action.type === "answer") ?? legal.find((action) => action.type === "mulligan") ?? null;
  }
  if (!myMain(ctx)) return null;

  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  const costOf = (instanceId: string): number => hand.find((card) => card.instanceId === instanceId)?.cost ?? 0;
  const plays = legal.filter(isPlay).filter((action) => !hitsOwnSide(view, action));
  plays.sort((a, b) => costOf(b.instanceId) - costOf(a.instanceId));
  const play = plays[0];
  if (play !== undefined) return play;

  const hero = heroTargetId(view, "opponent");
  const attacks = legal.filter((action): action is Extract<ActionBody, { type: "attack" }> => action.type === "attack");
  const face = attacks.find((action) => action.targetId === hero);
  if (face !== undefined) return face;
  if (attacks[0] !== undefined) return attacks[0];

  return legal.find((action) => action.type === "endTurn") ?? null;
}

/** Coach-passive fallback never plays a card independently. */
export function passive(ctx: CoachCtx): ActionBody | null {
  const { view, legal } = ctx;
  if (mulliganOpen(view) || (view.pending !== null && view.pending.forYou)) return autopilot(ctx);
  if (!myMain(ctx)) return null;
  const hero = heroTargetId(view, "opponent");
  const attacks = legal.filter((action) => action.type === "attack");
  return (
    attacks.find((action) => action.type === "attack" && action.targetId === hero) ??
    attacks[0] ??
    legal.find((action) => action.type === "endTurn") ??
    null
  );
}

function randomPolicy(ctx: CoachCtx, pick: (n: number) => number): ActionBody | null {
  const choices = ctx.legal.filter((action) => action.type !== "concede" && action.type !== "offerDraw" && action.type !== "answerDraw");
  if (choices.length === 0) return null;
  return choices[pick(choices.length)] ?? null;
}

function send(core: PracticeCore, counter: { n: number }, body: PracticeRequestBody): PracticeResponse {
  counter.n += 1;
  return core.handle({ ...body, id: counter.n } as Parameters<PracticeCore["handle"]>[0]);
}

function snapshotOf(response: PracticeResponse): PracticeSnapshot {
  if (response.type === "started" || response.type === "snapshot") return response.snapshot;
  throw new Error(`the practice core answered ${response.type}: ${response.type === "failed" ? response.message : ""}`);
}

export function playLesson(lessonId: string, options: Options = {}): LessonRun {
  const lesson = lessonById(lessonId);
  const script = scriptFor(lessonId);
  if (lesson === undefined || script === undefined) throw new Error(`no lesson "${lessonId}"`);
  const policy = options.policy ?? "coach";
  const rng = createRng(options.policySeed ?? `${lessonId}:policy`);

  const budget = options.budget ?? (policy === "random" ? RANDOM_POLICY_AI_BUDGET : AI_GATE_BUDGET);
  const core = createPracticeCore({ now: () => 0, dev: true, budget });
  const counter = { n: 0 };
  const startedResponse = send(core, counter, {
      type: "start",
      config: {
        seed: options.seed ?? lesson.seed,
        difficulty: "easy",
        humanSeat: lesson.humanSeat,
        deck: { kind: "random" },
        lesson: lesson.id,
      },
    });
  let snapshot = snapshotOf(startedResponse);
  const defs = startedResponse.type === "started" ? startedResponse.defs : {};

  let coach: CoachState = COACH_START;
  let previous: PlayerView | null = null;
  const shown: { id: string; turn: number }[] = [];
  const tips: string[] = [];
  const humanActions: LessonRun["humanActions"] = [];
  let humanTurns = 0;
  let lastTurn = -1;

  const ctxOf = (snap: PracticeSnapshot): CoachCtx => ({
    view: snap.view,
    legal: snap.legal,
    fresh: previous === null ? [] : newEventsSince(previous.events, snap.view.events),
    aiToAct: snap.aiToAct,
    nameOf: (defId) => snap.view.defs?.[defId]?.name ?? defs[defId]?.name,
  });

  for (;;) {
    if (counter.n > LESSON_REQUEST_CAP) throw new Error(`lesson "${lessonId}" did not end within ${LESSON_REQUEST_CAP} requests`);
    const ctx = ctxOf(snapshot);
    previous = snapshot.view;
    coach = coachObserve(script, coach, ctx);
    if (snapshot.view.turn !== lastTurn) {
      lastTurn = snapshot.view.turn;
      if (snapshot.view.active === snapshot.view.viewer && snapshot.view.phase === "main") humanTurns += 1;
    }

    let display = coachDisplay(script, coach, ctx);
    while (display.mode === "tip" || (display.mode === "step" && display.ack)) {
      const id = display.id;
      if (display.mode === "tip") tips.push(id);
      else if (!shown.some((entry) => entry.id === id)) shown.push({ id, turn: snapshot.view.turn });
      coach = coachAck(script, coach, ctx);
      display = coachDisplay(script, coach, ctx);
    }
    if (display.mode === "step") {
      const id = display.id;
      if (!shown.some((entry) => entry.id === id)) shown.push({ id, turn: snapshot.view.turn });
    }

    if (snapshot.view.result !== null) break;

    if (snapshot.aiToAct) {
      snapshot = snapshotOf(send(core, counter, { type: "aiStep" }));
      continue;
    }

    let action: ActionBody | null = null;
    let byCoach = false;
    if (policy === "coach") {
      const step = activeStep(script, coach);
      if (step?.kind === "act" && step.expect !== undefined) {
        const expected = step.expect;
        action = snapshot.legal.find((candidate) => expected(candidate, ctx)) ?? null;
        byCoach = action !== null;
      }
      action ??= autopilot(ctx);
    } else if (policy === "coach-passive") {
      const step = activeStep(script, coach);
      if (step?.kind === "act" && step.expect !== undefined) {
        const expected = step.expect;
        action = snapshot.legal.find((candidate) => expected(candidate, ctx)) ?? null;
        byCoach = action !== null;
      }
      action ??= passive(ctx);
    } else if (policy === "autopilot") {
      action = autopilot(ctx);
    } else {
      action = randomPolicy(ctx, (n) => rng.int(n));
    }
    if (action === null) {
      throw new Error(`lesson "${lessonId}" stalled on turn ${String(snapshot.view.turn)}: no human action and no AI step`);
    }
    const next = snapshotOf(send(core, counter, { type: "act", action }));
    humanActions.push({ action, refused: next.error, byCoach });
    snapshot = next;
  }

  const debugResponse = send(core, counter, { type: "debug" });
  if (debugResponse.type !== "debug") throw new Error("the dev core refused debug");
  return {
    lesson,
    script,
    winner: snapshot.view.result?.winner ?? null,
    humanSeat: lesson.humanSeat,
    humanTurns,
    outcomes: coach.outcomes,
    shown,
    tips,
    coach,
    debug: debugResponse.debug,
    humanActions,
    view: snapshot.view,
  };
}

export function humanLog(run: LessonRun): Action[] {
  return run.debug.log.filter((action) => action.playerId === run.humanSeat);
}
