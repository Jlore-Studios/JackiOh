// C+ #32.3 Blade Storm (SPEC §8.7 row 32.3; §4.5, R59, R283, R386, R652). (1) Spell token, printed Epic.
//   Base:    "Cast Whirlwind until a Unit dies." (the round cap stays a declared number, not shown)
//   Radiant: "Deal 1 damage to all enemy Units. Repeat until a Unit dies, up to {rounds|time|times}."
//
// Each round is one effect list followed by its own state check — one of the two effect lists R59 lets
// check inside themselves, beside R283's — so the Units a round killed die, and their Death hooks
// resolve, before the next round. The storm stops after a round in which any Unit died (a Reborn death counts), after
// its round cap, or when no Unit is left (`castRoundsUntilDeath`, `damageRoundsUntilDeath`). The cap is the declared
// number `rounds` (R386), read through `param`: `BLADE_STORM_ROUNDS` (30) on both faces, which
// only a Degrade or an Upgrade of this card moves. The base face's every round casts Whirlwind
// (C+ #21, Pierce dealing 1 to all Units) as a real Spell cast (R70), so each hit goes through §4.4 —
// Divine Shield, Armor, Spell Damage and Immune to Spells apply round by round — and "whenever you
// cast a Spell" answers every round. The Radiant face hits enemy Units only; a death on either side
// still stops it. Its `refs` name Whirlwind (R279).

import { param, type Script } from "@jackioh/engine";
import { castRoundsUntilDeath, damageRoundsUntilDeath } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-032-3");

/** Whirlwind (C+ #21), which the base face casts round after round. */
const WHIRLWIND = "classicplus-021";

/** The Radiant face's printed hit of each round: "Deal 1 damage". Not a declared number (§8.7 tunes only `rounds`). */
const HIT = 1;

export const base: Script = {
  cry: (ctx) => [castRoundsUntilDeath({ def: WHIRLWIND, rounds: param(ctx, "rounds") })],
};

export const radiant: Script = {
  cry: (ctx) => [damageRoundsUntilDeath({ amount: HIT, rounds: param(ctx, "rounds"), side: "enemy" })],
};
