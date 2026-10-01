// C+ #56 Book of Pain (SPEC §8.7 row 56). (1) Spell, Book, Epic.
//   Base:    "Your opponent discards {discards|card|cards}." — discards 2
//   Radiant: the same text, discards 4.
//   Engine:  "Their choice (R16): a hand prompt the opponent holds during your turn, with its own clock
//            (R79); fewer cards → all they have, none → nothing; a discarded unit-token card ceases to
//            exist (R11). Tunes: discards 2 ↑."
//
// One hand prompt over their own hand, answered by them (`chooseFromHand({ of: "enemy", by: "enemy" })`,
// as Classic #8's discard), asking for min(N, hand size) cards; an empty hand opens none. The caster
// reads only that a prompt is open (§10.6, R177). The answer continues as this card's controller.

import { param, type Script } from "@jackioh/engine";
import { chooseFromHand, discard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-056");

export const base: Script = {
  cry: (ctx) => {
    const count = param(ctx, "discards");
    return [
      chooseFromHand({
        of: "enemy",
        by: "enemy",
        count,
        step: "discarded",
        prompt: `Book of Pain: discard ${count === 1 ? "1 card" : `${count} cards`}`,
      }),
    ];
  },
  resume: {
    discarded: (ctx) => ctx.targets.map((_, index) => discard({ target: { of: "chosen", index } })),
  },
};

// The same script: the Radiant face's 4 is its declared `discards`, which `param` reads.
export const radiant: Script = base;
