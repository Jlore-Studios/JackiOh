// "Summon this from your hand or deck" (docs/classic-sets.md B5 E26): the verb a hand or deck
// trigger ends in (Classic #66 EU Striker, Classic+ #37 Wardrum). It is §6.3's Summon of a card that
// already exists: no Cry (R1), into its controller's leftmost open, unlocked, unreserved zone of its
// row (R64), summoning sick for the turn it arrives on (§4.1), and nothing at all when that row is
// full. Only a card in a hand or a library moves: a trigger that has left those zones by the time it
// resolves has nothing to summon.

import type { Effect } from "../script";
import { summon } from "./summon";

/** B5 E26: summon the card running the script out of its hand or its library (no Cry, R64). */
export function summonThis(): Effect {
  return {
    kind: "summonThis",
    apply(ctx): void {
      const zone = ctx.self?.zone.z;
      if (zone !== "hand" && zone !== "library") return;
      summon({ instance: { of: "self" } }).apply(ctx);
    },
  };
}
