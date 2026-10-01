// C+ #12.4 Powder Spray (SPEC §8.7 row 12.4): (1) Spell, Pancake, Token (printed Legendary).
//   Both faces: "Deal {damage} damage to each enemy." — 3, Radiant 6.
// One hit each on every enemy unit on top of its pile and the enemy hero, all before the state check (R59).

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { damageAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012-4");

export const base: Script = {
  cry: (ctx) => [damageAll({ amount: param(ctx, "damage"), side: "enemy", heroes: true })],
};

/** The Radiant face is the same text at 6, a catalog value. */
export const radiant: Script = base;
