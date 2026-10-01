// C+ #59 All Purpose Apple (SPEC §8.7 row 59). (1) Spell, Fruit, Rare.
//   Base:    "Summon a Rush Token. Heal your hero {heal}. Deal {damage} damage." — heal 2, damage 1
//   Radiant: "Summon a Radiant Rush Token. Heal your hero {heal}. Deal {damage} damage." — 4, 2
//   Engine:  "Resolves in the order written. The damage's target, any unit or hero, is declared at play
//            (R81); a full board summons nothing and the rest still happens. Tunes: heal 2 ↑; damage 1 ↑."
//
// The token takes the leftmost free unit zone (R64) and fizzles on a full row (§3.2); a hero heal has
// no cap (§3, R19); the hit is one §4.4 instance from this Spell, which Spell Damage raises (E6).

import { param, type Script } from "@jackioh/engine";
import { damage, heal, summon } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-059");

const RUSH_TOKEN = cardDef("core-t-rush").id;

/** §8 Conventions: "Deal N damage" with no target named is targeted — any unit or hero, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

function apple(radiant: boolean): Script {
  return {
    targets,
    cry: (ctx) => [
      summon({ defId: RUSH_TOKEN, radiant }),
      heal({ target: { of: "selfHero" }, amount: param(ctx, "heal") }),
      damage({ to: { of: "chosen" }, amount: param(ctx, "damage") }),
    ],
  };
}

export const base: Script = apple(false);

export const radiant: Script = apple(true);
