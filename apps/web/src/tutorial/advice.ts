// Coach advice for the coach's own line (SPEC §9.10, R292, R293): one legal action from the human
// view, never a rule (CLAUDE.md rule 7). It does no combat maths: what an attack would do (the damage
// after Armor and Divine Shield, the strike back, whether it kills, whether the attacker lives, whether
// the board is lethal) is the engine's `combatForecast`, which the practice worker asks beside the view
// (R3100000); this file only picks among those facts.

import type { ActionBody, AttackForecast, UnitView } from "@jackioh/shared";

import type { CoachAnchor, CoachCtx } from "./coach.ts";
import { heroTargetId, unitsOf } from "./steps.ts";

export type Attack = Extract<ActionBody, { type: "attack" }>;
export type Play = Extract<ActionBody, { type: "play" }>;

/**
 * One attack on a unit with the engine's forecast of it: `dealt` and `taken` are the damage each way,
 * `kills` and `survives` what becomes of the target and the attacker, `pops` that the blow breaks the
 * target's Divine Shield (it deals nothing, but the next hit lands), and the two sizes the measure the
 * coach ranks by.
 */
export type Trade = AttackForecast & {
  action: Attack;
  attacker: UnitView;
  target: UnitView;
};

export function legalAttacksOf(ctx: CoachCtx): Attack[] {
  return ctx.legal.filter((action): action is Attack => action.type === "attack");
}

export function myUnit(ctx: CoachCtx, instanceId: string): UnitView | undefined {
  return unitsOf(ctx.view, "you").find((unit) => unit.instanceId === instanceId);
}

export function enemyUnit(ctx: CoachCtx, instanceId: string): UnitView | undefined {
  return unitsOf(ctx.view, "opponent").find((unit) => unit.instanceId === instanceId);
}

export function hasKeyword(unit: UnitView, keyword: string): boolean {
  return unit.keywords.some((entry) => entry.kind === keyword);
}

/** The engine's forecast of this attack, or none in a game that was not asked for one. */
export function forecastOf(ctx: CoachCtx, action: Attack): AttackForecast | undefined {
  return ctx.forecast?.attacks.find((entry) => entry.attackerId === action.attackerId && entry.targetId === action.targetId);
}

export function trades(ctx: CoachCtx): Trade[] {
  const out: Trade[] = [];
  for (const action of legalAttacksOf(ctx)) {
    const attacker = myUnit(ctx, action.attackerId);
    const target = enemyUnit(ctx, action.targetId);
    const forecast = forecastOf(ctx, action);
    if (attacker === undefined || target === undefined || forecast === undefined) continue;
    out.push({ ...forecast, action, attacker, target });
  }
  return out;
}

/** Favor the biggest destroyed target, then the smallest surviving attacker. */
export function goodTrade(ctx: CoachCtx): Trade | undefined {
  const good = trades(ctx).filter((trade) => trade.kills && trade.survives);
  good.sort((a, b) => b.targetSize - a.targetSize || a.attackerSize - b.attackerSize);
  return good[0];
}

/** The hit on the enemy hero that deals the most, its first ready attacker among equals. */
export function heroAttack(ctx: CoachCtx): { action: Attack; attacker: UnitView } | undefined {
  const hero = heroTargetId(ctx.view, "opponent");
  const options = legalAttacksOf(ctx)
    .filter((action) => action.targetId === hero)
    .map((action) => ({ action, attacker: myUnit(ctx, action.attackerId), dealt: forecastOf(ctx, action)?.dealt ?? 0 }))
    .filter((entry): entry is { action: Attack; attacker: UnitView; dealt: number } => entry.attacker !== undefined);
  options.sort((a, b) => b.dealt - a.dealt);
  const best = options[0];
  return best === undefined ? undefined : { action: best.action, attacker: best.attacker };
}

/** Against an uncleared Taunt, preserve an attacker while making the best lasting dent. */
export function chip(ctx: CoachCtx): Trade | undefined {
  if (heroAttack(ctx) !== undefined) return undefined;
  // Spend the smallest surviving attacker to pop Divine Shield for the next hit.
  const options = trades(ctx).filter((trade) => trade.survives && (trade.dealt > 0 || trade.pops));
  options.sort((a, b) => b.dealt - a.dealt || a.taken - b.taken || a.attackerSize - b.attackerSize);
  return options[0];
}

