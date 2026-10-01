// C+ #54 Book of Books (SPEC §8.7 row 54). (1) Spell, Book, Epic.
//   Base:    "Add {books|random Book|random Books} to your hand. Each costs (0)." — books 2
//   Radiant: "Add {books|random Radiant Book|random Radiant Books} to your hand. Each costs (0)."
//   Engine:  "Non-token Books of every set (R380) but this one (R387), repeats allowed (R60);
//            `costOverride` 0; the hand cap burns extras (§2.4). Tunes: books 2 ↑."
//
// One tag pool; `addRandomFromCatalog` leaves out the card running the script by its def id (R387) and
// prices only a card that reached the hand (§2.4, R4).

import { param, type Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-054");

function bookOfBooks(radiant: boolean): Script {
  return {
    cry: (ctx) => [addRandomFromCatalog({ query: { tags: ["Book"] }, count: param(ctx, "books"), costOverride: 0, radiant })],
  };
}

export const base: Script = bookOfBooks(false);

export const radiant: Script = bookOfBooks(true);
