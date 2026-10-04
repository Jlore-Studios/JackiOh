// C #33 Joro (SPEC §8.6 row 33, §6.2 Replacement, §6.3 Redirect; R64, R97, R121,
// R177, R275, R347, R394, R450, R651). Unit, cost 0, Legendary, 1/1 → 1/1.
//   Base:    "While this is in your hand: When your opponent targets one of your Units with a Spell,
//            summon this and make it the new target."
//   Radiant: "Indestructible\nWhile this is in your hand: …" (the same).
//   Engine:  "A hand trigger (§6.2 hand and deck triggers) on the replacement point "a friendly unit is
//            targeted" (§6.2 Replacement): an enemy Spell played or cast whose declared target, or a
//            Spell's prompt answer, picks one of your units (R651). Joro is summoned (the leftmost open
//            zone, R64; with none, nothing happens), no Cry, summoning sick, and the pick is redirected
//            to it (Redirect, §6.3). "Targeted" means chosen by a Spell: a Spell's declared target or a
//            Spell's prompted pick; random picks, "all" effects, attacks and forced attacks (R121)
//            summon nothing. One Joro answers one targeting; a Spell that names several of your units
//            redirects the first. Joro answers from the hand only, a Yu-Gi-Oh hand trap (R394). Its
//            Radiant face keeps 1/1 and adds Indestructible, an endless decoy, and is named as the
//            exception to R275's stat half. Tunes: none."
//
// THE WHOLE CARD IS ONE REPLACEMENT, declared as data (B5 E5, `Script.replacements`): at "targeted",
// standing in its controller's hand (`where: "hand"`), answering only a Spell's targeting
// (`by: "spell"`, R651), it interposes — the engine summons it into its controller's leftmost empty,
// unlocked, unreserved unit zone (R64), with no Cry and summoning sick, and moves the pick to it
// (`redirected`). The engine asks only for the opponent's Spell choices of one of its controller's
// units on the field: a Spell's declared target (§10.5 step 1) or a Spell's prompt answer; never an
// attack, a Unit's Cry pick, an activation, a Trap's pick, a random pick, an "all" effect, a forced
// attack (R121) or its own controller's pick. With no open zone it does nothing and stays in hand.
// A Joro in a deck, a graveyard or on the field is not in the hand and answers nothing.
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
  replacements: [{ id: "joro", on: "targeted", where: "hand", by: "spell", instead: { interpose: true } }],
};

// The same script: the Radiant face differs only in what the engine reads off the catalog (its
// Indestructible).
export const radiant: Script = base;
