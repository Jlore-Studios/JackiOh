// C+ #65.2 Normal Grape (SPEC §8.7 row 65.2). (1) Spell, Fruit, Token (printed Common).
//   Base:    "Deal {amount} damage to an enemy or heal an ally {amount}. Draw {draw}. Reduce its cost
//            by (1)."
//   Radiant: the same text, amount 4 and draw 4: "… Draw {draw}. Reduce their cost by (1)."
//   Engine:  "The target, any unit or hero, is declared at play (R81); an enemy is one the opponent
//            controls, the heroes included. Each drawn card gets `costMod` −1 (never an X-cost card,
//            R65) — one card, "its cost", on the base face; four cards, "their cost", on the Radiant;
//            a card cast on draw never reaches the hand (R58) and a burned one isn't there, so
//            neither gets it. Tunes: amount 2 ↑; draw 1 ↑."
//
// The target travels in the play (R81): any Unit or hero on either side. Whether it is an enemy is read
// as the Spell resolves (`damageEnemyOrHealFriend`, effects/fruit.ts): an enemy takes one §4.4 hit from
// this Spell (Spell Damage raises it), a friend is healed (§6.3 Heal, R19).
//
// "Draw N" is N separate draws (§2.4), one `drawPriced` each, so a cast-on-draw card that asks pauses
// the rest of the list and the answer makes the rest (R113, R117). Each draw prices only the card IT
// put in the hand: `costMod` −1, which R65 never lets reach an X-cost card; a card cast on draw, a
// burned card, a fatigue draw and a limited draw get nothing.

import { param, type Script } from "@jackioh/engine";
import { damageEnemyOrHealFriend, drawPriced } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-065-2");

/** §8.7: "Choose a Unit or hero" — either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

/** §8.7: "It costs (1) less" — a discount that stacks with every other modifier (R65). */
const DISCOUNT = -1;

export const base: Script = {
  targets,
  cry: (ctx) => [
    damageEnemyOrHealFriend({ amount: param(ctx, "amount") }),
    ...Array.from({ length: param(ctx, "draw") }, () => drawPriced({ costMod: DISCOUNT })),
  ],
};

// The same script: the Radiant face's 4 and 2 are its declared `amount` and `draw`, which `param` reads.
export const radiant: Script = base;
