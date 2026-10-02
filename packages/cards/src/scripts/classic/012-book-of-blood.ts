// C #12 Book of Blood (SPEC §8.6 row 12). (1) Spell, Book, Common.
//   Base:    "Lifesteal\nDeal {damage} damage to a Unit." — damage 5
//   Radiant: "Lifesteal\nDeal {damage} damage to a Unit." — damage 10
//   Engine:  "A Unit target, either side; the effect states its own Lifesteal (R85), so your hero
//            heals the amount dealt. Tunes: damage 5 ↑."
//
// "a Unit" narrows §8's "target" to units, on either side, so no hero is offered (R90).
//
// R85: the damage carries its own Lifesteal (`lifesteal: true`), so §4.4 step 8 heals the caster's
// hero the amount actually dealt — after Armor, the anti-oneshot cap and R63's zero rule — and the
// heal is one heal however the Lifesteal is read: the face's printed keyword and the flag are one
// "has Lifesteal" test in `damage.ts`, never two heals.
//
// The amount is the declared number `damage` (R386), 5 or 10, read through `param`; both faces run
// this one script.

import { param, type Script } from "@jackioh/engine";
import { damage } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-012");

/** "a Unit": units only, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }];

export const base: Script = {
  targets,
  cry: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage"), lifesteal: true })],
};

// The same script: the Radiant face's 10 is its declared `damage`, which `param` reads off the running face.
export const radiant: Script = base;
