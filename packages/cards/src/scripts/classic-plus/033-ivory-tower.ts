// C+ #33 Ivory Tower (SPEC §8.7 row 33, R418): (2) Field Spell, Rare.
//   Base:    "Aura: Your cards have Stack. A Unit may be played on top of this. That Unit can't attack
//            or be attacked."
//   Radiant: the same, and that Unit becomes Radiant.
// The engine owns the arrangement (B5 E21, R446): `carrier` lets a Unit name this zone and stand on
// it as a Unit for every rule that can neither attack nor be attacked, the Tower acting beneath it,
// and steps it down into a unit zone when the Tower leaves. The aura reaches hands too (§10.4, E38).

import type { CardInstance, GameState, Script } from "@jackioh/engine";
import { carriedAt, slotOf } from "@jackioh/engine";
import { setRadiant } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-033");

/** The Unit this Tower carries now, if any. */
function riderOf(state: GameState, tower: CardInstance): CardInstance | null {
  const at = slotOf(state, tower);
  return at === null ? null : carriedAt(state, at);
}

export const base: Script = {
  staticFlags: { carrier: true },
  aura: ({ self }) => [
    { applies: (card) => card.controller === self.controller, mod: { keywords: [{ kind: "Stack" }] } },
  ],
};

export const radiant: Script = {
  ...base,
  triggers: [
    {
      id: "ivory-tower-radiant",
      on: ["cardPlayed"],
      // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
      run: (ctx) =>
        ctx.self !== null && ctx.event.type === "cardPlayed" && riderOf(ctx.state, ctx.self)?.id === ctx.event.instanceId
          ? [setRadiant({ instanceId: ctx.event.instanceId })]
          : [],
    },
  ],
};
