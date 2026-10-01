// C #77 Anti-Magic Monkey (SPEC §8.6 row 77, BUILD M9 Classic row C 77). (2) Unit 5/5 → 10/10, Common.
//   Base:    "Stack / Aura: Spells cost ({surcharge}) more." (1)
//   Radiant: "Stack / Aura: Spells cost ({surcharge}) more." (2)
//   Engine:  "Cost (§6.3, R65) on both players' Spells (the Spell type, not Field Spells) where a play
//            takes them from (a hand, or a graveyard a permission lets its owner play from, §6.3 Play),
//            as a price for a play; a cast pays nothing (R70). Tunes: surcharge 1 ↑ (Radiant 2)."
//
// The aura is B5 E15's price rule (`Script.costAura`): a flat rung of the declared surcharge
// (`param`, R386) on every player's cards of the Spell type — a card's type is its running face's
// (B2.7) — read by R65's `effectiveCost` wherever a play would take the card from. An X-cost Spell
// costs exactly its X and takes no rule (R65). The rule is laid only while the Monkey acts on the
// field: dormant under a Stack pile it is not on the field for effects (§3.2, R13), and it is gone the
// moment it leaves. Stack is printed on both faces (§10.4 layer 1), so it may be played onto an
// occupied unit zone (§6.2).

import type { CostAuraArgs, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-077");

export const base: Script = {
  costAura: (args: CostAuraArgs) => [{ whose: "all", types: ["Spell"], amount: param(args, "surcharge") }],
};

// The same script: the Radiant face differs only in its declared surcharge (2) and its doubled stats.
export const radiant: Script = base;
