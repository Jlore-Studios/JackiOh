// C+ #19.4 Support Loser (SPEC §8.7 row 19.4): end of turn, heal your hero, then each of your Units,
// {heal}.

import { param, type Script } from "@jackioh/engine";
import { cardsInScope, forEachCard, heal } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-019-4");

export const base: Script = {
  endOfTurn: (ctx) => {
    const amount = param(ctx, "heal");
    return [
      heal({ target: { of: "selfHero" }, amount }),
      forEachCard({
        cards: (each) => cardsInScope(each, { side: "self" }),
        each: (instanceId) => heal({ target: { of: "instance", instanceId }, amount }),
      }),
    ];
  },
};

// The same script: Reborn and the Radiant heal 6 are catalog data.
export const radiant: Script = base;
