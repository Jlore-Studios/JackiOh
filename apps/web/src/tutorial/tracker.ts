// Observe each snapshot before controller start so no event is missed or coalesced (SPEC §9.10, R292).
// Apply AI holds synchronously when a "Got it" display appears (R314); this tracker never sends actions (Rule 7).

import type { ActionBody } from "@jackioh/shared";

import { newEventsSince } from "../game/animations.ts";
import type { PracticeController } from "../practice/controller.ts";
import type { PracticeSnapshot } from "../practice/protocol.ts";
import {
  COACH_START,
  activeStep,
  coachAck,
  coachDisplay,
  coachObserve,
  type CoachCtx,
  type CoachDisplay,
  type CoachState,
  type LessonScript,
} from "./coach.ts";
import { myMain } from "./steps.ts";
import { coachTargets } from "./targets.ts";

export type CoachSource = Pick<PracticeController, "getState" | "subscribe" | "setHold">;

export const COACH_HOLD = "coach";

export type CoachView = {
  coach: CoachState;
  ctx: CoachCtx | null;
  display: CoachDisplay;
  targets: readonly string[];
  aiBusy: boolean;
  yourMove: boolean;
};

export type CoachTracker = {
  readonly script: LessonScript;
  getState(): CoachView;
  subscribe(fn: () => void): () => void;
  /** An expected display key prevents an unseen bubble from acknowledging its successor. */
  ack(expected?: string): void;
  dispose(): void;
};

const FINISHED: CoachDisplay = { mode: "finished" };

export function displayKey(display: CoachDisplay): string {
  return display.mode === "tip" || display.mode === "step" ? `${display.mode}:${display.id}` : display.mode;
}

function viewOf(coach: CoachState, ctx: CoachCtx | null, script: LessonScript): CoachView {
  if (ctx === null) return { coach, ctx, display: FINISHED, targets: [], aiBusy: false, yourMove: false };
  const display = coachDisplay(script, coach, ctx);
  const anchor = display.mode === "tip" || display.mode === "step" ? display.anchor : null;
  const view = ctx.view;
  return {
    coach,
    ctx,
    display,
    targets: coachTargets(anchor, view),
    aiBusy: ctx.aiToAct || (view.result === null && view.active !== view.viewer),
    yourMove: myMain(ctx),
  };
}

function holdsAi(display: CoachDisplay): boolean {
  return (display.mode === "tip" || display.mode === "step") && display.holdAi;
}

export function createCoachTracker(source: CoachSource, script: LessonScript): CoachTracker {
  let coach: CoachState = COACH_START;
  let ctx: CoachCtx | null = null;
  let lastSnapshot: PracticeSnapshot | null = null;
  let state: CoachView = viewOf(coach, ctx, script);
  let held = false;
  let disposed = false;
  const listeners = new Set<() => void>();

  function hold(next: boolean): void {
    if (next === held) return;
    held = next;
    source.setHold(COACH_HOLD, next);
  }

  function publish(): void {
    state = viewOf(coach, ctx, script);
    hold(!disposed && holdsAi(state.display));
    for (const fn of [...listeners]) fn();
  }

  function observe(): void {
    if (disposed) return;
    const snapshot = source.getState().snapshot;
    if (snapshot === null || snapshot === lastSnapshot) return;
    const previous = lastSnapshot?.view ?? null;
    lastSnapshot = snapshot;
    const defs = source.getState().defs;
    ctx = {
      view: snapshot.view,
      legal: snapshot.legal,
      fresh: previous === null ? [] : newEventsSince(previous.events, snapshot.view.events),
      aiToAct: snapshot.aiToAct,
      // §5.1: the catalog is public and arrives with `started`.
      nameOf: (defId) => snapshot.view.defs?.[defId]?.name ?? defs?.[defId]?.name,
    };
    coach = coachObserve(script, coach, ctx);
    publish();
  }

  const unsubscribe = source.subscribe(observe);
  observe();

  return {
    script,
    getState: () => state,
    subscribe(fn) {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },
    ack(expected) {
      if (disposed || ctx === null) return;
      if (expected !== undefined && expected !== displayKey(state.display)) return;
      const next = coachAck(script, coach, ctx);
      if (next === coach) return;
      coach = next;
      publish();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      unsubscribe();
      hold(false);
      listeners.clear();
    },
  };
}

/** Dev-only aid: returns an expected action but never performs it; e2e uses the real UI. */
export function suggestedAction(script: LessonScript, state: CoachView): ActionBody | null {
  const ctx = state.ctx;
  if (ctx === null) return null;
  const step = activeStep(script, state.coach);
  const expect = step?.expect;
  if (expect === undefined) return null;
  return (
    ctx.legal.find((action) => {
      try {
        return expect(action, ctx);
      } catch {
        return false;
      }
    }) ?? null
  );
}

export function silentScript(lessonId: string): LessonScript {
  return { lessonId, steps: [], tips: [] };
}
