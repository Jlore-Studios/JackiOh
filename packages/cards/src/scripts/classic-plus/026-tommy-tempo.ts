// C+ #26 Tommy Tempo (SPEC §8.7 row 26): Taunt, cast on draw; that cast ends your turn (Radiant: after
// {actions} more action, R415). No open zone: to the hand uncast (R560). Played from hand: a plain Unit.

import { isCastOnDraw, param, type EffectContext, type Script } from "@jackioh/engine";
import { endTurn, endTurnAfterActions } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-026");

function castOnDraw(ctx: EffectContext): boolean {
  return ctx.self !== null && isCastOnDraw(ctx.state, ctx.self);
}

export const base: Script = {
  staticFlags: { castOnDraw: true },
  cry: (ctx) => (castOnDraw(ctx) ? [endTurn()] : []),
};

export const radiant: Script = {
  staticFlags: { castOnDraw: true },
  cry: (ctx) => (castOnDraw(ctx) ? [endTurnAfterActions({ actions: param(ctx, "actions") })] : []),
};
