// C+ #46 Felinor Flagbearer (SPEC §8.7 row 46). (2) Unit, Felinor, Legendary, 4/4 → 8/8.
//   Base:    "Rush, Cleave. Cry: Your hero gains +{armor} Armor for the rest of the game. Aura: Your
//            other Felinors have +{aura}/+{aura}. Death: Shuffle a Felinor Flagbearer Prime into your
//            deck." — armor 1, aura 1
//   Radiant: the same with "Aura: Your Felinors have +{aura}/+{aura}." — armor 2, aura 2
//   Engine:  "Hero Armor is §4.4 step 2's per-hit reduction: `hero.armor` +1 (Radiant +2), kept for the
//            game and stacking with every such Cry. The aura (§10.4 layer 5) reaches your Felinor-tagged
//            Units, the Radiant's including itself. The Death shuffles a base C+ #46.1 in at a random
//            position (§6.3), turned away at R80's cap. Tunes: Armor 1 ↑; aura 1 ↑."
//
// The Armor lives on the hero (`gainHeroArmor`, effects/perks.ts), so it outlasts the card. The aura is
// computed on read (§10.4 layer 5): `applies` reads instance data only — controller, zone, def tags —
// never `unitView`, or the layers would recurse; a card dormant under a Stack pile is no unit on the
// field for it (§3.2, R13). The shuffle goes through `shuffleIntoLibrary`, so its owner is shown the
// card going in (R311) and a full library turns it away (R80).

import { defOf, param, type AuraHook, type Script } from "@jackioh/engine";
import { gainHeroArmor, shuffleInto } from "@jackioh/engine/effects";
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
    cry: (ctx) => [gainHeroArmor({ amount: param(ctx, "armor") })],
    aura: felinorAura(includeSelf),
    death: () => [shuffleInto({ defId: PRIME, count: 1 })],
  };
}

export const base: Script = flagbearer(false);

export const radiant: Script = flagbearer(true);
