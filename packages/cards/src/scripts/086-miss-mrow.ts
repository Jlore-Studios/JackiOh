// #86 "Miss" Mrow (SPEC §8.4, R12, R13, R15, R42, R78, R171, R361).
//
// Base: "Can't attack. Death: Take control of the Unit that destroyed this." Radiant: "Rush. Death:
// Take control of the Unit that destroyed this." (patch v0.1.1: the Death used to steal every enemy
// unit, and the Radiant face used to print Taunt).
//
// The Death clause is the same on both faces, so both faces run one Death hook and differ only in
// the printed face, which is the catalog's: the base face prints the keyword `Can't attack`, which
// `combat.ts`'s `whyAttackRefused` reads off `unitView(...).keywords`, and the radiant face prints
// `Rush` (§8 Conventions: a keyword list gives the face's complete list), so it may attack units the
// turn it lands. Neither needs a line of script.
//
// "The Unit that destroyed this" is R42's killer, which R361 makes a card-facing fact: the unit
// whose hit took Mrow to 0 health (or whose Poisonous hit marked it), read off Mrow's last-known
// state (R78) — the Death hook runs at §4.5 step 3 on that snapshot. `killerOf` answers it only while
// the killer is a Unit acting on the field, so a destroy effect, a Tribute, a Spell's damage, a
// killer that died in the same combat or one dormant under a Stack (R13) gives nothing to take.
//
// The take is one `steal`, which is §6.3's Steal: R15 places it (the same lane on Mrow's
// controller's side when free, else the first free zone; with none, it stays), it keeps its damage
// and buffs (R78), it has entered its new controller's side this turn (R171), and a killer its
// controller already controls — a unit of Mrow's own side — is left where it is. `ctx.controller`
// is the side Mrow was on as it died, so a stolen Mrow takes the killer for whoever controlled it.

import type { Effect, Hook, Script } from "@jackioh/engine";
import { killerOf } from "@jackioh/engine";
import { steal } from "@jackioh/engine/effects";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-086");

/** "Death: Take control of the Unit that destroyed this" — identical on both faces. */
const death: Hook = (ctx): Effect[] => {
  const killer = killerOf(ctx.state, ctx.self);
  return killer === null ? [] : [steal({ instanceId: killer.id })];
};

export const base: Script = { death };

export const radiant: Script = { death };
