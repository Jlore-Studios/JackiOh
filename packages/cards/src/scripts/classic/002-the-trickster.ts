// C #2 The Trickster (SPEC §8.6 row 2, §6.3 Cost; R65, R70). Unit, Human, cost 1, Common,
// 2/1 → 4/2.
//   Base:    "Cry: Your next Trap or Field Spell costs ({discount}) less."
//   Radiant: "Cry: Your next Trap or Field Spell costs (0)."
//   Engine:  "A player modifier like #35 Lunar Eclipse's next-Spell discount (`costDiscount`; Cost,
//            §6.3, R65) for Traps, Field Traps and Field Spells, consumed by the next such card you
//            play; "next" has no "this turn", so it waits across turns until used; Field Trap counts
//            as Trap. The Radiant face sets that card's cost to (0) instead of discounting it. Tunes:
//            discount 2 ↑."
//
// THE MODIFIER is a price rule on its controller (B5 E15, `addCostRule`) that lasts "until used": it
// reaches Traps, Field Traps and Field Spells ("Trap" names "Field Trap" too), and the first play of
// one whose price it changed spends it (`mana.costRulesSpentBy`), whatever turn that is. A Spell or
// a Unit is never reached, so it neither uses nor spends it. R65 floors the price at (0). A cast pays
// nothing, so it never uses the rule and never spends it (R70).
//
// The base face's discount is the card's declared number (`param(ctx, "discount")`), a negative
// `amount`; the Radiant face's "costs (0)" is a `setTo`, which wins over every add (B5 E15).

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import type { CardType } from "@jackioh/shared";
import { addCostRule } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-002");

/** "Trap or Field Spell": a Field Trap is a Trap. */
const REACHES: CardType[] = ["Trap", "Field Trap", "Field Spell"];

export const base: Script = {
  cry: (ctx) => [addCostRule({ rule: { types: REACHES, amount: -param(ctx, "discount") }, lasts: "used" })],
};

export const radiant: Script = {
  cry: () => [addCostRule({ rule: { types: REACHES, setTo: 0 }, lasts: "used" })],
};
