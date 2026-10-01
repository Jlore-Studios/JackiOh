// C #33 Joro (SPEC §8.6 row 33, §4.2 step 2, §6.2 Replacement, §6.3 Redirect; R53, R64, R97, R121,
// R177, R275, R347, R394). Unit, cost 0, Legendary, 1/1 → 1/1.
//   Base:    "While this is in your hand: When your opponent targets one of your Units, summon this and
//            make it the new target."
//   Radiant: "Indestructible\nWhile this is in your hand: …" (the same).
//   Engine:  "A hand trigger (§6.2 hand and deck triggers) on the replacement point "a friendly unit is
//            targeted" (§6.2 Replacement): an enemy attack declared at one of your units (§4.2 step 2,
//            before the trap window) or an enemy play, cast, activation or prompt answer that picks one
//            of your units. Joro is summoned (the leftmost open zone, R64; with none, nothing happens),
//            no Cry, summoning sick, and the attack or pick is redirected to it (Redirect, §6.3).
//            "Targeted" means chosen: an attack's target or a declared or prompted pick; random picks
//            and "all" effects target nothing. One Joro answers one targeting; a play that names
//            several of your units redirects the first. Joro answers from the hand only, a Yu-Gi-Oh
//            hand trap (R394). Its Radiant face keeps 1/1 and adds Indestructible, an endless decoy,
//            and is named as the exception to R275's stat half. Tunes: none."
//
// THE WHOLE CARD IS ONE REPLACEMENT, declared as data (B5 E5, `Script.replacements`): at "targeted",
// standing in its controller's hand (`where: "hand"`), it interposes — the engine summons it into its
// controller's leftmost empty, unlocked, unreserved unit zone (R64), with no Cry and summoning sick,
// and moves the attack or the pick to it (`redirected`). The engine asks only for the opponent's
// choices of one of its controller's units on the field: an attack's target (§4.2 step 2, before the
// trap window) and a play's, cast's, activation's or prompt's pick; never a random pick, an "all"
// effect, a forced attack (R121) or its own controller's pick. With no open zone it does nothing and
// stays in hand. A Joro in a deck, a graveyard or on the field is not in the hand and answers nothing.
// The first Joro in the hand answers; one answers one targeting.
//
// Nothing about it shows while it waits (R97, R177): a hand card is its owner's to read, and a
// replacement is decided in the engine, not offered as a choice, so neither the opponent's view nor
// their `legalActions` changes with a Joro in the hand.
//
// The Radiant face's Indestructible is the catalog's (R275's named exception keeps it 1/1), so the
// script is the same; it survives the hit it draws, and R347 keeps Taunt off it.

import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-033");

export const base: Script = {
  replacements: [{ id: "joro", on: "targeted", where: "hand", instead: { interpose: true } }],
};

// The same script: the Radiant face differs only in what the engine reads off the catalog (its
// Indestructible).
export const radiant: Script = base;
