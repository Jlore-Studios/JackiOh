// The lessons' moment builders (SPEC §9.10, R2205000). What a moment names here is only the coach's own
// line, `expect`, which the page never decides by (R293), and what its last hint points at. Every
// legality is the engine's: a pick reads `ctx.legal`.

import type { ActionBody, UnitView } from "@jackioh/shared";

import { goodTrade, heroAttack, legalAttacksOf, myUnit, nextMove, sameMove, zoneOf, type Attack, type Play } from "../advice.ts";
import type { CoachAnchor, CoachCtx, CoachStep, Text } from "../coach.ts";
import { inHand, moment, myHand, unitsOf } from "../steps.ts";

/** A pick of the coach's line: the action it would take now, or none. */
export type Pick = (ctx: CoachCtx) => ActionBody | undefined;

type MomentOptions = {
  id: string;
  title: string;
  text: string;
  discovers: readonly string[];
  hints: readonly [Text, Text];
  /** The coach's line; the moment shows only while it offers an action. */
  pick: Pick;
  /** Further gate on the moment's board, beyond `pick`. */
  when?: (ctx: CoachCtx) => boolean;
  /** What the pointed hint rings; by default what `pick` names. */
  point?: (ctx: CoachCtx) => CoachAnchor | null;
  revisit?: boolean;
  prompt?: boolean;
};

/** The board element an action is about. */
export function anchorOfAction(ctx: CoachCtx, action: ActionBody | undefined): CoachAnchor | null {
  if (action === undefined) return null;
  if (action.type === "play") {
    const card = myHand(ctx.view).find((each) => each.instanceId === action.instanceId);
    return card === undefined ? null : { kind: "handCard", defId: card.defId };
  }
  if (action.type === "attack") {
    if (action.targetId.startsWith("hero-")) return { kind: "hero", side: "opponent" };
    return zoneOf(ctx, "opponent", action.targetId);
  }
  if (action.type === "switchPosition") return zoneOf(ctx, "you", action.instanceId);
  if (action.type === "endTurn") return { kind: "endTurn" };
  if (action.type === "mulligan") return { kind: "prompt" };
  return null;
}

export function momentOf(options: MomentOptions): CoachStep {
  const { pick, when, point, ...rest } = options;
  return moment({
    ...rest,
    when: (ctx) => pick(ctx) !== undefined && (when === undefined || when(ctx)),
    hintAnchor: (ctx) => (point === undefined ? anchorOfAction(ctx, pick(ctx)) : point(ctx)),
    expect: (action, ctx) => sameMove(action, pick(ctx)),
  });
}

/** The last moment: win the game. It ends with it, so it has no turn limit (R292). */
export function finishOf(options: Omit<MomentOptions, "pick" | "discovers">): CoachStep {
  return { ...momentOf({ ...options, discovers: ["winning"], pick: (ctx) => nextMove(ctx)?.action }), final: true };
}

type PlayFilter = { lane?: number; target?: "enemy" | "own" | "none" };

function aimsAt(ctx: CoachCtx, play: Play, side: "enemy" | "own"): boolean {
  const ids = new Set(unitsOf(ctx.view, side === "enemy" ? "opponent" : "you").map((unit) => unit.instanceId));
  return (play.targets ?? []).some((target) => target.pick === "instance" && ids.has(target.instanceId));
}

/** The first legal play of the first of `defIds` in hand that meets `filter`. */
export function playOf(defIds: readonly string[], filter: PlayFilter = {}): Pick {
  return (ctx) => {
    for (const defId of defIds) {
      const card = inHand(ctx.view, defId);
      if (card === undefined) continue;
      const plays = ctx.legal.filter(
        (action): action is Play =>
          action.type === "play" &&
          action.instanceId === card.instanceId &&
          (filter.lane === undefined || action.zone?.lane === filter.lane) &&
          (filter.target === undefined ||
            (filter.target === "none" ? (action.targets ?? []).length === 0 : aimsAt(ctx, action, filter.target))),
      );
      // Against an enemy, aim at its biggest unit.
      if (filter.target === "enemy") {
        const attackOf = (play: Play): number => {
          const id = (play.targets ?? []).find((target) => target.pick === "instance");
          const unit = id?.pick === "instance" ? unitsOf(ctx.view, "opponent").find((each) => each.instanceId === id.instanceId) : undefined;
          return unit === undefined ? -1 : unit.attack + unit.health;
        };
        plays.sort((a, b) => attackOf(b) - attackOf(a));
      }
      if (plays[0] !== undefined) return plays[0];
    }
    return undefined;
  };
}

/** The coach's hit on the enemy hero, its biggest ready attacker first. */
export const hitHero: Pick = (ctx) => heroAttack(ctx)?.action;

/** A trade the attacker survives, else any attack on a unit. */
export const tradeUnits: Pick = (ctx) => goodTrade(ctx)?.action ?? legalAttacksOf(ctx).find((action) => !action.targetId.startsWith("hero-"));

/** An attack by a unit that `has` picks, on an enemy unit `at` picks (any when absent). */
export function attackBy(has: (unit: UnitView) => boolean, at?: (unit: UnitView) => boolean): Pick {
  return (ctx) =>
    legalAttacksOf(ctx).find((action: Attack) => {
      const attacker = myUnit(ctx, action.attackerId);
      if (attacker === undefined || !has(attacker)) return false;
      const target = unitsOf(ctx.view, "opponent").find((unit) => unit.instanceId === action.targetId);
      return target !== undefined && (at === undefined || at(target));
    });
}

export function keyworded(kind: string): (unit: UnitView) => boolean {
  return (unit) => unit.keywords.some((keyword) => keyword.kind === kind);
}

export function isCard(defId: string): (unit: UnitView) => boolean {
  return (unit) => unit.defId === defId;
}

export function either(...picks: Pick[]): Pick {
  return (ctx) => {
    for (const pick of picks) {
      const action = pick(ctx);
      if (action !== undefined) return action;
    }
    return undefined;
  };
}
