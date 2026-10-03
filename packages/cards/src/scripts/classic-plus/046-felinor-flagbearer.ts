// C+ #46 Felinor Flagbearer (SPEC §8.7 row 46). (2) Unit, Felinor, Legendary, 4/4 → 8/8.
//   Base:    "Rush. Aura: Your other Felinors have +{aura}/+{aura}. Death: Shuffle a Felinor Flagbearer
//            Prime into your deck." — aura 1 (no Cry, no Cleave, balance patch 1)
//   Radiant: the same with "Aura: Your Felinors have +{aura}/+{aura}." — aura 2
//   Engine:  "The aura (§10.4 layer 5) reaches your Felinor-tagged Units, the Radiant's including
//            itself. The Death shuffles a base C+ #46.1 in at a random position (§6.3), turned away at
//            R80's cap. Tunes: aura 1 ↑."
//
// The aura is computed on read (§10.4 layer 5): `applies` reads instance data only — controller,
// zone, def tags — never `unitView`, or the layers would recurse; a card dormant under a Stack pile
// is no unit on the field for it (§3.2, R13). The shuffle goes through `shuffleIntoLibrary`, so its
// owner is shown the card going in (R311) and a full library turns it away (R80).

import { defOf, param, type AuraHook, type Script } from "@jackioh/engine";
import { shuffleInto } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-046");

const PRIME = cardDef("classicplus-046-1").id;

/** "Your (other) Felinors have +N/+N": Felinor-tagged Units its controller has on the field. */
function felinorAura(includeSelf: boolean): AuraHook {
  return (ctx) => {
    const amount = param(ctx, "aura");
    return [
      {
        applies: (unit) =>
          unit.controller === ctx.self.controller &&
          unit.zone.z === "field" &&
          unit.zone.row === "units" &&
          (includeSelf || unit.id !== ctx.self.id) &&
          defOf(ctx.state, unit.defId).tags.includes("Felinor"),
        mod: { attack: amount, maxHealth: amount },
      },
    ];
  };
}

function flagbearer(includeSelf: boolean): Script {
  return {
    aura: felinorAura(includeSelf),
    death: () => [shuffleInto({ defId: PRIME, count: 1 })],
  };
}

export const base: Script = flagbearer(false);

export const radiant: Script = flagbearer(true);
