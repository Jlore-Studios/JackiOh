// C #6 Cloaked Toe Cracker (SPEC §8.6 row 6, §6.3 Cost, §6.3 Mana; R33, R65, R70). Unit, Human,
// cost 2, Common, 3/4 → 6/8.
//   Base:    "Aura: Your Traps cost (0)."
//   Radiant: "Aura: Your Traps cost (0).\nAfter you play a Trap, gain {mana} mana."
//   Engine:  "A cost aura (Cost, §6.3, R65) on its controller's Traps and Field Traps in hand.
//            Radiant: a trigger on your `cardPlayed` of a Trap or Field Trap, +1 temporary mana
//            (§6.3 Mana). Tunes: Radiant mana 1 ↑."
//
// THE AURA is a price rule the card lays while it acts on the field (B5 E15, `Script.costAura`):
// "costs (0)" (`setTo: 0`) on its controller's Traps and Field Traps ("Trap" names "Field Trap" too).
// A price is a play's, so it reaches the cards a play takes (the hand, R65) and lasts only while the
// card stands: once it leaves, the Traps are back at their own cost. The opponent's Traps and your
// Field Spells are not Traps of yours, so they are untouched.
//
// THE RADIANT TRIGGER answers every `cardPlayed` of a Trap or Field Trap by its controller, a cast
// included (R70: a cast counts as a play), and gains the card's declared mana (`param(ctx, "mana")`),
// temporary mana for this turn (§6.3 Mana). The `manaChanged` it makes names a player and numbers, so
// a trap played face-down stays unnamed to the opponent (R33, R97).

import type { GameEvent } from "@jackioh/shared";
import type { Script, TriggerDef } from "@jackioh/engine";
import { defOf, param } from "@jackioh/engine";
import { gainMana } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-006");

const TRAP_TYPES: readonly string[] = ["Trap", "Field Trap"];

const aura: Script["costAura"] = () => [{ whose: "yours", types: ["Trap", "Field Trap"], setTo: 0 }];

/**
 * After you play a Trap or a Field Trap (a cast is a play, R70), gain the card's mana. A permanent's
 * trigger reads its condition in `run` (a `when` predicate is a trap's, R99): any other play answers
 * with nothing.
 */
const afterYouPlayATrap: TriggerDef = {
  id: "toe-cracker-mana",
  on: ["cardPlayed"],
  run: (ctx) => {
    const event: GameEvent = ctx.event;
    if (event.type !== "cardPlayed" || event.player !== ctx.controller) return [];
    if (!TRAP_TYPES.includes(defOf(ctx.state, event.defId).type)) return [];
    return [gainMana({ amount: param(ctx, "mana") })];
  },
};

export const base: Script = { costAura: aura };

export const radiant: Script = { costAura: aura, triggers: [afterYouPlayATrap] };
