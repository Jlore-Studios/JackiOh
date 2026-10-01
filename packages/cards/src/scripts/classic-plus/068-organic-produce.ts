// C+ #68 Organic Produce (SPEC §8.7 row 68; BUILD M9 row C+ 68). (4) Field Spell, Fruit, Epic.
//   Base:    "Cry: Add {fruits|random Fruit|random Fruits} to your hand. / Aura: Fruits you play are Radiant." — fruits 1
//   Radiant: "Cry: Add {fruits|…} to your hand. Each costs (0). / (the same aura)" — fruits 2
//
// The Cry's pool is the Fruit pool (R382: the non-token Fruits and the five Grapes), never Organic
// Produce itself (R387: `addRandomFromCatalog` leaves out the running card's id), repeats allowed
// (R60). The aura is Core #64 Gifted Program's static flag at §10.5 step 3 by tag (R213, R214):
// every Fruit its controller plays, casts included, resolves on its Radiant face.

import { param, type Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-068");

/** The Radiant face's "Each costs (0)." */
const FREE = 0;

const staticFlags = { radiantPlaysTagged: ["Fruit" as const] };

export const base: Script = {
  staticFlags,
  cry: (ctx) => [addRandomFromCatalog({ query: { tags: ["Fruit"] }, count: param(ctx, "fruits") })],
};

export const radiant: Script = {
  staticFlags,
  cry: (ctx) => [addRandomFromCatalog({ query: { tags: ["Fruit"] }, count: param(ctx, "fruits"), costOverride: FREE })],
};
