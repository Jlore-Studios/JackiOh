// C+ #65.3 Large Grape (SPEC §8.7 row 65.3). (3) Spell, Fruit, Token (printed Rare).
//   Base:    "Choose a Unit or hero. If it's an enemy, deal {amount} damage to it; if it's yours, heal
//            it {amount}. Draw {draw}. Each costs (0)."
//   Radiant: the same text, amount 10 and draw 2.
//   Engine:  "As C+ #65.2, the drawn card taking `costOverride` 0. Tunes: amount 5 ↑; draw 1 ↑."
//
// C+ #65.2 Normal Grape with a set price in place of the discount: each card a draw put in the hand
// costs (0) (`costOverride`, R65), which an X-cost card's X never minds. A card cast on draw, a burned
// card, a fatigue draw and a limited draw get nothing (`drawPriced`, effects/fruit.ts).

import { param, type Script } from "@jackioh/engine";
import { damageEnemyOrHealFriend, drawPriced } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-065-3");

/** §8.7: "Choose a Unit or hero" — either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

/** §8.7: "It costs (0)". */
const SET_COST = 0;

export const base: Script = {
  targets,
  cry: (ctx) => [
    damageEnemyOrHealFriend({ amount: param(ctx, "amount") }),
    ...Array.from({ length: param(ctx, "draw") }, () => drawPriced({ costOverride: SET_COST })),
  ],
};

// The same script: the Radiant face's 10 and 2 are its declared `amount` and `draw`, which `param` reads.
export const radiant: Script = base;
