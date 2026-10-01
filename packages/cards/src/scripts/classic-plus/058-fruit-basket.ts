// C+ #58 Fruit Basket (SPEC §8.7 row 58). (1) Spell, Fruit, Rare.
//   Base:    "Add {fruits|random Fruit|random Fruits} to your hand." — fruits 3
//   Radiant: "Add {fruits|random Radiant Fruit|random Radiant Fruits} to your hand."
//   Engine:  "The Fruit pool (R382): the non-token Fruit cards of every set plus the five Grapes (C+
//            #65.1 to C+ #65.5), Fruit Basket excluded (R387); each pick uniform over the pool, a Grape
//            one entry like any card; repeats allowed (R60); the hand cap burns extras. Tunes: fruits 3 ↑."
//
// A `tags: ["Fruit"]` query is the Fruit pool, Grapes included (R382); the verb excludes this card (R387).

import { param, type Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-058");

function fruitBasket(radiant: boolean): Script {
  return { cry: (ctx) => [addRandomFromCatalog({ query: { tags: ["Fruit"] }, count: param(ctx, "fruits"), radiant })] };
}

export const base: Script = fruitBasket(false);

export const radiant: Script = fruitBasket(true);
