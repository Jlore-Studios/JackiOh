// C #83 Flame Lance (SPEC §8.6 row 83). (3) Spell, Common.
//   Base:    "Trample\nDeal {damage} damage to a Unit." — damage 11
//   Radiant: "Trample\nDeal {damage} damage to a Unit." — damage 22
//   Engine:  "Trample on a Spell (§4.4): the excess over the target Unit's health hits that Unit's
//            controller's hero as a new instance, as R346 put Pierce on a Spell. Tunes: damage 11 ↑
//            (step 2)."
//
// "a Unit": a declared target (R81), a Unit on either side, never a hero (R90). One §4.4 hit carrying
// Trample (B5 E6): the Spell prints it, and the effect states it too (`trample`), so the hit tramples
// however it is read. Step 2's Armor lowers the hit first; then the amount beyond the target's health
// before the hit goes to its controller's hero as a new instance (§4.4 step 9, R63). A Divine Shield
// (step 1) or an Indestructible target (step 4) stops the whole hit, and nothing tramples. The amount
// is the declared `damage` (R386), 11 or 22, read through `param`.
//
// Its proof: `test/classic/083-flame-lance.test.ts`.

import { param, type Script } from "@jackioh/engine";
import { damage } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-083");

/** "a Unit": units only, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }];

export const base: Script = {
  targets,
  cry: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage"), trample: true })],
};

// The same script: the Radiant face's 22 is its declared `damage`, which `param` reads off the face.
export const radiant: Script = base;
