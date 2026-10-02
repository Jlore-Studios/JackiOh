// C+ #55 Book of Greed (SPEC §8.7 row 55). (1) Spell, Book, Epic.
//   Base:    "Add {cards|random Legendary or Mythic card|…} to your hand." — cards 3
//   Radiant: "Add {cards|random Radiant Legendary or Mythic card|…} to your hand."
//   Engine:  "Non-token cards whose rarity is Legendary or Mythic, every set (R380); a token's rarity is
//            "Token" and its `printedRarity` never feeds a pool (§5), so none is found; repeats allowed
//            (R60); the hand cap burns extras. Tunes: cards 3 ↑."

import { param, type Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-055");

function bookOfGreed(radiant: boolean): Script {
  return {
    cry: (ctx) => [addRandomFromCatalog({ query: { rarity: ["Legendary", "Mythic"] }, count: param(ctx, "cards"), radiant })],
  };
}

export const base: Script = bookOfGreed(false);

export const radiant: Script = bookOfGreed(true);
