// C+ #48 Jlockheed's Lobbyist (SPEC §8.7 row 48). (1) Unit, Jlockeed, Legendary, 0/3 → 0/6.
//   Base:    "Can't be in Defense Position. Death: Add a random Jlockheed card to your hand. It costs (0)."
//   Radiant: "Death: Add a random Radiant Jlockheed card to your hand. It costs (0)."
//   Engine:  "The pool is the non-token cards with the `Jlockeed` tag (#13, #14, C #4, C+ #48, C+ #51,
//            C+ #52) but this one (R387): #13, #14, C #4, C+ #51 and C+ #52; `costOverride` 0; the hand
//            cap burns it (§2.4). The Defense restriction is a position validator flag (as #65.1's),
//            which the Radiant face drops. With 0 attack it never attacks. Tunes: none."
//
// The restriction is #65.1's `neverDefense` flag, which the switch action, `legalActions` and a switch
// made as an effect all honour (R20). The Death's pool is one tag (R278); `addRandomFromCatalog` leaves
// out the card running the hook by its def id (R387), and puts the (0) price on a card only once it is
// in the hand, so a burned one keeps none (§2.4, R4).

import type { Script } from "@jackioh/engine";
import { addRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-048");

export const base: Script = {
  staticFlags: { neverDefense: true },
  death: () => [addRandomFromCatalog({ query: { tags: ["Jlockeed"] }, costOverride: 0 })],
};

export const radiant: Script = {
  death: () => [addRandomFromCatalog({ query: { tags: ["Jlockeed"] }, costOverride: 0, radiant: true })],
};
