// Test-only full-lesson runner on the real practice core; its coach reads browser-equivalent
// snapshots (R293). A frozen clock and fixed node budget make replays deterministic. Policies cover
// coached, coached-passive (spec 22), autonomous, robust random play, and the reckless player who
// attacks into everything and plays nothing protective (R2205001), and the idle player who only ends
// the turn (R2201000).

import { AI_GATE_BUDGET, type SearchBudget } from "@jackioh/ai";
import { createRng } from "@jackioh/engine";
import type { ActionBody, CardDef, PlayerId, PlayerView } from "@jackioh/shared";

import { newEventsSince } from "../game/animations.ts";
import { createPracticeCore, type PracticeCore } from "../practice/core.ts";
import type { PracticeDebug, PracticeRequestBody, PracticeResponse, PracticeSnapshot } from "../practice/protocol.ts";
import {
  activeStep,
  coachAck,
  coachDisplay,
  coachObserve,
  coachStart,
  isBubble,
  type CoachCtx,
  type CoachState,
  type LessonScript,
  type StepOutcome,
} from "./coach.ts";
import { aimsAtOwnSide, bestPlay } from "./advice.ts";
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

export type LessonPolicy = "coach" | "coach-passive" | "autopilot" | "random" | "reckless" | "idle";

/** Every policy a lesson must be won under (R2205001; `idle`, R2201000). */
export const LESSON_POLICIES: readonly LessonPolicy[] = ["coach", "coach-passive", "autopilot", "random", "reckless", "idle"];

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
  /** R2205000: the discovery bubbles shown, in order. */
  discoveries: string[];
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
  /** A fixture script in place of the lesson's own (the coach framework's tests). */
  script?: LessonScript;
  /** What the player's Field Guide holds as the lesson starts (R2205002). */
  known?: readonly string[];
};

function isPlay(action: ActionBody): action is Extract<ActionBody, { type: "play" }> {
  return action.type === "play";
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

  const play = bestPlay(ctx);
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

const PROTECTIVE_KEYWORDS: readonly string[] = ["Taunt", "Divine Shield", "Armor"];

/** A card the reckless player never plays: a Trap, a Field Trap, or a unit that guards. */
function protective(def: CardDef | undefined): boolean {
  if (def === undefined) return false;
  if (def.type === "Trap" || def.type === "Field Trap") return true;
  return def.type === "Unit" && def.base.keywords.some((keyword) => PROTECTIVE_KEYWORDS.includes(keyword.kind));
}

/**
 * R2205001: the reckless player keeps its hand, answers each prompt with its first option, attacks
 * with everything into the enemy unit with the most Attack (the hero when no unit can be hit), then
 * plays its cheapest card that guards nothing and aims at nothing of its own, and ends the turn. It
 * never switches a unit to Defense Position.
 */
export function reckless(ctx: CoachCtx): ActionBody | null {
  const { view, legal } = ctx;
  if (mulliganOpen(view) || (view.pending !== null && view.pending.forYou)) return autopilot(ctx);
  if (!myMain(ctx)) return null;

  const attacks = legal.filter((action): action is Extract<ActionBody, { type: "attack" }> => action.type === "attack");
  const enemyAttack = (targetId: string): number =>
    view.opponent.units.find((unit) => unit !== null && unit.instanceId === targetId)?.attack ?? -1;
  attacks.sort((a, b) => enemyAttack(b.targetId) - enemyAttack(a.targetId));
  const attack = attacks[0];
  if (attack !== undefined) return attack;

  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  const cardOf = (instanceId: string) => hand.find((card) => card.instanceId === instanceId);
  const plays = legal
    .filter(isPlay)
    .filter((action) => !aimsAtOwnSide(ctx, action))
    .filter((action) => {
      const card = cardOf(action.instanceId);
      return card === undefined || !protective(ctx.defOf?.(card.defId));
    });
  plays.sort((a, b) => (cardOf(a.instanceId)?.cost ?? 0) - (cardOf(b.instanceId)?.cost ?? 0));
  const play = plays[0];
  if (play !== undefined) return play;

  return legal.find((action) => action.type === "endTurn") ?? null;
}

/**
 * R2201000: the idle player keeps its hand, answers each prompt with its first option, and otherwise
 * only ends its turn: it never plays a card, attacks or switches a position, so the lesson is won
 * by the opponent's deck running out and fatigue.
 */
export function idle(ctx: CoachCtx): ActionBody | null {
  const { view, legal } = ctx;
  if (mulliganOpen(view) || (view.pending !== null && view.pending.forYou)) return autopilot(ctx);
  if (!myMain(ctx)) return null;
  return legal.find((action) => action.type === "endTurn") ?? null;
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
  const script = options.script ?? scriptFor(lessonId);
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

  let coach: CoachState = coachStart(options.known ?? []);
  let previous: PlayerView | null = null;
  const shown: { id: string; turn: number }[] = [];
  const tips: string[] = [];
  const discoveries: string[] = [];
  const humanActions: LessonRun["humanActions"] = [];
  let humanTurns = 0;
  let lastTurn = -1;

  const ctxOf = (snap: PracticeSnapshot): CoachCtx => ({
    view: snap.view,
    legal: snap.legal,
    fresh: previous === null ? [] : newEventsSince(previous.events, snap.view.events),
    aiToAct: snap.aiToAct,
    nameOf: (defId) => snap.view.defs?.[defId]?.name ?? defs[defId]?.name,
    defOf: (defId) => snap.view.defs?.[defId] ?? defs[defId],
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
    while (isBubble(display) && display.ack) {
      const id = display.id;
      if (display.mode === "tip") tips.push(id);
      else if (display.mode === "discovery") discoveries.push(id);
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
      if ((step?.kind === "act" || step?.kind === "moment") && step.expect !== undefined) {
        const expected = step.expect;
        action = snapshot.legal.find((candidate) => expected(candidate, ctx)) ?? null;
        byCoach = action !== null;
      }
      action ??= autopilot(ctx);
    } else if (policy === "coach-passive") {
      const step = activeStep(script, coach);
      if ((step?.kind === "act" || step?.kind === "moment") && step.expect !== undefined) {
        const expected = step.expect;
        action = snapshot.legal.find((candidate) => expected(candidate, ctx)) ?? null;
        byCoach = action !== null;
      }
      action ??= passive(ctx);
    } else if (policy === "autopilot") {
      action = autopilot(ctx);
    } else if (policy === "reckless") {
      action = reckless(ctx);
    } else if (policy === "idle") {
      action = idle(ctx);
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
    discoveries,
    coach,
    debug: debugResponse.debug,
    humanActions,
    view: snapshot.view,
  };
}
