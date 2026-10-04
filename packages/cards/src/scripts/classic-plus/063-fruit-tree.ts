// C+ #63 Fruit Tree (SPEC §8.7 row 63). (2) Field Spell, Fruit, Rare.
//   Base:    "Start of Turn: Add a random Fruit to your hand. It costs (0)."
//   Radiant: "Start of Turn: Add a random Radiant Fruit to your hand. It costs (0)."
//   Engine:  "Its controller's start of turn (R62). The Fruit pool (R382) but Fruit Tree (R387);
//            `costOverride` 0; the hand cap burns it (§2.4). Tunes: none."
//
// `startOfTurn` fires for the controller alone, on their own turn (`turn.startTurn` asks
// `triggerOrder` for that one player, §2.2, R62), so the opponent's start of turn adds nothing. The
// Fruit pool is the engine's: a `tags: ["Fruit"]` query holds the non-token Fruit cards of every set
// and the five Grapes (R382), and `addRandomFromCatalog` leaves out the card running the script by
// its def id (R387). Repeats are allowed (R60); a full hand burns what doesn't fit (§2.4, R4), and the
// cost rider lands only on a card that reached the hand.

import type { Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-063");

/** The printed "a random Fruit": one card per start of turn. */
const FRUITS = 1;

/** The two faces differ only in whether the Fruit is Radiant (R276: the Radiant face's proposal). */
function fruitTree(radiant: boolean): Script {
  return {
    startOfTurn: () => [
      addRandomFromCatalog({
        query: { tags: ["Fruit"] },
        count: FRUITS,
        costOverride: 0,
        ...(radiant ? { radiant: true } : {}),
      }),
    ],
  };
}

export const base: Script = fruitTree(false);

export const radiant: Script = fruitTree(true);
