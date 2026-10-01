// C+ #32.3 Blade Storm (SPEC §8.7 row 32.3; §4.5, R59, R283, R386). (1) Spell token, printed Epic.
//   Base:    "Deal 1 damage to all Units. Repeat until a Unit dies, up to {rounds|time|times}." — 30
//   Radiant: "Deal 1 damage to all enemy Units. Repeat until a Unit dies, up to {rounds|time|times}."
//
// Each round is one effect list followed by its own state check — one of the two effect lists R59 lets
// check inside themselves, beside R283's — so the Units a round killed die, and their Death hooks
// resolve, before the next round. The storm stops after a round in which any Unit died (a Reborn death counts), after
// its round cap, or when no Unit it hits is left (`damageRoundsUntilDeath`). The cap is the declared
// number `rounds` (R386), read through `param`: printed `BLADE_STORM_ROUNDS` (30) on both faces, which
// only a Degrade or an Upgrade of this card moves. Each hit goes through §4.4, so Divine Shield, Armor,
// Spell Damage and Immune to Spells apply round by round. The Radiant face hits enemy Units only; a
// death on either side still stops it.

import { param, type Script } from "@jackioh/engine";
import { damageRoundsUntilDeath } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-032-3");

/** The printed hit of each round: "Deal 1 damage". Not a declared number (§8.7 tunes only `rounds`). */
const HIT = 1;

function bladeStorm(side: "any" | "enemy"): Script {
  return {
    cry: (ctx) => [damageRoundsUntilDeath({ amount: HIT, rounds: param(ctx, "rounds"), side })],
  };
}

export const base: Script = bladeStorm("any");

export const radiant: Script = bladeStorm("enemy");
