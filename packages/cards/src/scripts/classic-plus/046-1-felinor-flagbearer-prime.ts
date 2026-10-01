// C+ #46.1 Felinor Flagbearer Prime (SPEC §8.7 row 46.1). (2) Unit, Felinor, Token (printed
// Legendary), 5/5 → 10/10; the card C+ #46's Death shuffles in.
//   Base:    "Rush. Cry: Fill your board with copies of this. Aura: Your other Felinors have
//            +{aura}/+{aura}." — aura 1
//   Radiant: the same text, aura 2.
//   Engine:  "A unit-token card that lives in the deck and hand until it is played (R11). Its Cry fills
//            every empty, unlocked unit zone left to right (R64) with copies per R57 (Radiant flag,
//            buffs and granted keywords kept), which do not fire their Cry (R1), so nothing loops;
//            copies are not generation (R387). Every copy's aura lifts the others. Tunes: aura 1 ↑."
//
// "Fill your board" is one `summonCopy` per unit zone: each takes the leftmost empty, unlocked,
// unreserved zone (R64) and fizzles once none is left (§3.2), so a full board makes none. A copy is a
// summon, which fires no Cry (R1). The aura is #46's base aura ("your other Felinors").

import { UNIT_ZONES, defOf, param, type Script } from "@jackioh/engine";
import { summonCopy } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-046-1");

export const base: Script = {
  cry: () => Array.from({ length: UNIT_ZONES }, () => summonCopy({ of: { of: "self" } })),
  aura: (ctx) => {
    const amount = param(ctx, "aura");
    return [
      {
        applies: (unit) =>
          unit.controller === ctx.self.controller &&
          unit.zone.z === "field" &&
          unit.zone.row === "units" &&
          unit.id !== ctx.self.id &&
          defOf(ctx.state, unit.defId).tags.includes("Felinor"),
        mod: { attack: amount, maxHealth: amount },
      },
    ];
  },
};

// The same script: the Radiant face's 10/10 is catalog data and its +2/+2 its declared `aura`.
export const radiant: Script = base;
