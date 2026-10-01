// C+ #50 Adaptive Growth (SPEC §8.7 row 50). (1) Spell, Epic.
//   Base:    "Cast on draw: If you control fewer Units than your opponent, give all Units −3/−3.
//            Otherwise, give all Units +{buff}/+{buff}." — buff 2
//   Radiant: "Cast on draw: If you control fewer Units than your opponent, give enemy Units
//            −{debuff}/−{debuff}. Otherwise, give your Units +{buff}/+{buff}." — debuff 4, buff 3
//   Engine:  "Cast on draw (§6.2, R58); the counts are read as it resolves (Units on top of their
//            piles). Permanent buffs (§10.4 layer 4); −3/−3 lowers max health, so units at 0 die at
//            the state check. Tunes: buff 2 ↑; Radiant debuff 4 ↑."
//
// Played from a hand it does the same (§6.2: Cast on draw is a cast, R70, and the card stays playable).
// `conditionMet` (R195) is the same `fewer` the Cry branches on, so the glow and the branch agree; its
// proofs are in packages/cards/test/condition-active.test.ts.

import { activeUnitsOf, param, type GameState, type Script } from "@jackioh/engine";
import { buffAllUnits } from "@jackioh/engine/effects";
import { opponentOf, type PlayerId } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-050");

/** The base face's −3/−3, printed and not a declared number. */
const BASE_DEBUFF = 3;

/** "If you control fewer Units than your opponent": the tops of their piles, read now. */
function fewer(state: GameState, player: PlayerId): boolean {
  return activeUnitsOf(state, player).length < activeUnitsOf(state, opponentOf(player)).length;
}

export const base: Script = {
  staticFlags: { castOnDraw: true },
  cry: (ctx) => {
    const amount = fewer(ctx.state, ctx.controller) ? -BASE_DEBUFF : param(ctx, "buff");
    return [buffAllUnits({ side: "both", attack: amount, health: amount })];
  },
  conditionMet: (ctx) => ctx.zone === "hand" && fewer(ctx.state, ctx.controller),
};

export const radiant: Script = {
  staticFlags: { castOnDraw: true },
  cry: (ctx) => {
    if (fewer(ctx.state, ctx.controller)) {
      const amount = -param(ctx, "debuff");
      return [buffAllUnits({ side: "enemy", attack: amount, health: amount })];
    }
    const amount = param(ctx, "buff");
    return [buffAllUnits({ side: "self", attack: amount, health: amount })];
  },
  conditionMet: base.conditionMet,
};
