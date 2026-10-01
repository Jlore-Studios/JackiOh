// C+ #12 The Mother Pancake (SPEC §8.7 row 12): (3) Unit, Pancake, Legendary, 8/8 → 16/16.
//   Base:    "Taunt. End of turn: Add {tokens} random Pancake token(s) to your hand."
//   Radiant: the same at 16/16 with 2 tokens (the catalog's `tokens` param).
// The pool is the eight Pancake tokens, C+ #12.1–#12.8, which the text names (§5.1), on their base
// faces, repeats allowed (R60); a full hand burns what doesn't fit (§2.4).

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-012");

export const base: Script = {
  // The eight Pancake tokens: Pancake- and Token-tagged (Mother Pancake and Mommy Barker are no tokens).
  endOfTurn: (ctx) => [addRandomFromCatalog({ query: { tags: ["Pancake", "Token"] }, count: param(ctx, "tokens") })],
};

/** The Radiant face differs only in its stats and its `tokens` value, both catalog data. */
export const radiant: Script = base;
