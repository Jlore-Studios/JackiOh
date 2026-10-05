// C+ #56 Book of Pain (SPEC §8.7 row 56). (1) Spell, Book, Epic.
//   Base:    "Your opponent discards {discards|card|cards}." — discards 2
//   Radiant: the same text, discards 4.
//   Engine:  "Random from their hand (R661): no prompt opens; fewer cards → all they have, none →
//            nothing; a discarded unit-token card ceases to exist (R11). Tunes: discards 2 ↑."
//
// No prompt opens (R661), so the caster never waits on the opponent. The discard lands as the Cry
// reaches it.

import { param, type Script } from "@jackioh/engine";
import { discardRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-056");

export const base: Script = {
  cry: (ctx) => [discardRandom({ count: param(ctx, "discards"), player: "enemy" })],
};

// The same script: the Radiant face's 4 is its declared `discards`, which `param` reads.
export const radiant: Script = base;
