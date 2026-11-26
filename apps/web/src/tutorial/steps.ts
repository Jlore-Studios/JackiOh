// Tutorial primitives read the view and legal actions, never game rules (CLAUDE.md rule 7).

import type { CardView, GameEvent, PlayerView, UnitView } from "@jackioh/shared";

import { sideView, type Side } from "../game/contract.ts";
import type { Params } from "../i18n/i18n.ts";
import type { CoachCtx, CoachStep } from "./coach.ts";

export function isMyTurn(view: PlayerView): boolean {
  return view.active === view.viewer;
}

export function myMain(ctx: CoachCtx): boolean {
  const { view } = ctx;
  return view.result === null && isMyTurn(view) && view.phase === "main" && view.pending === null && !ctx.aiToAct;
}

/** The human's mulligan picker; both seats decide together (§2.1 step 3, R265). */
export function mulliganOpen(view: PlayerView): boolean {
  const prompt = view.pending !== null && view.pending.forYou && view.pending.kind === "mulligan";
  if (view.mulligan === undefined) return prompt;
  return !view.mulligan.youReady && prompt;
}

export function myHand(view: PlayerView): CardView[] {
  return Array.isArray(view.you.hand) ? view.you.hand : [];
}

export function inHand(view: PlayerView, defId: string): CardView | undefined {
  return myHand(view).find((card) => card.defId === defId);
}

export function unitsOf(view: PlayerView, side: Side): UnitView[] {
  return sideView(view, side).units.filter((unit): unit is UnitView => unit !== null);
}

/** A deck holds each card once (§2.6). */
export function unitOf(view: PlayerView, side: Side, defId: string): UnitView | undefined {
  return unitsOf(view, side).find((unit) => unit.defId === defId);
}

export function heroTargetId(view: PlayerView, side: Side): string {
  return `hero-${sideView(view, side).player}`;
}

export function freshOf<T extends GameEvent["type"]>(ctx: CoachCtx, type: T): Extract<GameEvent, { type: T }>[] {
  return ctx.fresh.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
}

type StepText = CoachStep["text"];

type Common = {
  id: string;
  title: CoachStep["title"];
  text: StepText;
  params?: Params;
  when?: (ctx: CoachCtx) => boolean;
  anchor?: CoachStep["anchor"];
  holdAi?: boolean;
};

export function info(options: Common & { done?: CoachStep["done"]; moot?: CoachStep["moot"]; final?: boolean }): CoachStep {
  return { kind: "info", ...options };
}

/** R2205000: pose a question and wait for the player to find what it sets up; never holds the AI (R314). */
export function moment(
  options: Omit<Common, "holdAi"> & {
    discovers: readonly string[];
    hints: readonly [StepText, StepText];
    hintAnchor?: CoachStep["hintAnchor"];
    expect?: CoachStep["expect"];
    done?: CoachStep["done"];
    moot?: CoachStep["moot"];
    revisit?: boolean;
    /** Shows on the human's own open prompt (the mulligan) rather than their main phase. */
    prompt?: boolean;
  },
): CoachStep {
  const { when, prompt, ...rest } = options;
  const ready = (ctx: CoachCtx): boolean =>
    prompt === true ? ctx.view.pending !== null && ctx.view.pending.forYou : myMain(ctx);
  return { kind: "moment", ...rest, when: (ctx) => ready(ctx) && (when === undefined || when(ctx)) };
}
