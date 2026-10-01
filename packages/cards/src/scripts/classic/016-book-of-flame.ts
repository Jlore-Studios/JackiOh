// C #16 Book of Flame (SPEC §8.6 row 16). (1) Spell, Book, Common.
//   Base:    "Deal {damage} damage." — damage 4
//   Radiant: "Deal {damage} damage." — damage 8
//   Engine:  "One targeted hit ("deal N damage" with no target named is targeted, as #68 Twisted
//            Sorcerer's is). This is the Book of Flame that C #23 Devil's Pact and C #29 Book of
//            Vital Kill name; C #55 Book of Wildfire is a different card with its own name (R381).
//            Tunes: damage 4 ↑."
//
// §8's Conventions: "target" is any unit or hero on either side, and a bare `target` declaration is
// units only (R90), so the heroes are named. The pick travels in the play action (R81). The hit is
// one §4.4 instance, so Armor, Divine Shield, the anti-oneshot cap and Indestructible all apply.
//
// The amount is the declared number `damage` (R386), read through `param`: 4 on the base face, 8 on
// the Radiant one. Both faces run this one script.

import { param, type Script } from "@jackioh/engine";
import { damage } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-016");

/** §8 Conventions: any unit or hero, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

export const base: Script = {
  targets,
  cry: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })],
};

// The same script: the Radiant face's 8 is its declared `damage`, which `param` reads off the running face.
export const radiant: Script = base;
