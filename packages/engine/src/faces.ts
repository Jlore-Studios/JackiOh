// A card's running face and what it says about the card (docs/classic-sets.md B2.7, SPEC §5.2).
//
// B2.7: a face may carry its own type (Classic+ #22 Blood Moon's Radiant face is a Field Trap), so a
// card's type is the running face's, and every rule that asks what an instance *is* reads it here.
// A definition read on its own — a pool, a catalog filter — reads `def.type`, the base face's.

import type { CardFace, CardType } from "@jackioh/shared";
import { defOf } from "./catalog";
import type { CardInstance, GameState } from "./state";

/** §5.2: the face the instance wears, radiant when it is. */
export function runningFace(state: GameState, instance: Pick<CardInstance, "defId" | "radiant">): CardFace {
  const def = defOf(state, instance.defId);
  return instance.radiant ? def.radiant : def.base;
}

/**
 * B2.7: the card's type now — its running face's own type, else its definition's.
 *
 * R383 (B3.1 rule 3): a card standing in a unit zone is a Unit for every rule, so an animated Field
 * Spell, Trap or Field Trap answers "Unit" while it stands there. Its trigger text still works the
 * way its face's type says — an animated Field Trap keeps firing as a trap — which is why the trap
 * machinery reads the face's own type (`animated.faceTypeOf`), never this.
 */
export function cardTypeOf(
  state: GameState,
  instance: Pick<CardInstance, "defId" | "radiant"> & { zone?: CardInstance["zone"] },
): CardType {
  if (instance.zone?.z === "field" && instance.zone.row === "units") return "Unit";
  return runningFace(state, instance).type ?? defOf(state, instance.defId).type;
}
