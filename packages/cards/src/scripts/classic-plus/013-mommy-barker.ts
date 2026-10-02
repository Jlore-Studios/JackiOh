// C+ #13 Mommy Barker (SPEC §8.7 row 13): (1) Unit, Human, Pancake, Legendary, 2/2 → 4/4.
//   Base:    "Death: Add {tokens} random Pancake token(s) to your hand."
//   Radiant: Reborn, and the same Death, which fires on both deaths (§4.5).
// The pool is C+ #12's, the eight Pancake tokens, on their base faces (R60); a full hand burns it.

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-013");

export const base: Script = {
  death: (ctx) => [addRandomFromCatalog({ query: { tags: ["Pancake", "Token"] }, count: param(ctx, "tokens") })],
};

/** Reborn is catalog data; the Death is the same. */
export const radiant: Script = base;
