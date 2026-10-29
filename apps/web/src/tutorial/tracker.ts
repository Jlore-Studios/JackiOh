// Observe each snapshot before controller start so no event is missed or coalesced (SPEC §9.10, R292).
// Apply AI holds synchronously when a "Got it" display appears (R314); this tracker never sends actions (Rule 7).

import type { ActionBody } from "@jackioh/shared";

import { newEventsSince } from "../game/animations.ts";
import { subscribeLocale } from "../i18n/i18n.ts";
import type { PracticeController } from "../practice/controller.ts";
import type { PracticeSnapshot } from "../practice/protocol.ts";
import {
  activeStep,
  coachAck,
  coachDisplay,
  coachHint,
  coachObserve,
  coachStart,
  isBubble,
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
  /** R2205000: the shown moment's next hint; an expected display key as for `ack`. */
  hint(expected?: string): void;
  dispose(): void;
};

const FINISHED: CoachDisplay = { mode: "finished" };

export function displayKey(display: CoachDisplay): string {
  return isBubble(display) ? `${display.mode}:${display.id}` : display.mode;
}

function viewOf(coach: CoachState, ctx: CoachCtx | null, script: LessonScript): CoachView {
  if (ctx === null) return { coach, ctx, display: FINISHED, targets: [], aiBusy: false, yourMove: false };
  const display = coachDisplay(script, coach, ctx);
  const anchor = isBubble(display) ? display.anchor : null;
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
  return isBubble(display) && display.holdAi;
}

export type CoachOptions = {
  /** The entries the player's Field Guide holds as the lesson starts (R2205002). */
  known?: readonly string[];
  /** Each entry as the game first shows it found, for the device's Field Guide (R2205002). */
  onFound?: (ids: readonly string[]) => void;
};

export function createCoachTracker(source: CoachSource, script: LessonScript, options: CoachOptions = {}): CoachTracker {
  let coach: CoachState = coachStart(options.known ?? []);
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
      defOf: (defId) => snapshot.view.defs?.[defId] ?? defs?.[defId],
    };
    const before = coach.found.length;
    coach = coachObserve(script, coach, ctx);
    if (coach.found.length > before) options.onFound?.(coach.found.slice(before));
    publish();
  }

  const unsubscribe = source.subscribe(observe);
  // A step's words are looked up when the display is built, so a language switch builds it again.
  const unsubscribeLocale = subscribeLocale(() => {
    if (!disposed && ctx !== null) publish();
  });
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
    hint(expected) {
      if (disposed || ctx === null) return;
      if (expected !== undefined && expected !== displayKey(state.display)) return;
      const next = coachHint(script, coach);
      if (next === coach) return;
      coach = next;
      publish();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      unsubscribe();
      unsubscribeLocale();
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
