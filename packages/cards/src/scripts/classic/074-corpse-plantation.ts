// C #74 Corpse Plantation (SPEC §8.6 row 74). (2) Field Spell, Epic.
//   Base:    "Cry: Place {tokens|Plague Token|Plague Tokens} on this.
//             You may play Units from your graveyard, paying with Plague Tokens from this: each token pays
//             (1), and each such play spends at least 1 token." — 2 tokens; Radiant: 4.
//   Engine:  "A Field Spell's Cry fires as it is played. Play from the graveyard (§6.3 Play) for Units,
//            live while this is on the field, with a second way to pay: the `play` action carries how
//            many tokens pay (at least 1, at most the tokens on this and the price), the rest in mana, and
//            each paying token is removed from this card; so a Unit that costs (0) can't be played this
//            way. … its choices and counting are as from hand, and R65's player discounts apply. Tunes:
//            tokens 2 ↑."
//
// The Cry is one placement of {tokens} on itself (R386's declared number). The permission is
// `graveyardPlay` with `units` and `plague`: the engine offers and checks the token payment, takes the
// card from the graveyard as a play (its Cry fires, it counts as played; not R70's free cast) and removes
// the spent tokens from this card. Tokens other cards place here pay too; R78 clears them when it leaves.

import { param, type Script } from "@jackioh/engine";
import { placePlague } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-074");

export const base: Script = {
  cry: (ctx) => [placePlague({ target: { of: "self" }, amount: param(ctx, "tokens") })],
  graveyardPlay: () => [{ units: true, plague: true }],
};

// The same script: the Radiant face's 4 tokens are its declared `tokens`.
export const radiant: Script = base;
