// C #40 MC Tech (SPEC §8.6 row 40). (1) Unit, Rare, 3/3 → 6/6.
//   Base:    "Cry: If your opponent controls {threshold} or more permanents, steal a random one." — 4
//   Radiant: "Cry: If your opponent controls {threshold} or more permanents, steal one of your choice." — 4
//   Engine:  "The count and the pick at resolution (the Radiant's pick is a prompt, since the
//            condition is read then); a random pick over their permanents, the tops of piles and the
//            backrow. R15's placement; with no free zone, it stays with them. A stolen face-down trap
//            is readable by you from then on (R33). Tunes: threshold 4 ↓."
//
// "Permanents your opponent controls": the top card of each of their unit piles (a card dormant under
// a Stack is not on the field, R13) and every card in their backrow, face-down ones included — the
// count is public, since both rows' occupancy is. It is read as the Cry resolves.
//
// Base: one of them at random (R60), drawn from the match rng as the Cry reaches the clause. Radiant:
// the player picks one in a target prompt opened then (§10.6) — the condition is only known at
// resolution, so the pick cannot be declared with the play (R81); a face-down option names nothing
// but its id to its chooser (R177).
//
// §6.3 Steal: R15 puts the card in the same lane of MC Tech's controller's row when that is free,
// else the first free zone of it; with none it stays with its owner and nothing happens. The change
// of control is an entry (R171). R33: a stolen face-down trap stays face-down, read by its new
// controller from then on.
//
// R195: in hand the card glows when the opponent controls enough permanents now.
//
// The threshold is the declared `threshold` (R386), read through `param`.

import {
  activeUnitsOf,
  cardAt,
  param,
  slotsOf,
  type CardInstance,
  type ConditionContext,
  type EffectContext,
  type Effect,
  type GameState,
  type Script,
} from "@jackioh/engine";
import { chooseTarget, forEachCard, steal } from "@jackioh/engine/effects";
import { opponentOf, type PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-040");

/** The Radiant pick's resume step. */
const STEAL_STEP = "steal";

/** The permanents `player`'s opponent controls: the tops of their unit piles, then their backrow. */
function enemyPermanents(state: GameState, player: PlayerId): CardInstance[] {
  const enemy = opponentOf(player);
  const backrow = slotsOf(enemy, "backrow").flatMap((ref) => {
    const card = cardAt(state, ref);
    return card === null ? [] : [card];
  });
  return [...activeUnitsOf(state, enemy), ...backrow];
}

/** "If your opponent controls {threshold} or more permanents": the one test the Cry and the glow make. */
function enoughPermanents(ctx: EffectContext | ConditionContext): boolean {
  return enemyPermanents(ctx.state, ctx.controller).length >= param(ctx, "threshold");
}

/** R195: in hand, whether playing it now would steal. */
const conditionMet = (ctx: ConditionContext): boolean => ctx.zone === "hand" && enoughPermanents(ctx);

export const base: Script = {
  cry: (ctx): Effect[] =>
    enoughPermanents(ctx)
      ? [
          forEachCard({
            // R60: one of them at random, drawn as the Cry reaches the clause.
            cards: (c) => c.rng.shuffle(enemyPermanents(c.state, c.controller)).slice(0, 1),
            each: (instanceId) => steal({ instanceId }),
          }),
        ]
      : [],
  conditionMet,
};

export const radiant: Script = {
  cry: (ctx): Effect[] =>
    enoughPermanents(ctx)
      ? [chooseTarget({ step: STEAL_STEP, scope: { side: "enemy", of: ["unit", "backrow"] }, prompt: "Steal one of their permanents" })]
      : [],
  resume: {
    [STEAL_STEP]: () => [steal({ target: { of: "chosen" } })],
  },
  conditionMet,
};
