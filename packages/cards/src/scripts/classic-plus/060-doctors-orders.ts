// C+ #60 Doctors Orders (SPEC §8.7 row 60). (1) Field Spell, Rare.
//   Base:    "Cry and start of turn: Add {apples|All Purpose Apple|All Purpose Apples} to your hand." — 1
//   Radiant: "Cry and start of turn: Add {apples|Radiant All Purpose Apple|…} to your hand."
//   Engine:  "A Field Spell (R421) … The Cry fires as it enters from a play (§6.2); the start-of-turn
//            trigger is its controller's (R62). The Apple is C+ #59, named, so no pool; the hand cap
//            burns it (§2.4). No Fruit tag. Tunes: apples 1 ↑."

import { param, type Hook, type Script } from "@jackioh/engine";
import { addToHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-060");

const APPLE = cardDef("classicplus-059").id;

function doctorsOrders(radiant: boolean): Script {
  const apples: Hook = (ctx) => Array.from({ length: param(ctx, "apples") }, () => addToHand({ defId: APPLE, radiant }));
  return { cry: apples, startOfTurn: apples };
}

export const base: Script = doctorsOrders(false);

export const radiant: Script = doctorsOrders(true);
