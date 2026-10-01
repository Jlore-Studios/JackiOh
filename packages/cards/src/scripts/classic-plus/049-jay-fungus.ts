// C+ #49 Jay Fungus (SPEC §8.7 row 49). (2) Unit, Rare, 3/6 → 6/12.
//   Base:    "Taunt. End of turn: A random card in your hand costs ({discount}) less." — discount 2
//   Radiant: the same text, discount 20.
//   Engine:  "`costMod` −2 (Radiant −20) on a random card in your hand it can make cheaper: current cost
//            above 0 and not an X-cost card (R65: modifiers never reach X); with none, nothing. The cost
//            floors at 0 (§2.3). Tunes: discount 2 ↑."
//
// "End of turn" is its controller's (§6.2), so the opponent's turn end does nothing. The pick and the
// R129 rule that an empty choice draws nothing are `discountRandomInHand`'s (effects/perks.ts).

import { param, type Script } from "@jackioh/engine";
import { discountRandomInHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-049");

export const base: Script = {
  endOfTurn: (ctx) => [discountRandomInHand({ amount: param(ctx, "discount") })],
};

// The same script: the Radiant face's 20 is its declared `discount`, which `param` reads.
export const radiant: Script = base;
