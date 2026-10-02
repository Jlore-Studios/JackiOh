// C+ #20 Mushroom Power (SPEC §8.7 row 20): Cry gives the Units beside it (§3.1) +{buff}/+{buff}.

import { param, type Script } from "@jackioh/engine";
import { adjacentTo, buff, forEachCard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-020");

export const base: Script = {
  cry: (ctx) => {
    const amount = param(ctx, "buff");
    return [
      forEachCard({
        cards: (each) => adjacentTo(each, { of: "self" }),
        each: (instanceId) => buff({ target: { of: "instance", instanceId }, attack: amount, health: amount }),
      }),
    ];
  },
};

// The same script: the Radiant +4/+4 is its declared `buff`.
export const radiant: Script = base;
