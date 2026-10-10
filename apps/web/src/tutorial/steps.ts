// Tutorial primitives read the view and legal actions, never game rules (CLAUDE.md rule 7).

import type { ActionBody, CardView, GameEvent, PlayerView, UnitView } from "@jackioh/shared";

import { sideView, type Side } from "../game/contract.ts";
import type { CoachAnchor, CoachCtx, CoachStep, CoachTip } from "./coach.ts";

export function isMyTurn(view: PlayerView): boolean {
  return view.active === view.viewer;
}

export function myMain(ctx: CoachCtx): boolean {
  const { view } = ctx;
  return view.result === null && isMyTurn(view) && view.phase === "main" && view.pending === null && !ctx.aiToAct;
}

export function aiTurn(view: PlayerView): boolean {
  return view.result === null && !isMyTurn(view) && view.phase !== "mulligan" && view.phase !== "setup";
}

/** The human's mulligan picker; both seats decide together (§2.1 step 3, R265). */
export function mulliganOpen(view: PlayerView): boolean {
  const prompt = view.pending !== null && view.pending.forYou && view.pending.kind === "mulligan";
  if (view.mulligan === undefined) return prompt;
  return !view.mulligan.youReady && prompt;
}

export function promptOpen(view: PlayerView, kind?: string): boolean {
  return view.pending !== null && view.pending.forYou && (kind === undefined || view.pending.kind === kind);
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

export function myTurnNumber(view: PlayerView): number {
  // p1 takes odd player-turns and p2 even ones (§2.1 step 5, §2.5).
  return view.viewer === "p1" ? Math.ceil(view.turn / 2) : Math.floor(view.turn / 2);
}

export function heroTargetId(view: PlayerView, side: Side): string {
  return `hero-${sideView(view, side).player}`;
}

export function freshOf<T extends GameEvent["type"]>(ctx: CoachCtx, type: T): Extract<GameEvent, { type: T }>[] {
  return ctx.fresh.filter((event): event is Extract<GameEvent, { type: T }> => event.type === type);
}

export function legalPlays(ctx: CoachCtx, defId: string): Extract<ActionBody, { type: "play" }>[] {
  const card = inHand(ctx.view, defId);
  if (card === undefined) return [];
  return ctx.legal.filter(
    (action): action is Extract<ActionBody, { type: "play" }> => action.type === "play" && action.instanceId === card.instanceId,
  );
}

export function legalAttacks(
  ctx: CoachCtx,
  attacker: string,
  target?: AttackTarget,
): Extract<ActionBody, { type: "attack" }>[] {
  const unit = unitOf(ctx.view, "you", attacker);
  if (unit === undefined) return [];
  const targetId = target === undefined ? undefined : attackTargetId(ctx.view, target);
  return ctx.legal.filter(
    (action): action is Extract<ActionBody, { type: "attack" }> =>
      action.type === "attack" &&
      action.attackerId === unit.instanceId &&
      (targetId === undefined || action.targetId === targetId),
  );
}

export type AttackTarget = "hero" | { defId: string };

export function attackTargetId(view: PlayerView, target: AttackTarget): string | undefined {
  if (target === "hero") return heroTargetId(view, "opponent");
  return unitOf(view, "opponent", target.defId)?.instanceId;
}

type StepText = CoachStep["text"];

type Common = {
  id: string;
  title: string;
  text: StepText;
  when?: (ctx: CoachCtx) => boolean;
  anchor?: CoachStep["anchor"];
  holdAi?: boolean;
};

export function info(options: Common & { done?: CoachStep["done"]; moot?: CoachStep["moot"]; final?: boolean }): CoachStep {
  return { kind: "info", ...options };
}

export function tip(options: CoachTip): CoachTip {
  return options;
}

/** Wait for an offered play; a card gone before display is moot. */
export function playCard(options: Common & { defId: string; lane?: number }): CoachStep {
  const { defId, lane, when, ...rest } = options;
  return {
    kind: "act",
    anchor: { kind: "handCard", defId },
    ...rest,
    when: (ctx) => myMain(ctx) && legalPlays(ctx, defId).length > 0 && (when === undefined || when(ctx)),
    done: (ctx) => inHand(ctx.view, defId) === undefined,
    moot: (ctx, since) => since === null && inHand(ctx.view, defId) === undefined,
    expect: (action, ctx) => {
      const card = inHand(ctx.view, defId);
      if (card === undefined || action.type !== "play" || action.instanceId !== card.instanceId) return false;
      return lane === undefined || action.zone?.lane === lane;
    },
  };
}

/** Wait for an offered attack; retire it when the shown turn ends. */
export function attackWith(options: Common & { attacker: string; target?: AttackTarget }): CoachStep {
  const { attacker, target, when, ...rest } = options;
  const defaultAnchor = (): CoachAnchor | null => {
    if (target === undefined) return { kind: "unit", side: "you", defId: attacker };
    return target === "hero" ? { kind: "hero", side: "opponent" } : { kind: "unit", side: "opponent", defId: target.defId };
  };
  return {
    kind: "act",
    anchor: defaultAnchor,
    ...rest,
    when: (ctx) => myMain(ctx) && legalAttacks(ctx, attacker, target).length > 0 && (when === undefined || when(ctx)),
    // Track the shown instance because tokens can share a definition.
    done: (ctx, since) => {
      const unit = unitOf(since, "you", attacker) ?? unitOf(ctx.view, "you", attacker);
      if (unit === undefined) return true;
      const onField = unitsOf(ctx.view, "you").some((candidate) => candidate.instanceId === unit.instanceId);
      if (!onField) return true;
      return freshOf(ctx, "attackDeclared").some((event) => event.attackerId === unit.instanceId && !event.forced);
    },
    moot: (ctx, since) => {
      if (since !== null) return ctx.view.turn !== since.turn;
      return unitOf(ctx.view, "you", attacker) === undefined && inHand(ctx.view, attacker) === undefined;
    },
    expect: (action, ctx) =>
      action.type === "attack" && legalAttacks(ctx, attacker, target).some((legal) => legal.targetId === action.targetId && legal.attackerId === action.attackerId),
  };
}

export function endTurn(options: Common): CoachStep {
  const { when, ...rest } = options;
  return {
    kind: "act",
    anchor: { kind: "endTurn" },
    ...rest,
    when: (ctx) => myMain(ctx) && (when === undefined || when(ctx)),
    done: (ctx, since) => ctx.view.turn !== since.turn,
    // An automatic end before display (R82) must not end the next turn.
    turnBound: true,
    expect: (action) => action.type === "endTurn",
  };
}

export function keepHand(options: Common): CoachStep {
  const { when, ...rest } = options;
  return {
    kind: "act",
    anchor: { kind: "prompt" },
    ...rest,
    when: (ctx) => mulliganOpen(ctx.view) && (when === undefined || when(ctx)),
    done: (ctx, since) => since !== ctx.view && !mulliganOpen(ctx.view),
    moot: (ctx, since) => since === null && ctx.view.phase !== "mulligan" && ctx.view.phase !== "setup",
    expect: (action, ctx) => {
      const pending = ctx.view.pending;
      if (action.type !== "mulligan" || pending === null || !pending.forYou) return false;
      return action.keep.length === pending.options.length;
    },
  };
}

export function mulliganAway(options: Common & { defIds: readonly string[] }): CoachStep {
  const { defIds, when, ...rest } = options;
  return {
    kind: "act",
    anchor: { kind: "prompt" },
    ...rest,
    when: (ctx) => mulliganOpen(ctx.view) && (when === undefined || when(ctx)),
    done: (ctx, since) => since !== ctx.view && !mulliganOpen(ctx.view),
    moot: (ctx, since) => since === null && ctx.view.phase !== "mulligan" && ctx.view.phase !== "setup",
    expect: (action, ctx) => {
      const pending = ctx.view.pending;
      if (action.type !== "mulligan" || pending === null || !pending.forYou) return false;
      const returned = new Set(pending.options.filter((option) => defIds.includes(option.defId ?? "")).map((option) => option.key));
      const kept = pending.options.map((option) => option.key).filter((key) => !returned.has(key));
      return action.keep.length === kept.length && kept.every((key) => action.keep.includes(key));
    },
  };
}

export function switchPosition(options: Common & { defId: string; to: "ATK" | "DEF" }): CoachStep {
  const { defId, to, when, ...rest } = options;
  const legalSwitch = (ctx: CoachCtx): boolean => {
    const unit = unitOf(ctx.view, "you", defId);
    return unit !== undefined && unit.position !== to && ctx.legal.some((action) => action.type === "switchPosition" && action.instanceId === unit.instanceId);
  };
  return {
    kind: "act",
    anchor: { kind: "unit", side: "you", defId },
    ...rest,
    when: (ctx) => myMain(ctx) && legalSwitch(ctx) && (when === undefined || when(ctx)),
    done: (ctx) => {
      const unit = unitOf(ctx.view, "you", defId);
      return unit === undefined || unit.position === to;
    },
    moot: (ctx, since) => since !== null && ctx.view.turn !== since.turn,
    expect: (action, ctx) => {
      const unit = unitOf(ctx.view, "you", defId);
      return unit !== undefined && action.type === "switchPosition" && action.instanceId === unit.instanceId;
    },
  };
}
