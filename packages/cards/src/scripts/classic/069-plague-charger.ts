// C #69 Plague Charger (SPEC §8.6 row 69). (2) Unit, Rare, 4/2 → 8/4.
//   Base:    "Charge
//             Has First Strike while it has a Plague Token.
//             Has +{attack} Attack for each Plague Token on it." — +2
//   Radiant: the same text — +4
//   Engine:  "A conditional keyword (§6.1) and a self stat layer (§10.4) reading its own
//            `counters.plague`, both following the count as tokens are placed and consumed. Tunes:
//            attack per token 2 ↑."
//
// Charge is printed on both catalog faces (§10.4 layer 1).
//
// "Has First Strike while it has a Plague Token" is a keyword that holds only while a condition does
// (§6.1): the card's `conditionalKeywords`, read with its printed keywords on every read (§10.4), so it
// comes and goes with the tokens. "+{attack} Attack for each Plague Token on it" is §10.4 layer 5's
// "stats per Plague Token": an aura of this card on itself alone, its tokens times the declared
// `attack`. Both read the card's own counters, which R78 clears when it leaves the field, so a Charger
// with no token — in hand, or back on the field after a bounce — has neither. A Vanilla Charger has no
// text, so neither (§6.3). The aura's own attack reaches no other unit.
//
// R195: the printed condition is "while it has a Plague Token", read on the field (`conditionMet`,
// proved in `test/condition-active.test.ts`): it glows exactly while it has First Strike. A Charger in
// hand holds no tokens (R78), so it never glows there.
//
// The number is the declared `attack` (R386), read through `param` on the face it wears.

import { param, plagueOn, type CardInstance, type Script } from "@jackioh/engine";
import type { Keyword } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-069");

/** "while it has a Plague Token": the one predicate the keyword and the glow share. */
function hasPlague(self: CardInstance): boolean {
  return plagueOn(self) > 0;
}

const FIRST_STRIKE: Keyword = { kind: "First Strike" };

export const base: Script = {
  conditionalKeywords: ({ self }) => (hasPlague(self) ? [FIRST_STRIKE] : []),
  aura: ({ state, self, radiant }) => {
    const tokens = plagueOn(self);
    if (tokens === 0) return [];
    const perToken = param({ state, self, radiant }, "attack");
    return [{ applies: (unit) => unit.id === self.id, mod: { attack: tokens * perToken } }];
  },
  conditionMet: ({ self, zone }) => zone === "field" && hasPlague(self),
};

// The same script: the Radiant face's +4 is its declared `attack`, which `param` reads off the face it
// wears.
export const radiant: Script = base;
