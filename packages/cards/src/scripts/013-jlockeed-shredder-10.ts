// #13 Jlockeed Shredder-10 (SPEC §8.1): 8/10 → 16/20 Unit, cost 3. Base "End of turn: deal 2 damage
// to each enemy unit and the enemy hero", radiant "5 damage" — a radiant cell that changes only a
// number changes only that number (§8 Conventions). §8's Engine cell: "One damage instance per
// target, controller's end of turn".
//
// R51 ("all enemies"): every enemy unit plus the enemy hero, ONE damage instance each — not one
// area effect, so each hit goes through §4.4's pipeline on its own and meets that target's Armor,
// Divine Shield and Anti-oneshot cap by itself.
//
// R59 (when the state check runs): "after each action, each whole effect or trigger … never between
// the hits of one effect". So a unit the first hit kills is still on the field while the rest land,
// and every death happens together once the hook has finished. `endTurn` in `engine/src/turn.ts`
// runs `stateCheck` after each end-of-turn hook, which is exactly that point.
//
// "The controller's end of turn" is not this card's business either: `triggerOrder(sink,
// "endOfTurn", player)` narrows the scan to the player whose turn is ending, so the hook simply
// never runs on the opponent's end of turn (the comment there cites this card).
//
// The hits are `damageAll` (engine/src/effects/damage.ts): one `dealDamage` per enemy unit in lane
// order and then, with `heroes`, one to the enemy hero — all inside ONE effect, so no state check
// runs between the hits (R59).

import type { Effect, Script } from "@jackioh/engine";
import { damageAll } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-013");

/** R51: every enemy unit and the enemy hero, one instance each, in one effect (R59). */
function shred(amount: number): Effect {
  return damageAll({ side: "enemy", amount, heroes: true });
}

export const base: Script = {
  endOfTurn: () => [shred(2)],
};

export const radiant: Script = {
  endOfTurn: () => [shred(5)],
};
