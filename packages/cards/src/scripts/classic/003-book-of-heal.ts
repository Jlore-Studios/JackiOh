// C #3 Book of Heal (SPEC §8.6 row 3). (1) Spell, Book, Epic.
//   Base:    "Heal a target {heal}." — heal 9
//   Radiant: "Heal a target {heal}." — heal 18
//   Engine:  "§6.3 Heal on any unit or hero, as #47 Fig of Life reads "Heal" (R19). Tunes: heal 9 ↑
//            (step 2)."
//
// R19 and §8's Conventions: "a target" is any unit or hero on either side, so the declaration names
// both kinds with `side: "any"` (a bare `target` is units only, R90). The pick is declared, so it
// travels in the play action (R81) and resolution never pauses.
//
// What "heal" does is §6.3's row, not this card's: a unit loses up to that much damage and never
// rises past its max health, a hero gains it with no cap (§3). `effects/heal.ts` is that split.
//
// The amount is the declared number `heal` (R386), read through `param`: 9 on the base face, 18 on
// the Radiant one, moved by a Degrade or an Upgrade two at a time (its declared step). Both faces run
// this one script, since the faces differ only in that number.

import { param, type Script } from "@jackioh/engine";
import { heal } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-003");

/** R19: any unit or hero, either side. */
const targets: TargetDecl[] = [
  // R656: a heal helps, so a random cast that targets enemies aims this at friends.
  { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] }, aim: "help" },
];

export const base: Script = {
  targets,
  cry: (ctx) => [heal({ target: { of: "chosen" }, amount: param(ctx, "heal") })],
};

// The same script: the Radiant face's 18 is its declared `heal`, which `param` reads off the running face.
export const radiant: Script = base;
