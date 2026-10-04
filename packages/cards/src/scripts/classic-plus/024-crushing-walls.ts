// C+ #24 Crushing Walls (SPEC §8.7 row 24): destroy the top card of every zone in lanes 1 and 5, both
// rows (Radiant: the enemy's only); a Unit standing on an Ivory Tower while its play resolves is passed
// by (R446, R651), and the Tower is destroyed whatever it has fused (R418).

import { isCarried, slotOf, type EffectContext, type Script } from "@jackioh/engine";
import { cardsInScope, destroy, forEachCard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-024");

/** Each player's outermost lanes (§3.1). */
const WALL_LANES: readonly number[] = [1, 5];

function walls(side: "any" | "enemy"): Script {
  const inTheWalls = (ctx: EffectContext): string[] =>
    cardsInScope(ctx, { side, rows: ["units", "backrow"] }).flatMap((card) => {
      if (isCarried(ctx.state, card)) return [];
      const at = slotOf(ctx.state, card);
      return at !== null && WALL_LANES.includes(at.lane) ? [card.id] : [];
    });
  return {
    cry: () => [forEachCard({ cards: inTheWalls, each: (instanceId) => destroy({ target: { of: "instance", instanceId } }) })],
  };
}

export const base: Script = walls("any");

export const radiant: Script = walls("enemy");
