// C #89 Paul Allen's Ghost (SPEC §8.6 row 89, BUILD M9 Classic row C 89). (2) Unit 5/6 → 10/12, Rare.
//   Base:    "Divine Shield / To target this with anything but an attack, a player must also discard
//            {discard|card|cards}." (2)
//   Radiant: "Divine Shield, Reborn / (the same)."
//   Engine:  "A replacement at 'a friendly unit is targeted' (§6.2 Replacement) that adds a cost: a
//            declared target (a play or an activation) naming it costs 2 discards, random at pay time
//            (balance patch 1, R682), and the action carries none; a prompt answer naming it pays them
//            before it goes on. With fewer than 2 other cards in hand it is not a legal target. It binds
//            both players, its controller included. 'Target' is as R394 reads it: a declared or
//            prompted pick, while random picks and 'all' effects target nothing. Tunes: discard 2 ↑."
//
// The cost is B5 E5's targeting point (`Script.targetingDiscards`): a pure read of how many cards
// targeting this card costs now, asked while it acts on the field. The engine does the rest: a play's
// or an activation's declared pick naming it is offered only to a chooser who holds that many other
// hand cards (`legalActions`, one action with no paying set) and refused otherwise, and §10.5 step 2
// pays the cost in random discards (R682); a prompt's pick naming it is offered only to a chooser who
// can pay, and the answer pays at random before it goes on; the discards are §6.3 Discards (C #64
// sees them). An attack is no targeting, nor is a random pick or an "all" effect. The number is the
// declared one, `param(…, "discard")` (R386) — "↑" is better for its controller, so an Upgrade
// raises it. Divine Shield, and Reborn on the Radiant face, are printed.

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-089");

export const base: Script = {
  targetingDiscards: (args) => param(args, "discard"),
};

// The same script: the Radiant face differs only in what the engine reads off the catalog (its doubled
// stats and Reborn).
export const radiant: Script = base;
