// C+ #69 Buff Billy (SPEC §8.7 row 69, R348, R386, R396; BUILD M9 row C+ 69). (X) Unit, Human, Rare.
//   Base:    "This is a 3X/3X. / Cry: Upgrade this X times."
//   Radiant: "This is a 7X/7X. / Cry: Upgrade this 2X times."
//
// The 3X/3X (7X/7X) is the catalog's `xStats` (E40), read by the engine's layer 1 off the X the card
// was played for (at least 1, R348); a Recruit or a summon has no X and arrives 0/0 to die at the
// state check. The Cry is X (2X) separate Upgrades of itself (R386), each drawn from what fits it on
// the field: its X has resolved there, so only the stats and keyword rows remain (R396).

import type { Script } from "@jackioh/engine";
import { upgrade } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-069");

/** The Radiant face's "2X times". */
const RADIANT_UPGRADES_PER_X = 2;

export const base: Script = {
  cry: (ctx) => [upgrade({ target: { of: "self" }, times: ctx.x })],
};

export const radiant: Script = {
  cry: (ctx) => [upgrade({ target: { of: "self" }, times: RADIANT_UPGRADES_PER_X * ctx.x })],
};
