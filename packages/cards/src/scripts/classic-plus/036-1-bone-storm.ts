// C+ #36.1 Bone Storm (SPEC §8.7 row 36.1): (1) Spell, Token (printed Rare), from C+ #36.
//   Base:    "Cast on draw: Deal {damage} damage to each enemy."
//   Radiant: "Echo 1" plus the same: it resolves a second time.
// Cast on draw is §6.2's static flag (R58, R70); "each enemy" is one hit on every enemy unit and
// the enemy hero, all landing before the state check (R59). Echo is the `echo` flag (R51's repeat).

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { damageAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-036-1");

export const base: Script = {
  staticFlags: { castOnDraw: true },
  cry: (ctx) => [damageAll({ amount: param(ctx, "damage"), side: "enemy", heroes: true })],
};

export const radiant: Script = { ...base, staticFlags: { castOnDraw: true, echo: 1 } };
