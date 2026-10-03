// C+ #78 Claude's Datacenter (SPEC §8.7 row 78, B7). (2) Field Spell, Legendary.
//   Base:    "End of turn: Add {cards|random AI generated card|random AI generated cards} to your hand.
//            Each costs (0)."
//   Radiant: "… random Radiant AI generated card …"
//   Engine:  "Its controller's end of turn (R62). The pool is the ten AI generated cards (T-AI-1 to
//            T-AI-10, §7), named by the text, so tokens reach it; `costOverride` 0; the hand cap burns
//            it (§2.4). Tunes: cards 1 ↑."
//
// An `endOfTurn` hook runs on its controller's turn only (§6.2). Each card is its own pick (R60) through
// §6.3's Add to hand, so a full hand burns it, and the opponent sees the sentinel (R97).

import type { Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-078");

/** §8.7: "a random AI generated card" — the AI tag's ten tokens, which a pool names by asking for tokens. */
const AI_POOL = { tags: ["AI" as const], token: true };

/** §8.7: "It costs (0)". */
const FREE = 0;

/** The printed "a random AI generated card": one card per end of turn. */
const CARDS = 1;

function datacenter(radiant: boolean): Script {
  return {
    endOfTurn: () => [
      addRandomFromCatalog({ query: AI_POOL, count: CARDS, costOverride: FREE, ...(radiant ? { radiant } : {}) }),
    ],
  };
}

export const base: Script = datacenter(false);

export const radiant: Script = datacenter(true);
