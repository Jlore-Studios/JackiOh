// C #85 King Wagtoggle (SPEC §8.6 row 85). (4) Unit, Legendary, 5/5 → 10/10.
//   Base:    "Cry: Swap decks with your opponent."
//   Radiant: "Cry: Swap decks with your opponent. Then Recruit {recruits|card|cards}." — recruits 1
//   Engine:  "R73's library swap: contents swap, each swapped card's owner becomes the player whose
//            deck now holds it (R12), and fatigue counters stay. The Radiant recruits (Recruit, §6.3)
//            the first permanent from the top of your new deck. Tunes: Radiant recruits 1 ↑."
//
// `swapLibrary` is #87 Pocket Chaos's swap (R73): the libraries change places whole, empty ones too, and
// each card's owner becomes the player whose library now holds it (R12); the fatigue counts are the
// players' and stay. What each player may know of their new library is R311's: the list shows only
// what its owner was shown going in, and no event carries a position. The Radiant face then Recruits
// (§6.3) from the new library: the first permanent from the top, summoned per R64 (a Trap face-down,
// R33); none, or a full row, summons nothing. "Recruit N" is N scans, the declared `recruits` read
// through `param` (R386).
//
// Rulings: R12, R33, R73, R311, R386. Its proof: `test/classic/085-king-wagtoggle.test.ts`.

import { param, type Script } from "@jackioh/engine";
import { recruit, swapLibrary } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-085");

export const base: Script = {
  cry: () => [swapLibrary()],
};

export const radiant: Script = {
  cry: (ctx) => [swapLibrary(), recruit({ count: param(ctx, "recruits") })],
};
