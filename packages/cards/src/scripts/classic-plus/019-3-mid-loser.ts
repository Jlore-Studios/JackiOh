// C+ #19.3 Mid Loser (SPEC §8.7 row 19.3): Cry flips a coin (Lucky X: X more flips, heads kept): heads
// +{heads}/+{heads}; tails −{tails}/−{tails} and the opponent's next refresh is 1 higher.

import { numberedSum, param, unitView, type EffectContext, type Effect, type Script } from "@jackioh/engine";
import { buff, nextTurnMana } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-019-3");

/** "your opponent gains 1 mana next turn". */
const TAILS_MANA = 1;

/** One coin, Lucky X times more, heads kept if any of them lands heads. */
function landsHeads(ctx: EffectContext): boolean {
  const self = ctx.self;
  const lucky = self !== null && self.zone.z === "field" ? (numberedSum(unitView(ctx.state, self).keywords, "Lucky") ?? 0) : 0;
  const flip = (): boolean => ctx.rng.coin();
  return lucky > 0 ? ctx.rng.lucky(lucky, flip, (a, b) => a || b) : flip();
}

function cry(ctx: EffectContext): Effect[] {
  if (landsHeads(ctx)) {
    const heads = param(ctx, "heads");
    return [buff({ target: { of: "self" }, attack: heads, health: heads })];
  }
  const tails = param(ctx, "tails");
  return [buff({ target: { of: "self" }, attack: -tails, health: -tails }), nextTurnMana({ amount: TAILS_MANA, player: "enemy" })];
}

export const base: Script = { cry };

// The same script: the Radiant face's Lucky 1 and its heads 10 are catalog data the script reads.
export const radiant: Script = base;
