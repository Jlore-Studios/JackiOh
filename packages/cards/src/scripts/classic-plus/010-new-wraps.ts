// C+ #10 New Wraps (SPEC §8.7 row 10). (0) Spell, Common.
// A target Unit on either side gains Reborn as a granted keyword (§10.4, R78). Radiant: then the same
// Unit is made Radiant (§5.2: its face swaps at once, damage and buffs kept, no Cry).

import type { Script } from "@jackioh/engine";
import { grantKeyword, setRadiant } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-010");

// R656: Reborn helps, so a random cast that targets enemies aims this at friends.
const A_UNIT: TargetDecl[] = [
  { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] }, aim: "help" },
];

const giveReborn = grantKeyword({ target: { of: "chosen" }, keyword: { kind: "Reborn" } });

export const base: Script = { targets: A_UNIT, cry: () => [giveReborn] };

export const radiant: Script = { targets: A_UNIT, cry: () => [giveReborn, setRadiant()] };
