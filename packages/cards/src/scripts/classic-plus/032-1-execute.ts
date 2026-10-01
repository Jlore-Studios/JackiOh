// C+ #32.1 Execute (SPEC §8.7 row 32.1; §4, §6.3 Destroy, R46, R59, R81). (1) Spell token, printed Epic.
//   Base:    "Destroy a damaged Unit."
//   Radiant: "Destroy every damaged enemy Unit."
//
// A damaged Unit has damage above 0 (§4): its health below its max health through the §10.4 layers.
// The base face's target is one damaged Unit on either side, declared with the play (R81, the
// `damaged` filter), so an undamaged Unit is no legal target; with none the Spell fizzles and still
// counts as played (§8's Conventions). The Radiant face names no target: every damaged enemy Unit it
// may affect is marked at once (an Immune to Spells Unit is not among them), the one state check after
// the list collecting them together (R59). An Indestructible Unit survives either way (R46).

import { unitView, type EffectContext, type Script } from "@jackioh/engine";
import { cardsInScope, destroy, forEachCard } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-032-1");

/** R81: one damaged Unit, either side. */
const DAMAGED_UNIT: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"], damaged: true } }];

/** §4: damage above 0, read through the layers. */
function damagedEnemyUnits(ctx: EffectContext): string[] {
  return cardsInScope(ctx, { side: "enemy" })
    .filter((unit) => {
      const view = unitView(ctx.state, unit);
      return view.health < view.maxHealth;
    })
    .map((unit) => unit.id);
}

export const base: Script = {
  targets: DAMAGED_UNIT,
  cry: () => [destroy({ target: { of: "chosen" } })],
};

export const radiant: Script = {
  cry: () => [
    forEachCard({
      cards: damagedEnemyUnits,
      each: (instanceId) => destroy({ target: { of: "instance", instanceId } }),
    }),
  ],
};
