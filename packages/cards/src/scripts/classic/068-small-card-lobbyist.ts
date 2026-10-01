// C #68 Small Card Lobbyist (SPEC §8.6 row 68, BUILD M9 Classic row C 68). (4) Unit 11/13 → 22/26, Common.
//   Base:    "Aura: ({threshold})+ Cost cards cost ({surcharge}) more." (3, 1)
//   Radiant: "Aura: Your opponent can't play ({threshold})+ Cost cards." (3)
//   Engine:  "Cost (§6.3, R65) on both players' cards where a play takes them from (a hand, or a
//            graveyard a permission lets its owner play from, §6.3 Play), as a price for a play; '(3)+'
//            is read where R363 reads Professor Curvature's '(4)+', on the cost before this aura adds
//            its (1). Radiant: `legalActions` offers the opponent no play of a card that costs (3) or
//            more at that moment (R65); casts (R70) are not plays from hand and are unaffected. Tunes:
//            surcharge 1 ↑; threshold 3 ↓."
//
// The aura is B5 E15's price rule (`Script.costAura`), laid while the Lobbyist acts on the field
// (the top of its pile) and read by R65's `effectiveCost` on every card a play would take:
//   - Base: a threshold rung on every player's cards, `minCost` the declared threshold and `amount` the
//     declared surcharge; the ladder tests the threshold against the price the flat rungs left (R363),
//     before this rule adds its own, so a (2) Cost card is never lifted into range. An X-cost card
//     costs exactly its X and takes no rule (R65); a cast pays nothing (R70).
//   - Radiant: a ban (`ban: true`) on the opponent's cards whose finished price is the threshold or
//     more (an X card at its chosen X included): `legalActions` never offers such a play and §10.5
//     step 1 refuses it, read last (R65); its controller's own plays and every cast are untouched.
// Both numbers are declared and read through `param` (R386); "threshold ↓" moves toward harder for a
// Degrade — up, so fewer cards are caught.

import type { CostAuraArgs, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-068");

export const base: Script = {
  costAura: (args: CostAuraArgs) => [
    { whose: "all", minCost: param(args, "threshold"), amount: param(args, "surcharge") },
  ],
};

export const radiant: Script = {
  costAura: (args: CostAuraArgs) => [{ whose: "opponents", minCost: param(args, "threshold"), ban: true }],
};
