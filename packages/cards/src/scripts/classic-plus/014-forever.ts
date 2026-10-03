// C+ #14 Forever& (SPEC §8.7 row 14, R410): (1) Spell, Epic.
//   Base:    "The next Spell you play gains "After this resolves, return it to your hand. This can't
//            cost less than ({floor})."" — floor 2.
//   Radiant: floor 1, no Draw (balance patch 1).
// A player modifier that waits until used (not turn-scoped) stamps E39's enchantment on the next Spell
// played, a cast included (`enchantNextSpell`); the enchantment rides the card in every zone and its
// floor applies after every discount (R65). Installed as this resolves, so Forever& never stamps itself.

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { enchantNextSpell } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-014");

export const base: Script = {
  cry: (ctx) => [enchantNextSpell({ enchantment: { kind: "returnAfterResolve", floor: param(ctx, "floor") } })],
};

export const radiant: Script = {
  cry: (ctx) => [
    enchantNextSpell({ enchantment: { kind: "returnAfterResolve", floor: param(ctx, "floor") } }),
  ],
};
