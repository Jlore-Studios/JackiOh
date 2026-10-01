// C+ #76.1 Brother Ping (SPEC §8.7 row 76.1). (2) Unit, CN, Human, Token (printed Rare), 4/4 → 8/8.
//   Base:    "Pierce / Activate: Deal {damage} damage."
//   Radiant: "Pierce / Activate 2: Deal {damage} damage."
//   Engine:  "Activate (§6.2, R384): its controller, in their own main phase with no prompt open, while it
//            is on the field; once per turn (Radiant twice), counted in `memory.activations` and reset
//            when it leaves the field (R78); summoning sickness and exertion don't apply, and it is not a
//            play. The target, any unit or hero, is declared with the activation (`activate { instanceId,
//            targets }`, §10.2). The hit's source is Brother Ping, so its Pierce applies (R346).
//            Tunes: Activate 1 ↑ (its X); damage 1 ↑."
//
// One ability on each face, its uses the printed Activate N (1, Radiant 2): the engine's activate
// subsystem counts them per turn on the instance, refuses a use past them (and lists none), and reads the
// count through B3.4's X change, so an Upgrade makes it Activate 2 and a Degrade never takes it below 1.
// The hit is one §4.4 instance whose source is this Unit, so its Pierce skips Armor (R346); it is not a
// Spell's hit, so Spell Damage never raises it.

import { param, type ActivationDecl, type Script } from "@jackioh/engine";
import { damage } from "@jackioh/engine/effects";
import type { TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-076-1");

/** §8.7: "Deal 1 damage" with no target named is targeted: any unit or hero, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

/** §8.7: "Activate" is once a turn, the Radiant face's "Activate 2" twice. */
const BASE_USES = 1;
const RADIANT_USES = 2;

function ping(uses: number): ActivationDecl {
  return {
    id: "ping",
    label: "Deal damage",
    uses,
    targets,
    run: (ctx) => [damage({ to: { of: "chosen" }, amount: param(ctx, "damage") })],
  };
}

export const base: Script = { activations: [ping(BASE_USES)] };

export const radiant: Script = { activations: [ping(RADIANT_USES)] };
