// C+ #21 Whirlwind (SPEC §8.7 row 21): Pierce, {damage} damage to every Unit (R346); the Radiant face
// returns from the graveyard to hand at the end of the turn it was played (R68).

import { param, type Script } from "@jackioh/engine";
import { bounce, damageAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-021");

export const base: Script = {
  cry: (ctx) => [damageAll({ amount: param(ctx, "damage"), side: "any", ignoreArmor: true })],
};

export const radiant: Script = {
  ...base,
  endOfTurn: (ctx) => (ctx.self?.returnToHandAtEndOfTurn === true ? [bounce({ target: { of: "self" } })] : []),
};
