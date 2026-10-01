// C+ #23 Dropshipping (SPEC §8.7 row 23): add {cards} random cards from every card and token but this
// (R382, R387), each given Brittle {brittle} (R385); Radiant: each costs (1).

import { param, type EffectContext, type Script } from "@jackioh/engine";
import { addRandomFromCatalog, forEachCard, giveBrittle } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-023");

/** "Each costs (1)" on the Radiant face. */
const RADIANT_COST = 1;

/** The cards this list put in its controller's hand (R136: its own events, from `eventsFrom`). */
function addedByThisList(ctx: EffectContext): string[] {
  return ctx.events
    .slice(ctx.eventsFrom)
    .flatMap((event) => (event.type === "addedToHand" && event.player === ctx.controller ? [event.instanceId] : []));
}

function dropship(radiant: boolean): Script {
  return {
    cry: (ctx) => {
      const n = param(ctx, "brittle");
      return [
        addRandomFromCatalog({
          query: { withTokens: true, excludeDefId: def.id },
          count: param(ctx, "cards"),
          ...(radiant ? { costOverride: RADIANT_COST } : {}),
        }),
        forEachCard({ cards: addedByThisList, each: (instanceId) => giveBrittle({ instanceId, n }) }),
      ];
    },
  };
}

export const base: Script = dropship(false);

export const radiant: Script = dropship(true);
