// C+ #77 Anti-Softlock (SPEC §8.7 row 77, E20, E21, E38, R81, R346, R386, R440). (2) Spell, Rare.
//   Draw {draw}. Every card on the field, in hands and in decks gains Stack and Pierce. Unlock every
//   zone. Radiant: Draw {draw}. Choose all cards or only yours: each of them gains Stack and Pierce.
//   Unlock every zone.
//
// In the order written, so the drawn card gains the keywords too. The grants are E38's: on the field
// at once, in a hand or a deck carried onto the field as the card enters, silent on a card someone may
// not read (R440). This card, resolving, is in none of those zones, and a card dormant under a Stack is
// not on the field. Stack on a backrow card lets it be played onto an occupied backrow zone (E21); on a
// Spell, Pierce is R346's and Stack does nothing. Unlock clears every Locked zone of both players and
// leaves a reserved one reserved. The Radiant's choice is a mode declared at play (R81).

import { param, type EffectContext, type Script } from "@jackioh/engine";
import { chosenOptions, draw, grantKeywordCards, unlockAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-077");

const ALL = "All cards";
const YOURS = "Only yours";

function antiSoftlock(side: (ctx: EffectContext) => "any" | "self"): Script["cry"] {
  return (ctx) => {
    const scope = { side: side(ctx), zones: ["field" as const, "hand" as const, "library" as const] };
    return [
      draw({ count: param(ctx, "draw") }),
      grantKeywordCards({ scope, keyword: { kind: "Stack" } }),
      grantKeywordCards({ scope, keyword: { kind: "Pierce" } }),
      unlockAll(),
    ];
  };
}

export const base: Script = { cry: antiSoftlock(() => "any") };

export const radiant: Script = {
  modes: [{ kind: "mode", options: [ALL, YOURS] }],
  cry: antiSoftlock((ctx) => (chosenOptions(ctx)[0] === YOURS ? "self" : "any")),
};
