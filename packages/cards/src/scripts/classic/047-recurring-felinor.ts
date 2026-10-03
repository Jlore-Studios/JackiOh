// C #47 Recurring Felinor (SPEC §8.6 row 47; §6.3 Cast; R4, R68, R70, R78, R87, R386). Unit, Felinor,
// 3/2 → 6/4, cost 2, Rare.
//   Base:    "Cry: Cast Ancient Acquisition.\nWhile this is in your graveyard: When one of your Traps
//            activates, Bounce this."
//   Radiant: "… Bounce this. It costs ({returnCost})." (0)
//   Engine:  the Cry casts a generated C #34 on its base face, free, with your picks, then to your
//            graveyard (R70, R87). The return is a graveyard trigger on your `trapFired`, a Field Trap's
//            firing included, live only while this card is in your graveyard; the hand cap applies (R4).
//            Radiant: it returns with `costOverride` 0, which persists in every zone (R78).
//
// A graveyard trigger's `when` is not consulted outside the trap window, so "one of your Traps" is
// checked in `run`, which answers anything else with no effects.

import { param, type EffectContext, type Script, type TriggerDef } from "@jackioh/engine";
import { addToHand, castNew } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-047");

const ANCIENT_ACQUISITION = "classic-034";

function recur(costOverride?: (ctx: EffectContext) => number): TriggerDef {
  return {
    id: "recurring-felinor",
    on: ["trapFired"],
    run: (ctx) => {
      const event: GameEvent = ctx.event;
      if (event.type !== "trapFired" || event.controller !== ctx.controller) return [];
      return [addToHand({ instance: { of: "self" }, ...(costOverride === undefined ? {} : { costOverride: costOverride(ctx) }) })];
    },
  };
}

const cry: Script["cry"] = () => [castNew({ def: { defId: ANCIENT_ACQUISITION, radiant: false } })];

export const base: Script = { cry, graveyardTriggers: [recur()] };

export const radiant: Script = { cry, graveyardTriggers: [recur((ctx) => param(ctx, "returnCost"))] };