export function clearTaunt(ctx: CoachCtx): Trade | undefined {
  if (heroAttack(ctx) !== undefined) return undefined;
  const options = trades(ctx).filter((trade) => trade.kills && hasKeyword(trade.target, "Taunt"));
  options.sort((a, b) => Number(b.survives) - Number(a.survives) || a.attackerSize - b.attackerSize);
  return options[0];
}

/** The engine's verdict that the attackers who can reach the hero finish it; traps can still answer the plan. */
export function lethalOnBoard(ctx: CoachCtx): boolean {
  return ctx.forecast?.lethal === true;
}

export function aimsAtOwnSide(ctx: CoachCtx, play: Play): boolean {
  const own = new Set<string>(unitsOf(ctx.view, "you").map((unit) => unit.instanceId));
  for (const card of ctx.view.you.backrow) if (card !== null && !card.faceDown) own.add(card.instanceId);
  return (play.targets ?? []).some(
    (target) =>
      (target.pick === "instance" && own.has(target.instanceId)) ||
      (target.pick === "hero" && target.player === ctx.view.viewer) ||
      (target.pick === "zone" && target.player === ctx.view.viewer && target.row === "backrow"),
  );
}

export function bestPlay(ctx: CoachCtx): Play | undefined {
  const hand = Array.isArray(ctx.view.you.hand) ? ctx.view.you.hand : [];
  const costOf = (instanceId: string): number => hand.find((card) => card.instanceId === instanceId)?.cost ?? 0;
  const plays = ctx.legal.filter((action): action is Play => action.type === "play" && !aimsAtOwnSide(ctx, action));
  plays.sort((a, b) => costOf(b.instanceId) - costOf(a.instanceId));
  return plays[0];
}

export type Move =
  | { kind: "play"; action: Play; defId: string }
  | { kind: "trade"; action: Attack; trade: Trade }
  | { kind: "hero"; action: Attack; attacker: UnitView; lethal: boolean }
  | { kind: "clear"; action: Attack; trade: Trade }
  | { kind: "chip"; action: Attack; trade: Trade }
  | { kind: "end"; action: ActionBody };

export function nextMove(ctx: CoachCtx): Move | undefined {
  if (lethalOnBoard(ctx)) {
    const hero = heroAttack(ctx);
    if (hero !== undefined) return { kind: "hero", action: hero.action, attacker: hero.attacker, lethal: true };
  }
  const play = bestPlay(ctx);
  if (play !== undefined) {
    const hand = Array.isArray(ctx.view.you.hand) ? ctx.view.you.hand : [];
    const card = hand.find((candidate) => candidate.instanceId === play.instanceId);
    return { kind: "play", action: play, defId: card?.defId ?? "" };
  }
  const trade = goodTrade(ctx);
  if (trade !== undefined) return { kind: "trade", action: trade.action, trade };
  const hero = heroAttack(ctx);
  if (hero !== undefined) return { kind: "hero", action: hero.action, attacker: hero.attacker, lethal: false };
  const clear = clearTaunt(ctx);
  if (clear !== undefined) return { kind: "clear", action: clear.action, trade: clear };
  const dent = chip(ctx);
  if (dent !== undefined) return { kind: "chip", action: dent.action, trade: dent };
  const end = ctx.legal.find((action) => action.type === "endTurn");
  return end === undefined ? undefined : { kind: "end", action: end };
}

export function sameMove(a: ActionBody, b: ActionBody | undefined): boolean {
  return b !== undefined && JSON.stringify(a) === JSON.stringify(b);
}

/** A zone anchor distinguishes duplicate units of the same card. */
export function zoneOf(ctx: CoachCtx, side: "you" | "opponent", instanceId: string): CoachAnchor | null {
  const units = side === "you" ? ctx.view.you.units : ctx.view.opponent.units;
  const index = units.findIndex((unit) => unit?.instanceId === instanceId);
  return index < 0 ? null : { kind: "zone", side, row: "units", lane: index + 1 };
}
