// C+ #60 Doctors Orders (SPEC §8.7 row 60). (1) Field Spell, Rare.
//   Base:    "Activate: Add {apples|All Purpose Apple|All Purpose Apples} to your hand." — 1
//            (balance patch 1: the Cry and the start-of-turn trigger became one Activate)
//   Radiant: "Activate: Add {apples|Radiant All Purpose Apple|…} to your hand."
//   Engine:  "A Field Spell (R421). The Apple is C+ #59, named, so no pool; the hand cap burns it
//            (§2.4). No Fruit tag. Tunes: apples 1 ↑."

import { param, type ActivationDecl, type Script } from "@jackioh/engine";
import { addToHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-060");

const APPLE = cardDef("classicplus-059").id;

function doctorsOrders(radiant: boolean): Script {
  const order: ActivationDecl = {
    id: "order",
    label: "Add an All Purpose Apple to your hand",
    uses: 1,
    run: (ctx) => Array.from({ length: param(ctx, "apples") }, () => addToHand({ defId: APPLE, radiant })),
  };
  return { activations: [order] };
}

export const base: Script = doctorsOrders(false);

export const radiant: Script = doctorsOrders(true);
