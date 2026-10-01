// T-AI-7 Alignment Tax (SPEC §8.7 row T-AI-7, §7, B8). (1) Spell, AI, Token.
//   Base:    "Your opponent's cards cost (1) more during their next turn."
//   Radiant: "Your opponent's cards cost (2) more during their next turn."
//   Engine:  "A player modifier on the opponent (Cost, §6.3, R65), #77 Professor Curvature's timing
//            turned outward (R48, R363): it holds only during their next turn and expires at its
//            cleanup; X-cost cards ignore it (R65). Tunes: none."
//
// E15's price rule on the opponent (`addCostRule`, R455) lasting "theirNextTurn": R48's `nextTurnOf`
// expiry, live only once they are the active player on a later turn and gone at that turn's cleanup.
// Two Taxes are two modifiers, and they add.

import type { Script } from "@jackioh/engine";
import { addCostRule } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-07");

/** §8.7: "(1) more", Radiant "(2) more". An AI card declares no params (B8), so its numbers live here. */
const TAX = { base: 1, radiant: 2 } as const;

function tax(amount: number): Script {
  return { cry: () => [addCostRule({ player: "enemy", rule: { amount }, lasts: "theirNextTurn" })] };
}

export const base: Script = tax(TAX.base);

export const radiant: Script = tax(TAX.radiant);
