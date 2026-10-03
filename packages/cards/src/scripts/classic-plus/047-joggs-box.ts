// C+ #47 Jogg's Box (SPEC §8.7 row 47). (4) Spell, Legendary.
//   Base:    "Cast {casts|random Spell|random Spells}." — casts 10
//   Radiant: "Echo 1. Cast {casts|random Spell|random Spells}."
//   Engine:  "Random casts (Cast, §6.3), one after another: each a random non-token card of the Spell
//            type (not Field Spell or Trap) of every set (R380) but Jogg's Box (R387), repeats allowed
//            (R60), cast from no zone. Every choice of a random cast is random, targets, modes and
//            Discover picks alike, as "Targets chosen randomly" (§6.2), and its X is your current mana,
//            at least 1; so nothing pauses for a prompt. Each cast is free and counts as a play (R70),
//            and each cast Spell goes to your graveyard when it resolves (R87). Echo 1 runs the ten
//            again (Hearthstone's Yogg-Saron). Tunes: casts 10 ↑ (step 2)."
//
// `castRandom` (B5 E12, R452) is the whole card: each cast draws its Spell as it begins, never this
// card's definition, and makes every choice from the match rng; a Call to Chaos it casts is the first
// link of its chain (R28, R593). The Radiant Echo is §6.1's printed `Echo X` (R30), a repeat that
// runs the Cry again with fresh picks.

import { param, type Script } from "@jackioh/engine";
import { castRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-047");

export const base: Script = {
  cry: (ctx) => [castRandom({ query: { type: "Spell" }, count: param(ctx, "casts") })],
};

export const radiant: Script = { ...base, staticFlags: { echo: 1 } };
