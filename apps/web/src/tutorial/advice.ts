// Coach advice for the coach's own line (SPEC §9.10, R292, R293): one legal action from the human
// view, never a rule (CLAUDE.md rule 7).

import type { ActionBody, UnitView } from "@jackioh/shared";

import type { CoachAnchor, CoachCtx } from "./coach.ts";
import { heroTargetId, unitsOf } from "./steps.ts";

export type Attack = Extract<ActionBody, { type: "attack" }>;
export type Play = Extract<ActionBody, { type: "play" }>;

export type Trade = {
  action: Attack;
  attacker: UnitView;
  target: UnitView;
  dealt: number;
  taken: number;
  kills: boolean;
  survives: boolean;
  /** The blow breaks the target's Divine Shield (it deals nothing, but the next hit lands). */
  pops: boolean;
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

/** One unit's blow on another, as the view shows them: its attack less the target's Armor, none into a shield. */
export function blow(from: UnitView, to: UnitView): number {
  if (hasKeyword(to, "Divine Shield")) return 0;
  return Math.max(0, from.attack - to.armor);
}

export function trades(ctx: CoachCtx): Trade[] {
  const out: Trade[] = [];
  for (const action of legalAttacksOf(ctx)) {
    const attacker = myUnit(ctx, action.attackerId);
    const target = enemyUnit(ctx, action.targetId);
    if (attacker === undefined || target === undefined) continue;
    const dealt = blow(attacker, target);
    const kills = dealt >= target.health;
    const strikesFirst = hasKeyword(attacker, "First Strike") && !hasKeyword(target, "First Strike");
    const taken = kills && strikesFirst ? 0 : blow(target, attacker);
    const pops = hasKeyword(target, "Divine Shield") && attacker.attack > 0;
    out.push({ action, attacker, target, dealt, taken, kills, survives: taken < attacker.health, pops });
  }
  return out;
}

const size = (unit: UnitView): number => unit.attack + unit.health;

/** Favor the biggest destroyed target, then the smallest surviving attacker. */
export function goodTrade(ctx: CoachCtx): Trade | undefined {
  const good = trades(ctx).filter((trade) => trade.kills && trade.survives);
  good.sort((a, b) => size(b.target) - size(a.target) || size(a.attacker) - size(b.attacker));
  return good[0];
}

export function heroAttack(ctx: CoachCtx): { action: Attack; attacker: UnitView } | undefined {
  const hero = heroTargetId(ctx.view, "opponent");
  const options = legalAttacksOf(ctx)
    .filter((action) => action.targetId === hero)
    .map((action) => ({ action, attacker: myUnit(ctx, action.attackerId) }))
    .filter((entry): entry is { action: Attack; attacker: UnitView } => entry.attacker !== undefined);
  options.sort((a, b) => b.attacker.attack - a.attacker.attack);
  return options[0];
}

/** Against an uncleared Taunt, preserve an attacker while making the best lasting dent. */
export function chip(ctx: CoachCtx): Trade | undefined {
  if (heroAttack(ctx) !== undefined) return undefined;
  // Spend the smallest surviving attacker to pop Divine Shield for the next hit.
  const options = trades(ctx).filter((trade) => trade.survives && (trade.dealt > 0 || trade.pops));
  options.sort((a, b) => b.dealt - a.dealt || a.taken - b.taken || size(a.attacker) - size(b.attacker));
  return options[0];
}

export function clearTaunt(ctx: CoachCtx): Trade | undefined {
  if (heroAttack(ctx) !== undefined) return undefined;
  const options = trades(ctx).filter((trade) => trade.kills && hasKeyword(trade.target, "Taunt"));
  options.sort((a, b) => Number(b.survives) - Number(a.survives) || size(a.attacker) - size(b.attacker));
  return options[0];
}

/** Sum distinct attackers' view-visible damage after Armor; traps can still answer the plan. */
export function lethalOnBoard(ctx: CoachCtx): boolean {
  const hero = heroTargetId(ctx.view, "opponent");
  const armor = ctx.view.opponent.hero.armor;
  const attackers = new Set<string>();
  let damage = 0;
  for (const action of legalAttacksOf(ctx)) {
    if (action.targetId !== hero || attackers.has(action.attackerId)) continue;
    const attacker = myUnit(ctx, action.attackerId);
    if (attacker === undefined) continue;
    attackers.add(action.attackerId);
    damage += Math.max(0, attacker.attack - armor);
  }
  return attackers.size > 0 && damage >= ctx.view.opponent.hero.health;
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
