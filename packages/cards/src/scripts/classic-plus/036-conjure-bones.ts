// C+ #36 Conjure Bones (SPEC §8.7 row 36): (2) embiggen (4) Spell, Rare.
//   Base:    "Shuffle {count} Bone Storms into your deck. Paid (4): {paidCount} instead."
//   Radiant: the same, the Bone Storms Radiant.
// The embiggen price is a play-time choice that lands on the instance (R81, `ctx.embiggened`);
// `shuffleInto` puts each fresh token at a random position and R80's cap turns the rest away.

import type { Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { shuffleInto } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-036");

const BONE_STORM = "classicplus-036-1";

function conjureBones(radiant: boolean): Script {
  return {
    cry: (ctx) => [
      shuffleInto({
        defId: BONE_STORM,
        count: param(ctx, ctx.embiggened ? "paidCount" : "count"),
        radiant,
      }),
    ],
  };
}

export const base: Script = conjureBones(false);

export const radiant: Script = conjureBones(true);
